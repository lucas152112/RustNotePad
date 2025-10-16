use crate::editor::{Caret, EditOperation, EditorBuffer, EditorError};

/// 描述矩形選取範圍。 / Describes a rectangular selection across multiple lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnSelection {
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
}

impl ColumnSelection {
    /// 建立新的矩形選取，會自動正規化起訖索引。 / Creates a normalised selection.
    pub fn new(line_a: usize, line_b: usize, col_a: usize, col_b: usize) -> Self {
        let (start_line, end_line) = if line_a <= line_b {
            (line_a, line_b)
        } else {
            (line_b, line_a)
        };
        let (start_column, end_column) = if col_a <= col_b {
            (col_a, col_b)
        } else {
            (col_b, col_a)
        };
        Self {
            start_line,
            end_line,
            start_column,
            end_column,
        }
    }

    fn width(&self) -> usize {
        self.end_column.saturating_sub(self.start_column)
    }
}

/// 以欄位模式取代文字。 / Performs a column-mode replacement, padding lines as needed.
pub fn replace_columnar(
    buffer: &mut EditorBuffer,
    selection: ColumnSelection,
    payload: &[String],
) -> Result<(), EditorError> {
    let normalized = ColumnSelection::new(
        selection.start_line,
        selection.end_line,
        selection.start_column,
        selection.end_column,
    );
    let text = buffer.contents().to_owned();
    let mut line_bounds = compute_line_bounds(&text);
    if line_bounds.is_empty() {
        line_bounds.push((0, 0, 0));
    }

    let max_index = line_bounds.len().saturating_sub(1);
    let start_line = normalized.start_line.min(max_index);
    let end_line = normalized.end_line.min(max_index);
    let mut ops = Vec::new();

    for (idx, line_idx) in (start_line..=end_line).enumerate() {
        let (line_start, content_len, _newline) = line_bounds[line_idx];
        let mut line = text[line_start..line_start + content_len].to_string();
        let insertion = resolve_payload(payload, idx);

        let max_col = normalized.start_column.max(normalized.end_column);
        ensure_column_capacity(&mut line, max_col);

        let start_col = normalized.start_column;
        let end_col = normalized.start_column + normalized.width();
        let start_byte = column_to_byte(&line, start_col);
        let end_byte = column_to_byte(&line, end_col);

        line.replace_range(start_byte..end_byte, insertion);

        ops.push(EditOperation {
            start: line_start,
            end: line_start + content_len,
            text: line,
        });
    }

    if ops.is_empty() {
        return Ok(());
    }

    buffer.apply_edit_plan(ops)?;
    update_carets(buffer, start_line, end_line, normalized.start_column, payload)?;
    Ok(())
}

fn resolve_payload<'a>(payload: &'a [String], idx: usize) -> &'a str {
    if payload.is_empty() {
        ""
    } else if payload.len() == 1 {
        payload[0].as_str()
    } else {
        payload[idx.min(payload.len() - 1)].as_str()
    }
}

fn ensure_column_capacity(line: &mut String, column: usize) {
    let current = line.chars().count();
    if current < column {
        line.extend(std::iter::repeat(' ').take(column - current));
    }
}

fn column_to_byte(line: &str, column: usize) -> usize {
    let mut current = 0usize;
    for (byte_idx, ch) in line.char_indices() {
        if current == column {
            return byte_idx;
        }
        current += 1;
        if current == column {
            return byte_idx + ch.len_utf8();
        }
    }
    line.len()
}

fn compute_line_bounds(text: &str) -> Vec<(usize, usize, usize)> {
    let mut results = Vec::new();
    let mut start = 0usize;
    let mut iter = text.char_indices().peekable();
    while let Some((idx, ch)) = iter.next() {
        if ch == '\n' {
            results.push((start, idx - start, 1));
            start = idx + 1;
        } else if iter.peek().is_none() {
            results.push((start, idx + ch.len_utf8() - start, 0));
        }
    }
    if start < text.len() {
        results.push((start, text.len() - start, 0));
    } else if text.is_empty() {
        results.push((0, 0, 0));
    }
    results
}

fn update_carets(
    buffer: &mut EditorBuffer,
    start_line: usize,
    end_line: usize,
    start_column: usize,
    payload: &[String],
) -> Result<(), EditorError> {
    let final_text = buffer.contents().to_owned();
    let line_bounds = compute_line_bounds(&final_text);
    let mut carets = Vec::new();

    for (idx, line_idx) in (start_line..=end_line).enumerate() {
        if line_idx >= line_bounds.len() {
            continue;
        }
        let (line_start, content_len, _) = line_bounds[line_idx];
        let line_slice = &final_text[line_start..line_start + content_len];
        let payload_cols = if payload.is_empty() {
            0
        } else if payload.len() == 1 {
            payload[0].chars().count()
        } else {
            payload[idx.min(payload.len() - 1)].chars().count()
        };
        let caret_col = start_column + payload_cols;
        let caret_byte = column_to_byte(line_slice, caret_col);
        carets.push(Caret::new(line_start + caret_byte));
    }

    if !carets.is_empty() {
        buffer.set_carets(carets)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_columnar_inserts_text_with_padding() {
        let mut buffer = EditorBuffer::new("one\nlonger line\nshort");
        let payload = vec!["X".into()];
        replace_columnar(
            &mut buffer,
            ColumnSelection::new(0, 2, 2, 4),
            &payload,
        )
        .unwrap();
        assert_eq!(buffer.contents(), "onX\nloXer line\nshXt");
    }

    #[test]
    fn replace_columnar_per_line_payload() {
        let mut buffer = EditorBuffer::new("a\nb\nc");
        let payload = vec!["1".into(), "22".into(), "333".into()];
        replace_columnar(
            &mut buffer,
            ColumnSelection::new(0, 2, 0, 0),
            &payload,
        )
        .unwrap();
        assert_eq!(buffer.contents(), "1a\n22b\n333c");
        assert_eq!(buffer.carets().len(), 3);
    }
}
