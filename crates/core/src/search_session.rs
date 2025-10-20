use std::collections::BTreeSet;
use std::path::PathBuf;

use rustnotepad_search::{
    FileSearchResult, ReplaceAllOutcome, SearchDirection, SearchEngine, SearchError, SearchMatch,
    SearchOptions, SearchReport, SearchScope,
};

use crate::{BookmarkManager, Document};

/// Tracks the active search context for a document, including cached matches and bookmark marks.
/// （追蹤文件目前的搜尋狀態，包含快取結果與書籤標記。）
#[derive(Debug, Clone)]
pub struct SearchSession {
    options: SearchOptions,
    matches: Vec<SearchMatch>,
    current: Option<usize>,
    marked_lines: BTreeSet<usize>,
}

impl SearchSession {
    /// Creates a new session with the provided options. The caller must invoke [`refresh`] before searching.
    /// （以指定選項建立新的搜尋會話；在搜尋前必須呼叫 [`refresh`]。）
    pub fn new(options: SearchOptions) -> Result<Self, SearchError> {
        options.validate()?;
        Ok(Self {
            options,
            matches: Vec::new(),
            current: None,
            marked_lines: BTreeSet::new(),
        })
    }

    /// Returns a shared reference to the underlying options.
    /// （提供搜尋選項的唯讀參考。）
    pub fn options(&self) -> &SearchOptions {
        &self.options
    }

    /// Returns a mutable reference to the options, clearing cached state.
    /// （取得可變搜尋選項並清除快取狀態。）
    pub fn options_mut(&mut self) -> &mut SearchOptions {
        self.matches.clear();
        self.current = None;
        self.marked_lines.clear();
        &mut self.options
    }

    /// Recomputes matches against the given document contents using the current options.
    /// （依目前選項重新計算文件內的符合結果。）
    pub fn refresh(&mut self, document: &Document) -> Result<(), SearchError> {
        let engine = SearchEngine::new(document.contents());
        self.matches = engine.find_all(&self.options)?;
        self.current = None;
        self.marked_lines.clear();
        Ok(())
    }

    /// Returns all cached matches (refresh must be called beforehand).
    /// （回傳快取的所有符合結果；需先呼叫 `refresh`。）
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Indicates whether cached results are empty.
    /// （判斷目前是否沒有快取結果。）
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Selects the next match, respecting wrap-around rules.
    /// （依循循環規則選取下一筆符合結果。）
    pub fn find_next(&mut self) -> Option<&SearchMatch> {
        self.advance(SearchDirection::Forward)
    }

    /// Selects the previous match, respecting wrap-around rules.
    /// （依循循環規則選取上一筆符合結果。）
    pub fn find_previous(&mut self) -> Option<&SearchMatch> {
        self.advance(SearchDirection::Backward)
    }

    fn advance(&mut self, direction: SearchDirection) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        let wrap = self.options.wrap_around;
        let idx = match (self.current, direction) {
            (None, SearchDirection::Forward) => Some(0),
            (None, SearchDirection::Backward) => {
                if wrap {
                    Some(self.matches.len().saturating_sub(1))
                } else {
                    None
                }
            }
            (Some(i), SearchDirection::Forward) => {
                if i + 1 < self.matches.len() {
                    Some(i + 1)
                } else if wrap {
                    Some(0)
                } else {
                    None
                }
            }
            (Some(i), SearchDirection::Backward) => {
                if i > 0 {
                    Some(i - 1)
                } else if wrap {
                    Some(self.matches.len().saturating_sub(1))
                } else {
                    None
                }
            }
        };

        if let Some(next) = idx {
            self.current = Some(next);
            Some(&self.matches[next])
        } else {
            None
        }
    }

    /// Returns the currently selected match.
    /// （回傳目前選取的符合結果。）
    pub fn current(&self) -> Option<&SearchMatch> {
        self.current.and_then(|idx| self.matches.get(idx))
    }

    /// Replaces the currently selected match (if any) with the provided text.
    /// （以指定文字取代當前選取的結果（若存在）。）
    /// Returns the original match that was replaced.
    /// （回傳被取代的原始結果。）
    pub fn replace_current(
        &mut self,
        replacement: &str,
        document: &mut Document,
    ) -> Result<Option<SearchMatch>, SearchError> {
        let idx = match self.current {
            Some(idx) => idx,
            None => return Ok(None),
        };
        let target = match self.matches.get(idx) {
            Some(m) => m.clone(),
            None => return Ok(None),
        };

        let mut updated = document.contents().to_string();
        updated.replace_range(target.start..target.end, replacement);
        document.set_contents(updated);

        self.refresh(document)?;
        self.current = self
            .matches
            .iter()
            .enumerate()
            .find(|(_, m)| m.start >= target.start)
            .map(|(i, _)| i);
        Ok(Some(target))
    }

    /// Replaces every match within the current scope, returning the number of replacements performed.
    /// （取代目前範圍內的所有結果，並回傳替換次數。）
    pub fn replace_all(
        &mut self,
        replacement: &str,
        document: &mut Document,
    ) -> Result<usize, SearchError> {
        let engine = SearchEngine::new(document.contents());
        let ReplaceAllOutcome {
            replaced_text,
            replacements,
            ..
        } = engine.replace_all(replacement, &self.options)?;
        if replacements > 0 {
            document.set_contents(replaced_text);
            self.refresh(document)?;
        }
        Ok(replacements)
    }

    /// Marks the currently selected match by recording its line in the provided [`BookmarkManager`].
    /// （在給定的 [`BookmarkManager`] 中記錄目前結果所在行。）
    pub fn mark_current(&mut self, bookmarks: &mut BookmarkManager) -> Option<usize> {
        let idx = self.current?;
        let match_ref = self.matches.get_mut(idx)?;
        if self.marked_lines.insert(match_ref.line) {
            bookmarks.add(match_ref.line);
        }
        match_ref.mark();
        Some(match_ref.line)
    }

    /// Marks every cached match, returning the number of newly marked lines.
    /// （標記所有快取結果並回傳新增的行數。）
    pub fn mark_all(&mut self, bookmarks: &mut BookmarkManager) -> usize {
        let mut count = 0usize;
        for entry in &mut self.matches {
            if self.marked_lines.insert(entry.line) {
                bookmarks.add(entry.line);
                count += 1;
            }
            entry.mark();
        }
        count
    }

    /// Clears all marks previously applied via this session, restoring the bookmark manager.
    /// （清除此會話加上的所有標記，還原書籤管理器。）
    pub fn clear_marks(&mut self, bookmarks: &mut BookmarkManager) {
        for line in self.marked_lines.iter().copied() {
            bookmarks.remove(line);
        }
        self.marked_lines.clear();
        for entry in &mut self.matches {
            entry.clear_mark();
        }
    }

    /// Produces a [`SearchReport`] for the currently cached matches, tagging them with the provided path.
    /// （針對目前快取結果產生 [`SearchReport`]，並附上對應路徑。）
    pub fn report(&self, path: Option<PathBuf>) -> SearchReport {
        if self.matches.is_empty() {
            return SearchReport::default();
        }
        SearchReport::new(vec![FileSearchResult::new(path, self.matches.clone())])
    }

    /// Applies a follow-up query against the cached matches (search-in-results).
    /// （針對快取結果執行次級查詢，實現「結果再搜尋」。）
    pub fn search_in_results(&self, options: &SearchOptions) -> Result<SearchReport, SearchError> {
        self.report(None).search_in_results(options)
    }

    /// Restricts the session to a selection range.
    /// （將會話限制在選取範圍內。）
    pub fn set_selection_scope(&mut self, start: usize, end: usize) {
        self.options.scope = if start == end {
            SearchScope::EntireDocument
        } else {
            SearchScope::Selection { start, end }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_document(contents: &str) -> Document {
        let mut doc = Document::new();
        doc.set_contents(contents);
        doc
    }

    #[test]
    fn session_find_next_and_previous_wraps() {
        let doc = new_document("alpha beta gamma beta");
        let mut session = SearchSession::new(SearchOptions::new("beta")).unwrap();
        session.refresh(&doc).unwrap();

        let first = session.find_next().unwrap();
        assert_eq!(first.line, 1);
        assert_eq!(first.column, 7);

        let second = session.find_next().unwrap();
        assert_eq!(second.column, 18);

        // Wrap-around to the first hit to respect cyclic navigation.
        // 為符合循環導覽，需回繞至第一筆符合項。
        let third = session.find_next().unwrap();
        assert_eq!(third.column, 7);

        // Wrap backwards to the final hit for reverse navigation.
        // 反向搜尋時需回繞至最後一筆符合項。
        let prev = session.find_previous().unwrap();
        assert_eq!(prev.column, 18);
    }

    #[test]
    fn replace_current_updates_document() {
        let mut doc = new_document("foo bar foo");
        let mut session = SearchSession::new(SearchOptions::new("foo")).unwrap();
        session.refresh(&doc).unwrap();
        session.find_next();

        let replaced = session
            .replace_current("baz", &mut doc)
            .unwrap()
            .expect("match expected");
        assert_eq!(replaced.matched, "foo");
        assert_eq!(doc.contents(), "baz bar foo");

        // The next lookup should now reference the second occurrence.
        // 下一次搜尋結果應對應第二個出現位置。
        let next = session.find_next().unwrap();
        assert_eq!(next.column, 9);
    }

    #[test]
    fn replace_all_applies_within_scope() {
        let mut doc = new_document("one two two three");
        let mut options = SearchOptions::new("two");
        options.scope = SearchScope::Selection { start: 4, end: 11 };
        let mut session = SearchSession::new(options).unwrap();
        session.refresh(&doc).unwrap();

        let replacements = session.replace_all("TWO", &mut doc).unwrap();
        assert_eq!(replacements, 2);
        assert_eq!(doc.contents(), "one TWO TWO three");
    }

    #[test]
    fn mark_all_tracks_bookmarks() {
        let doc = new_document("beta beta beta");
        let mut session = SearchSession::new(SearchOptions::new("beta")).unwrap();
        session.refresh(&doc).unwrap();
        let mut bookmarks = BookmarkManager::default();

        let marked = session.mark_all(&mut bookmarks);
        // Multiple hits on the same line collapse into one bookmark.
        // 同一行多次出現僅會計算一次書籤。
        assert_eq!(marked, 1);
        assert!(bookmarks.is_bookmarked(1));

        session.clear_marks(&mut bookmarks);
        assert!(!bookmarks.is_bookmarked(1));
        assert!(session.matches().iter().all(|m| !m.is_marked));
    }

    #[test]
    fn search_in_results_filters_matches() {
        let doc = new_document("hello world\nhi universe\nworldwide");
        let mut session = SearchSession::new(SearchOptions::new("world")).unwrap();
        session.refresh(&doc).unwrap();

        let mut nested_opts = SearchOptions::new("hello");
        let filtered = session.search_in_results(&nested_opts).unwrap();
        assert_eq!(filtered.total_matches, 1);

        nested_opts.pattern = "world".into();
        let filtered = session.search_in_results(&nested_opts).unwrap();
        assert_eq!(filtered.total_matches, 2);
    }
}
