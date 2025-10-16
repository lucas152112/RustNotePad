use std::cmp::Ordering;

use thiserror::Error;

/// 描述多重游標環境中的插入點。 / Represents a caret within the editor buffer (optional selection).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caret {
    position: usize,
    selection: Option<Selection>,
}

impl Caret {
    /// 建立指定位置的游標。 / Creates a caret at the given position.
    pub fn new(position: usize) -> Self {
        Self {
            position,
            selection: None,
        }
    }

    /// 建立帶有選取範圍的游標。 / Creates a caret with the provided selection range.
    pub fn with_selection(position: usize, selection: Selection) -> Self {
        Self {
            position,
            selection: Some(selection),
        }
    }

    /// 取得游標所在位置。 / Returns the caret position.
    pub fn position(&self) -> usize {
        self.position
    }

    /// 取得選取範圍（若有）。 / Returns the active selection if present.
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    fn edit_range(&self) -> (usize, usize) {
        if let Some(selection) = &self.selection {
            (selection.start, selection.end)
        } else {
            (self.position, self.position)
        }
    }

    fn set_position(&mut self, position: usize) {
        self.position = position;
        self.selection = None;
    }

    fn clamp(&mut self, len: usize) {
        if self.position > len {
            self.position = len;
        }
        if let Some(selection) = &mut self.selection {
            selection.clamp(len);
            if self.position < selection.start {
                self.position = selection.start;
            } else if self.position > selection.end {
                self.position = selection.end;
            }
            if selection.start == selection.end {
                self.selection = None;
            }
        }
    }
}

/// 定義一段已排序（start <= end）的文字範圍。 / Represents an ordered selection range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    start: usize,
    end: usize,
}

impl Selection {
    /// 建立新的選取範圍，會自動將 start/end 排序。 / Creates a selection with automatically ordered bounds.
    pub fn new(a: usize, b: usize) -> Self {
        if a <= b {
            Self { start: a, end: b }
        } else {
            Self { start: b, end: a }
        }
    }

    /// 範圍起點。 / Returns the start of the selection.
    pub fn start(&self) -> usize {
        self.start
    }

    /// 範圍終點。 / Returns the end of the selection.
    pub fn end(&self) -> usize {
        self.end
    }

    /// 選取長度。 / Returns the length of the selection.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    fn clamp(&mut self, len: usize) {
        if self.start > len {
            self.start = len;
        }
        if self.end > len {
            self.end = len;
        }
        if self.start > self.end {
            self.start = self.end;
        }
    }
}

/// 編輯器緩衝區錯誤。 / Error conditions exposed by the editing buffer.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EditorError {
    #[error("caret index {index} is out of bounds for buffer of length {len}")]
    CaretOutOfBounds { index: usize, len: usize },
    #[error("caret selections overlap and cannot be applied safely")]
    OverlappingCarets,
}

/// 具備多重游標與基本編輯操作的文字緩衝。 / Text buffer supporting multi-caret editing primitives.
#[derive(Debug, Clone)]
pub struct EditorBuffer {
    contents: String,
    carets: Vec<Caret>,
}

/// 描述一段待取代的編輯操作。 / Represents a replacement operation within the buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditOperation {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

impl EditorBuffer {
    /// 從給定文字建立緩衝區，預設建立單一游標在開頭。 / Creates a buffer with a single caret at the start.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            contents: text.into(),
            carets: vec![Caret::new(0)],
        }
    }

    /// 以指定的游標集合建立緩衝區。 / Creates a buffer with explicit caret positions.
    pub fn with_carets(text: impl Into<String>, carets: Vec<Caret>) -> Result<Self, EditorError> {
        let buffer = Self {
            contents: text.into(),
            carets,
        };
        buffer.validate_carets()?;
        Ok(buffer)
    }

    /// 取得目前的內容。 / Returns the current buffer contents.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// 取得所有游標。 / Returns all carets.
    pub fn carets(&self) -> &[Caret] {
        &self.carets
    }

    /// 取得可變游標集合。 / Returns a mutable slice of carets.
    pub fn carets_mut(&mut self) -> &mut [Caret] {
        self.carets.as_mut_slice()
    }

    /// 取代每個游標範圍為給定文字。 / Replaces each caret selection with the provided text.
    pub fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.apply_replacements(|_, caret| {
            let (start, end) = caret.edit_range();
            Ok(Replacement {
                start,
                end,
                text: text.to_string(),
                owner: None,
            })
        })
    }

    /// 模擬 Backspace：若有選取則刪除選取，否則刪除游標前一個字元。 / Simulates a backspace operation.
    pub fn delete_backward(&mut self) -> Result<(), EditorError> {
        self.apply_replacements(|text, caret| {
            let (start, end) = caret.edit_range();
            if start != end {
                return Ok(Replacement {
                    start,
                    end,
                    text: String::new(),
                    owner: None,
                });
            }
            if start == 0 {
                return Ok(Replacement {
                    start,
                    end,
                    text: String::new(),
                    owner: None,
                });
            }
            let prev = prev_grapheme_boundary(text, start).unwrap_or(0);
            Ok(Replacement {
                start: prev,
                end: start,
                text: String::new(),
                owner: None,
            })
        })
    }

    /// 模擬 Delete 行為：刪除選取或游標後一個字元。 / Simulates a forward-delete operation.
    pub fn delete_forward(&mut self) -> Result<(), EditorError> {
        self.apply_replacements(|text, caret| {
            let (start, end) = caret.edit_range();
            if start != end {
                return Ok(Replacement {
                    start,
                    end,
                    text: String::new(),
                    owner: None,
                });
            }
            if start >= text.len() {
                return Ok(Replacement {
                    start,
                    end,
                    text: String::new(),
                    owner: None,
                });
            }
            let next = next_grapheme_boundary(text, start).unwrap_or(text.len());
            Ok(Replacement {
                start,
                end: next,
                text: String::new(),
                owner: None,
            })
        })
    }

    /// 在每個游標位置插入換行。 / Inserts a newline at every caret position.
    pub fn insert_newline(&mut self) -> Result<(), EditorError> {
        self.insert_text("\n")
    }

    /// 以新的游標集合取代現有游標。 / Replaces the caret list with the provided set.
    pub fn set_carets(&mut self, carets: Vec<Caret>) -> Result<(), EditorError> {
        let candidate = carets;
        self.validate_custom_carets(&candidate)?;
        self.carets = candidate;
        Ok(())
    }

    /// 加入新的游標至集合。 / Appends an additional caret to the buffer.
    pub fn push_caret(&mut self, caret: Caret) -> Result<(), EditorError> {
        let mut candidate = self.carets.clone();
        candidate.push(caret);
        self.validate_custom_carets(&candidate)?;
        self.carets = candidate;
        Ok(())
    }

    /// 清除所有游標並重設為單一游標於開頭。 / Clears all carets and resets to a single caret at the start.
    pub fn clear_carets(&mut self) {
        self.carets.clear();
        self.carets.push(Caret::new(0));
    }

    /// 將選取範圍設定為整份文件。 / Selects the entire document.
    pub fn select_all(&mut self) {
        let len = self.contents.len();
        self.carets = vec![Caret::with_selection(len, Selection::new(0, len))];
    }

    /// 直接套用一組編輯操作。 / Applies an arbitrary list of edit operations.
    pub fn apply_edit_plan(&mut self, operations: Vec<EditOperation>) -> Result<(), EditorError> {
        if operations.is_empty() {
            return Ok(());
        }
        let replacements: Vec<Replacement> = operations
            .into_iter()
            .map(|op| Replacement {
                start: op.start,
                end: op.end,
                text: op.text,
                owner: None,
            })
            .collect();

        self.commit_replacements(replacements, None)?;
        self.clamp_carets();
        Ok(())
    }

    fn apply_replacements<F>(&mut self, mut plan: F) -> Result<(), EditorError>
    where
        F: FnMut(&str, &Caret) -> Result<Replacement, EditorError>,
    {
        self.validate_carets()?;

        let mut replacements = Vec::with_capacity(self.carets.len());
        for (idx, caret) in self.carets.iter().enumerate() {
            let mut replacement = plan(&self.contents, caret)?;
            replacement.owner = Some(idx);
            replacements.push(replacement);
        }

        let new_positions = self.commit_replacements(replacements, Some(self.carets.len()))?;
        for (idx, pos) in new_positions.into_iter().enumerate() {
            self.carets[idx].set_position(pos);
        }
        Ok(())
    }

    fn commit_replacements(
        &mut self,
        mut replacements: Vec<Replacement>,
        owners: Option<usize>,
    ) -> Result<Vec<usize>, EditorError> {
        // 確保取代區段不重疊。 / Ensure edits do not overlap.
        let mut order: Vec<usize> = (0..replacements.len()).collect();
        order.sort_by(|&a, &b| {
            let left = &replacements[a];
            let right = &replacements[b];
            match left.start.cmp(&right.start) {
                Ordering::Equal => left.end.cmp(&right.end),
                other => other,
            }
        });
        for window in order.windows(2) {
            if replacements[window[0]].end > replacements[window[1]].start {
                return Err(EditorError::OverlappingCarets);
            }
        }

        let mut offset: isize = 0;
        let mut new_positions = vec![];
        if let Some(count) = owners {
            new_positions.resize(count, 0);
        }

        for &index in &order {
            let replacement = &mut replacements[index];
            let adjusted_start = (replacement.start as isize + offset).max(0) as usize;
            let adjusted_end = (replacement.end as isize + offset).max(0) as usize;
            self.contents
                .replace_range(adjusted_start..adjusted_end, &replacement.text);

            let delta =
                replacement.text.len() as isize - (replacement.end - replacement.start) as isize;
            offset += delta;
            if let Some(owner_idx) = replacement.owner {
                new_positions[owner_idx] = adjusted_start + replacement.text.len();
            }
        }
        Ok(new_positions)
    }

    fn validate_carets(&self) -> Result<(), EditorError> {
        self.validate_custom_carets(&self.carets)
    }

    fn validate_custom_carets(&self, carets: &[Caret]) -> Result<(), EditorError> {
        let len = self.contents.len();
        let mut spans: Vec<(usize, usize)> = Vec::with_capacity(carets.len());
        for (idx, caret) in carets.iter().enumerate() {
            let (start, end) = caret.edit_range();
            let position = caret.position();
            if start > len || end > len || position > len {
                return Err(EditorError::CaretOutOfBounds { index: idx, len });
            }
            if position < start || position > end {
                return Err(EditorError::CaretOutOfBounds { index: idx, len });
            }
            spans.push((start, end));
        }
        spans.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for window in spans.windows(2) {
            if window[0].1 > window[1].0 {
                return Err(EditorError::OverlappingCarets);
            }
        }
        Ok(())
    }

    fn clamp_carets(&mut self) {
        let len = self.contents.len();
        for caret in &mut self.carets {
            caret.clamp(len);
        }
        let _ = self.validate_carets();
    }
}

struct Replacement {
    start: usize,
    end: usize,
    text: String,
    owner: Option<usize>,
}

fn prev_grapheme_boundary(text: &str, index: usize) -> Option<usize> {
    if index == 0 || index > text.len() {
        return None;
    }
    text[..index].char_indices().last().map(|(idx, _)| idx)
}

fn next_grapheme_boundary(text: &str, index: usize) -> Option<usize> {
    if index >= text.len() {
        return None;
    }
    for (idx, ch) in text[index..].char_indices() {
        if idx != 0 {
            return Some(index + idx);
        }
        if ch.len_utf8() > 0 {
            return Some(index + ch.len_utf8());
        }
    }
    Some(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_with_multiple_carets() {
        let carets = vec![
            Caret::with_selection(5, Selection::new(0, 5)),
            Caret::new("alpha\nbeta".len()),
        ];
        let mut buffer = EditorBuffer::with_carets("alpha\nbeta", carets.clone()).unwrap();
        buffer.insert_text("RNP").unwrap();
        assert_eq!(buffer.contents(), "RNP\nbetaRNP");
        assert_eq!(buffer.carets()[0].position(), 3);
        assert_eq!(buffer.carets()[1].position(), "RNP\nbetaRNP".len());

        // verify setting carets succeeds
        let mut buffer = EditorBuffer::new("abc");
        buffer
            .set_carets(vec![Caret::new(0), Caret::new(2)])
            .unwrap();
        assert_eq!(buffer.carets().len(), 2);
    }

    #[test]
    fn delete_backward_handles_utf8() {
        let carets = vec![Caret::new("你好".len())];
        let mut buffer = EditorBuffer::with_carets("你好", carets).unwrap();
        buffer.delete_backward().unwrap();
        assert_eq!(buffer.contents(), "你");
        assert_eq!(buffer.carets()[0].position(), "你".len());
    }

    #[test]
    fn overlapping_carets_error() {
        let carets = vec![
            Caret::with_selection(2, Selection::new(0, 4)),
            Caret::with_selection(4, Selection::new(3, 6)),
        ];
        let result = EditorBuffer::with_carets("abcdef", carets);
        assert!(matches!(result, Err(EditorError::OverlappingCarets)));
    }

    #[test]
    fn delete_forward_removes_next_grapheme() {
        let mut buffer = EditorBuffer::new("a你好c");
        buffer
            .set_carets(vec![Caret::new("a".len())])
            .expect("carets");
        buffer.delete_forward().unwrap();
        assert_eq!(buffer.contents(), "a好c");
        assert_eq!(buffer.carets()[0].position(), "a".len());
    }

    #[test]
    fn apply_edit_plan_replaces_segments() {
        let mut buffer = EditorBuffer::new("line1\nline2\nline3");
        buffer
            .apply_edit_plan(vec![
                EditOperation {
                    start: 0,
                    end: 5,
                    text: "LINE1".into(),
                },
                EditOperation {
                    start: 6,
                    end: 11,
                    text: "LINE2".into(),
                },
            ])
            .unwrap();
        assert_eq!(buffer.contents(), "LINE1\nLINE2\nline3");
    }
}
