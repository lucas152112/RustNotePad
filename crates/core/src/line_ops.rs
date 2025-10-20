use std::collections::HashSet;

use crate::editor::{EditOperation, EditorBuffer, EditorError, Selection};

/// 指定要進行大小寫轉換的模式。 / Enumerates supported case conversion transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseTransform {
    Upper,
    Lower,
    Toggle,
    Title,
}

/// 指定行排序的模式。 / Sorting strategy for line operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
    CaseInsensitiveAscending,
}

/// 調整行前縮排。 / Indents all targeted lines with the provided prefix.
pub fn indent_lines(buffer: &mut EditorBuffer, indent: &str) -> Result<(), EditorError> {
    if indent.is_empty() {
        return Ok(());
    }

    let text = buffer.contents().to_owned();
    let (range_start, range_end, line_indices) = target_line_span(buffer, &text);

    if line_indices.is_empty() {
        return Ok(());
    }

    let mut ops = Vec::with_capacity(line_indices.len());
    for &(line_start, _, _) in &line_indices {
        ops.push(EditOperation {
            start: line_start,
            end: line_start,
            text: indent.to_string(),
        });
    }

    let total_inserted = indent.len() * line_indices.len();
    buffer.apply_edit_plan(ops)?;
    clamp_selection(buffer, range_start, range_end + total_inserted);
    Ok(())
}

/// 移除行前縮排（若存在）。 / Removes the provided indent prefix from targeted lines.
pub fn outdent_lines(buffer: &mut EditorBuffer, indent: &str) -> Result<(), EditorError> {
    if indent.is_empty() {
        return Ok(());
    }

    let text = buffer.contents().to_owned();
    let (range_start, range_end, line_indices) = target_line_span(buffer, &text);
    if line_indices.is_empty() {
        return Ok(());
    }

    let mut ops = Vec::new();
    for &(line_start, content_len, _) in &line_indices {
        let line_end = line_start + content_len;
        let line_slice = &text[line_start..line_end];
        if line_slice.starts_with(indent) {
            ops.push(EditOperation {
                start: line_start,
                end: line_start + indent.len(),
                text: String::new(),
            });
        } else {
            let trimmed = line_slice
                .chars()
                .take_while(|ch| ch.is_whitespace())
                .collect::<String>();
            if !trimmed.is_empty() {
                let remove_len = trimmed.len().min(indent.len());
                ops.push(EditOperation {
                    start: line_start,
                    end: line_start + remove_len,
                    text: String::new(),
                });
            }
        }
    }

    let removed_total: usize = ops.iter().map(|op| op.end.saturating_sub(op.start)).sum();
    buffer.apply_edit_plan(ops)?;
    let new_end = range_end.saturating_sub(removed_total);
    clamp_selection(buffer, range_start, new_end);
    Ok(())
}

/// 修剪所有行末尾的空白。 / Removes trailing whitespace across targeted lines.
pub fn trim_trailing_whitespace(buffer: &mut EditorBuffer) -> Result<usize, EditorError> {
    let text = buffer.contents().to_owned();
    let (_, _, line_indices) = target_line_span(buffer, &text);
    if line_indices.is_empty() {
        return Ok(0);
    }

    let mut ops = Vec::new();
    for &(line_start, content_len, _) in &line_indices {
        let line_end = line_start + content_len;
        let line_slice = &text[line_start..line_end];
        let mut trim_end = line_slice.len();
        while trim_end > 0 {
            let ch = line_slice[..trim_end].chars().last().unwrap();
            if ch == ' ' || ch == '\t' {
                trim_end -= ch.len_utf8();
            } else {
                break;
            }
        }
        if trim_end < line_slice.len() {
            ops.push(EditOperation {
                start: line_start + trim_end,
                end: line_end,
                text: String::new(),
            });
        }
    }

    let trimmed = ops.len();
    buffer.apply_edit_plan(ops)?;
    Ok(trimmed)
}

/// 對選取範圍的文字進行大小寫轉換。 / Applies a case transformation to the active selection or whole buffer.
pub fn convert_case(
    buffer: &mut EditorBuffer,
    transform: CaseTransform,
) -> Result<(), EditorError> {
    let text = buffer.contents();
    let (start, end) = selection_span(buffer, text);
    if start == end {
        return Ok(());
    }

    let target = &text[start..end];
    let converted = match transform {
        CaseTransform::Upper => target.to_uppercase(),
        CaseTransform::Lower => target.to_lowercase(),
        CaseTransform::Toggle => toggle_case(target),
        CaseTransform::Title => title_case(target),
    };

    buffer.apply_edit_plan(vec![EditOperation {
        start,
        end,
        text: converted,
    }])?;
    Ok(())
}

/// 針對選取或整個範圍的行進行排序。 / Sorts lines within the active selection (or whole buffer).
pub fn sort_lines(buffer: &mut EditorBuffer, order: SortOrder) -> Result<(), EditorError> {
    let text = buffer.contents().to_owned();
    let (range_start, range_end, line_indices) = target_line_span(buffer, &text);
    if line_indices.len() <= 1 {
        return Ok(());
    }

    let mut lines: Vec<&str> = line_indices
        .iter()
        .map(|&(start, len, _)| &text[start..start + len])
        .collect();

    lines.sort_by(|a, b| match order {
        SortOrder::Ascending => a.cmp(b),
        SortOrder::Descending => b.cmp(a),
        SortOrder::CaseInsensitiveAscending => a.to_lowercase().cmp(&b.to_lowercase()),
    });

    let joined = join_lines(&lines, &line_indices, &text);
    buffer.apply_edit_plan(vec![EditOperation {
        start: range_start,
        end: range_end,
        text: joined,
    }])?;
    Ok(())
}

/// 移除重複行，僅保留首次出現。 / Deduplicates lines while preserving first occurrence order.
pub fn dedup_lines(buffer: &mut EditorBuffer, case_sensitive: bool) -> Result<(), EditorError> {
    let text = buffer.contents().to_owned();
    let (range_start, range_end, line_indices) = target_line_span(buffer, &text);
    if line_indices.len() <= 1 {
        return Ok(());
    }

    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for &(start, len, _) in &line_indices {
        let line = &text[start..start + len];
        let key = if case_sensitive {
            line.to_owned()
        } else {
            line.to_lowercase()
        };
        if seen.insert(key) {
            ordered.push(line);
        }
    }

    let joined = join_lines(&ordered, &line_indices, &text);
    buffer.apply_edit_plan(vec![EditOperation {
        start: range_start,
        end: range_end,
        text: joined,
    }])?;
    Ok(())
}

fn selection_span(buffer: &EditorBuffer, text: &str) -> (usize, usize) {
    let mut start = usize::MAX;
    let mut end = 0usize;
    for caret in buffer.carets() {
        if let Some(selection) = caret.selection() {
            start = start.min(selection.start());
            end = end.max(selection.end());
        }
    }
    if start == usize::MAX {
        (0, text.len())
    } else {
        (start, end)
    }
}

fn toggle_case(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_lowercase() {
                ch.to_uppercase().next().unwrap_or(ch)
            } else if ch.is_uppercase() {
                ch.to_lowercase().next().unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect()
}

fn title_case(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut new_word = true;
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            if new_word {
                result.extend(ch.to_uppercase());
                new_word = false;
            } else {
                result.extend(ch.to_lowercase());
            }
        } else {
            new_word = ch.is_whitespace();
            result.push(ch);
        }
    }
    result
}

fn join_lines(lines: &[&str], indices: &[(usize, usize, usize)], original: &str) -> String {
    let mut result = String::new();
    for (idx, line) in lines.iter().enumerate() {
        result.push_str(line);
        if let Some(&(start, content_len, newline_len)) = indices.get(idx) {
            if newline_len > 0 {
                let newline_slice =
                    &original[start + content_len..start + content_len + newline_len];
                result.push_str(newline_slice);
            }
        }
    }
    result
}

fn target_line_span(
    buffer: &EditorBuffer,
    text: &str,
) -> (usize, usize, Vec<(usize, usize, usize)>) {
    let line_meta = compute_line_boundaries(text);
    if line_meta.is_empty() {
        return (0, 0, Vec::new());
    }

    let (sel_start, sel_end) = selection_span(buffer, text);
    let start_line = find_line_for_index(sel_start, &line_meta);
    let end_line = find_line_for_index(sel_end.saturating_sub(1), &line_meta);

    let range_start = line_meta[start_line].0;
    let range_end = line_meta[end_line].0 + line_meta[end_line].1 + line_meta[end_line].2;

    let slice = line_meta[start_line..=end_line].to_vec();
    (range_start, range_end, slice)
}

fn clamp_selection(buffer: &mut EditorBuffer, start: usize, end: usize) {
    let len = buffer.contents().len();
    let clamped_start = start.min(len);
    let clamped_end = end.min(len);
    if buffer.carets().is_empty() {
        buffer.clear_carets();
    }
    if let Some(caret) = buffer.carets_mut().first_mut() {
        *caret = crate::editor::Caret::with_selection(
            clamped_end,
            Selection::new(clamped_start, clamped_end),
        );
    }
}

fn compute_line_boundaries(text: &str) -> Vec<(usize, usize, usize)> {
    if text.is_empty() {
        return vec![(0, 0, 0)];
    }

    let mut results = Vec::new();
    let mut cursor = 0usize;
    for segment in text.split_inclusive('\n') {
        let content_len = segment.trim_end_matches('\n').len();
        let newline_len = segment.len() - content_len;
        results.push((cursor, content_len, newline_len));
        cursor += segment.len();
    }

    if text.ends_with('\n') {
        results.push((text.len(), 0, 0));
    }

    results
}

fn find_line_for_index(index: usize, lines: &[(usize, usize, usize)]) -> usize {
    if lines.is_empty() {
        return 0;
    }
    for (i, &(start, content_len, newline_len)) in lines.iter().enumerate() {
        let line_end = start + content_len + newline_len;
        if index < line_end || i == lines.len() - 1 {
            return i;
        }
    }
    lines.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Caret, EditorBuffer, Selection};

    fn buffer_with_selection(text: &str, start: usize, end: usize) -> EditorBuffer {
        let caret = Caret::with_selection(end, Selection::new(start, end));
        EditorBuffer::with_carets(text, vec![caret]).unwrap()
    }

    #[test]
    fn trim_whitespace_removes_trailing_spaces() {
        let mut buffer = EditorBuffer::new("foo \nbar\t\nbaz");
        let trimmed = trim_trailing_whitespace(&mut buffer).unwrap();
        assert_eq!(buffer.contents(), "foo\nbar\nbaz");
        assert_eq!(trimmed, 2);
    }

    #[test]
    fn indent_and_outdent_roundtrip() {
        let text = "a\nb\nc";
        let mut buffer = buffer_with_selection(text, 0, text.len());
        indent_lines(&mut buffer, "    ").unwrap();
        assert_eq!(buffer.contents(), "    a\n    b\n    c");
        outdent_lines(&mut buffer, "    ").unwrap();
        assert_eq!(buffer.contents(), "a\nb\nc");
    }

    #[test]
    fn convert_case_toggle_and_title() {
        let mut buffer = buffer_with_selection("hello world", 0, 11);
        convert_case(&mut buffer, CaseTransform::Toggle).unwrap();
        assert_eq!(buffer.contents(), "HELLO WORLD");
        convert_case(&mut buffer, CaseTransform::Title).unwrap();
        assert_eq!(buffer.contents(), "Hello World");
    }

    #[test]
    fn sort_and_dedup_lines() {
        let mut buffer = buffer_with_selection("c\nb\na\nb\n", 0, 8);
        sort_lines(&mut buffer, SortOrder::Ascending).unwrap();
        assert_eq!(buffer.contents(), "a\nb\nb\nc\n");
        dedup_lines(&mut buffer, true).unwrap();
        assert_eq!(buffer.contents(), "a\nb\nc\n");
    }
}
