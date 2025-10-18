use std::collections::BTreeSet;
use std::path::PathBuf;

use rustnotepad_search::{
    FileSearchResult, ReplaceAllOutcome, SearchDirection, SearchEngine, SearchError, SearchMatch,
    SearchOptions, SearchReport, SearchScope,
};

use crate::{BookmarkManager, Document};

/// Tracks the active search context for a document, including cached matches and bookmark marks.
#[derive(Debug, Clone)]
pub struct SearchSession {
    options: SearchOptions,
    matches: Vec<SearchMatch>,
    current: Option<usize>,
    marked_lines: BTreeSet<usize>,
}

impl SearchSession {
    /// Creates a new session with the provided options. The caller must invoke [`refresh`] before searching.
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
    pub fn options(&self) -> &SearchOptions {
        &self.options
    }

    /// Returns a mutable reference to the options, clearing cached state.
    pub fn options_mut(&mut self) -> &mut SearchOptions {
        self.matches.clear();
        self.current = None;
        self.marked_lines.clear();
        &mut self.options
    }

    /// Recomputes matches against the given document contents using the current options.
    pub fn refresh(&mut self, document: &Document) -> Result<(), SearchError> {
        let engine = SearchEngine::new(document.contents());
        self.matches = engine.find_all(&self.options)?;
        self.current = None;
        self.marked_lines.clear();
        Ok(())
    }

    /// Returns all cached matches (refresh must be called beforehand).
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Indicates whether cached results are empty.
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Selects the next match, respecting wrap-around rules.
    pub fn find_next(&mut self) -> Option<&SearchMatch> {
        self.advance(SearchDirection::Forward)
    }

    /// Selects the previous match, respecting wrap-around rules.
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
    pub fn current(&self) -> Option<&SearchMatch> {
        self.current.and_then(|idx| self.matches.get(idx))
    }

    /// Replaces the currently selected match (if any) with the provided text.
    /// Returns the original match that was replaced.
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
    pub fn report(&self, path: Option<PathBuf>) -> SearchReport {
        if self.matches.is_empty() {
            return SearchReport::default();
        }
        SearchReport::new(vec![FileSearchResult::new(path, self.matches.clone())])
    }

    /// Applies a follow-up query against the cached matches (search-in-results).
    pub fn search_in_results(&self, options: &SearchOptions) -> Result<SearchReport, SearchError> {
        self.report(None).search_in_results(options)
    }

    /// Restricts the session to a selection range.
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

        // Wrap-around to first hit.
        let third = session.find_next().unwrap();
        assert_eq!(third.column, 7);

        // Backwards wrap to last hit.
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

        // The next match should now point at the second occurrence.
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
        assert_eq!(marked, 1); // multiple hits on same line collapse to a single bookmark
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
