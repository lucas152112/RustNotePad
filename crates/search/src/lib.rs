//! Search and replace engine used across RustNotePad components.
//!
//! The implementation covers single-document, selection-limited, and multi-file
//! search workflows with support for regex, whole-word, case sensitivity, and
//! forward/backward traversal. Replace helpers expose the edits required to
//! update the underlying buffers, while `SearchReport` aggregates results for
//! UI consumption (result panels, bookmarks, search-in-results).

use std::borrow::Cow;
use std::ops::Range;
use std::path::PathBuf;

use regex::{Regex, RegexBuilder};
use thiserror::Error;

/// Error conditions raised by the search engine.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SearchError {
    #[error("search pattern cannot be empty")]
    EmptyPattern,
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),
}

/// Determines how the search pattern is interpreted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Plain,
    Regex,
}

/// Direction for iterative searches (`Find Next` / `Find Previous`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

impl Default for SearchDirection {
    fn default() -> Self {
        Self::Forward
    }
}

/// Search target scope within a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchScope {
    EntireDocument,
    Selection { start: usize, end: usize },
}

impl Default for SearchScope {
    fn default() -> Self {
        Self::EntireDocument
    }
}

impl SearchScope {
    fn resolve(&self, text_len: usize) -> Range<usize> {
        match *self {
            SearchScope::EntireDocument => 0..text_len,
            SearchScope::Selection { start, end } => {
                if start == end {
                    return 0..text_len;
                }
                let lo = start.min(end).min(text_len);
                let hi = end.max(start).min(text_len);
                lo..hi
            }
        }
    }
}

/// Options supplied to the search engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchOptions {
    pub pattern: String,
    pub mode: SearchMode,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub direction: SearchDirection,
    pub wrap_around: bool,
    pub scope: SearchScope,
    pub dot_matches_newline: bool,
}

impl SearchOptions {
    /// Creates a new option set for the specified pattern with sensible defaults.
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            mode: SearchMode::Plain,
            case_sensitive: false,
            whole_word: false,
            direction: SearchDirection::Forward,
            wrap_around: true,
            scope: SearchScope::EntireDocument,
            dot_matches_newline: false,
        }
    }

    pub fn validate(&self) -> Result<(), SearchError> {
        if self.pattern.is_empty() {
            return Err(SearchError::EmptyPattern);
        }
        Ok(())
    }
}

/// Represents a single match produced by a search query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub matched: String,
    pub line_text: String,
    pub is_marked: bool,
}

impl SearchMatch {
    /// Marks the match, typically used when user toggles bookmarks from the search panel.
    pub fn mark(&mut self) {
        self.is_marked = true;
    }

    /// Clears the mark on the match.
    pub fn clear_mark(&mut self) {
        self.is_marked = false;
    }
}

/// Aggregated search results for a file or document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSearchResult {
    pub path: Option<PathBuf>,
    pub matches: Vec<SearchMatch>,
}

impl FileSearchResult {
    pub fn new(path: Option<PathBuf>, matches: Vec<SearchMatch>) -> Self {
        Self { path, matches }
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

/// Summary of search results, used for UI counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchSummary {
    pub total_matches: usize,
    pub files_with_matches: usize,
}

/// Result set for batch searches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchReport {
    pub results: Vec<FileSearchResult>,
    pub total_matches: usize,
}

impl SearchReport {
    pub fn new(results: Vec<FileSearchResult>) -> Self {
        let total_matches = results.iter().map(|entry| entry.matches.len()).sum();
        Self {
            results,
            total_matches,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_matches == 0
    }

    /// Returns a compact summary with aggregate statistics.
    pub fn summary(&self) -> SearchSummary {
        let files_with_matches = self
            .results
            .iter()
            .filter(|entry| !entry.matches.is_empty())
            .count();
        SearchSummary {
            total_matches: self.total_matches,
            files_with_matches,
        }
    }

    /// Marks every match satisfying the predicate, returning the number of matches affected.
    pub fn mark_where<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(&SearchMatch) -> bool,
    {
        let mut count = 0;
        for entry in &mut self.results {
            for m in &mut entry.matches {
                if predicate(m) {
                    m.mark();
                    count += 1;
                }
            }
        }
        count
    }

    /// Filters the result set by applying a secondary search against each match's line text.
    /// This implements the "search in results" workflow.
    pub fn search_in_results(&self, options: &SearchOptions) -> Result<SearchReport, SearchError> {
        options.validate()?;
        let mut filtered_entries = Vec::new();
        for entry in &self.results {
            let mut retained = Vec::new();
            for m in &entry.matches {
                let mut nested_opts = options.clone();
                nested_opts.scope = SearchScope::EntireDocument;
                let nested_engine = SearchEngine::new(&m.line_text);
                if nested_engine.find_all(&nested_opts)?.is_empty() {
                    continue;
                }
                retained.push(m.clone());
            }
            if !retained.is_empty() {
                filtered_entries.push(FileSearchResult::new(entry.path.clone(), retained));
            }
        }
        Ok(SearchReport::new(filtered_entries))
    }
}

/// Captures the outcome of a `replace_all` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceAllOutcome {
    pub replaced_text: String,
    pub replacements: usize,
    pub matches: Vec<SearchMatch>,
}

/// Represents a batch search input (e.g., when scanning a workspace).
#[derive(Clone, Debug)]
pub struct FileSearchInput<'a> {
    pub path: PathBuf,
    pub contents: Cow<'a, str>,
}

impl<'a> FileSearchInput<'a> {
    pub fn new(path: impl Into<PathBuf>, contents: impl Into<Cow<'a, str>>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }
}

/// Search engine bound to a particular text buffer.
pub struct SearchEngine<'a> {
    text: &'a str,
    line_index: LineIndex<'a>,
}

impl<'a> SearchEngine<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            line_index: LineIndex::new(text),
        }
    }

    /// Finds the next match according to the search options, starting from the given byte index.
    pub fn find(
        &self,
        start_pos: usize,
        options: &SearchOptions,
    ) -> Result<Option<SearchMatch>, SearchError> {
        options.validate()?;
        let regex = build_regex(options)?;
        let text_len = self.text.len();
        let scope = options.scope.resolve(text_len);
        if scope.is_empty() {
            return Ok(None);
        }
        let bounded_start = start_pos.clamp(scope.start, scope.end);
        let offset = scope.start;
        let subset = &self.text[scope.clone()];
        let entries = self.collect_prepared_matches(&regex, subset, offset, options);
        if entries.is_empty() {
            return Ok(None);
        }
        let relative_cursor = bounded_start.saturating_sub(offset);

        match options.direction {
            SearchDirection::Forward => {
                for entry in &entries {
                    if entry.rel_start >= relative_cursor
                        || (relative_cursor > entry.rel_start && relative_cursor < entry.rel_end)
                    {
                        return Ok(Some(self.build_match(
                            entry.abs_start,
                            entry.abs_end,
                            &entry.matched,
                        )));
                    }
                }
                if options.wrap_around {
                    if let Some(entry) = entries.first() {
                        return Ok(Some(self.build_match(
                            entry.abs_start,
                            entry.abs_end,
                            &entry.matched,
                        )));
                    }
                }
                Ok(None)
            }
            SearchDirection::Backward => {
                let mut candidate: Option<&PreparedMatch> = None;
                for entry in &entries {
                    if entry.rel_end <= relative_cursor {
                        candidate = Some(entry);
                    }
                }
                if let Some(entry) = candidate {
                    return Ok(Some(self.build_match(
                        entry.abs_start,
                        entry.abs_end,
                        &entry.matched,
                    )));
                }
                if options.wrap_around {
                    if let Some(entry) = entries.last() {
                        return Ok(Some(self.build_match(
                            entry.abs_start,
                            entry.abs_end,
                            &entry.matched,
                        )));
                    }
                }
                Ok(None)
            }
        }
    }

    /// Returns all matches that satisfy the given options within the configured scope.
    pub fn find_all(&self, options: &SearchOptions) -> Result<Vec<SearchMatch>, SearchError> {
        options.validate()?;
        let regex = build_regex(options)?;
        let text_len = self.text.len();
        let scope = options.scope.resolve(text_len);
        if scope.is_empty() {
            return Ok(Vec::new());
        }

        let subset = &self.text[scope.clone()];
        let offset = scope.start;
        let matches = self
            .collect_prepared_matches(&regex, subset, offset, options)
            .into_iter()
            .map(|entry| self.build_match(entry.abs_start, entry.abs_end, &entry.matched))
            .collect();
        Ok(matches)
    }

    /// Produces a report for the current document, omitting empty results.
    pub fn report(&self, options: &SearchOptions) -> Result<SearchReport, SearchError> {
        let matches = self.find_all(options)?;
        if matches.is_empty() {
            return Ok(SearchReport::default());
        }
        Ok(SearchReport::new(vec![FileSearchResult::new(
            None, matches,
        )]))
    }

    /// Applies `replace_all` within the given scope and returns the updated text along with match metadata.
    pub fn replace_all(
        &self,
        replacement: &str,
        options: &SearchOptions,
    ) -> Result<ReplaceAllOutcome, SearchError> {
        options.validate()?;
        let regex = build_regex(options)?;
        let text_len = self.text.len();
        let scope = options.scope.resolve(text_len);
        if scope.is_empty() {
            return Ok(ReplaceAllOutcome {
                replaced_text: self.text.to_string(),
                replacements: 0,
                matches: Vec::new(),
            });
        }

        let subset = &self.text[scope.clone()];
        let offset = scope.start;
        let mut matches: Vec<SearchMatch> = Vec::new();
        let mut replaced_segment = String::with_capacity(subset.len());
        let mut last = 0usize;

        for caps in regex.captures_iter(subset) {
            let m = caps
                .get(0)
                .expect("regex::captures_iter should always yield group 0");
            let rel_start = m.start();
            let rel_end = m.end();
            let abs_start = offset + rel_start;
            let abs_end = offset + rel_end;

            if options.whole_word && !self.is_whole_word(abs_start, abs_end) {
                continue;
            }

            matches.push(self.build_match(abs_start, abs_end, m.as_str()));
            replaced_segment.push_str(&subset[last..rel_start]);
            if matches!(options.mode, SearchMode::Regex) {
                caps.expand(replacement, &mut replaced_segment);
            } else {
                replaced_segment.push_str(replacement);
            }
            last = rel_end;
        }

        if matches.is_empty() {
            return Ok(ReplaceAllOutcome {
                replaced_text: self.text.to_string(),
                replacements: 0,
                matches,
            });
        }

        replaced_segment.push_str(&subset[last..]);

        let mut new_text =
            String::with_capacity(self.text.len() - subset.len() + replaced_segment.len());
        new_text.push_str(&self.text[..scope.start]);
        new_text.push_str(&replaced_segment);
        new_text.push_str(&self.text[scope.end..]);

        Ok(ReplaceAllOutcome {
            replaced_text: new_text,
            replacements: matches.len(),
            matches,
        })
    }

    fn build_match(&self, start: usize, end: usize, matched: &str) -> SearchMatch {
        let (line, column) = self.line_index.line_and_column(start);
        let line_text = self.line_index.line_text(line);
        SearchMatch {
            start,
            end,
            line,
            column,
            matched: matched.to_string(),
            line_text,
            is_marked: false,
        }
    }

    fn collect_prepared_matches(
        &self,
        regex: &Regex,
        subset: &str,
        offset: usize,
        options: &SearchOptions,
    ) -> Vec<PreparedMatch> {
        regex
            .captures_iter(subset)
            .filter_map(|caps| {
                let m = caps.get(0)?;
                let rel_start = m.start();
                let rel_end = m.end();
                let abs_start = offset + rel_start;
                let abs_end = offset + rel_end;
                if options.whole_word && !self.is_whole_word(abs_start, abs_end) {
                    return None;
                }
                Some(PreparedMatch {
                    rel_start,
                    rel_end,
                    abs_start,
                    abs_end,
                    matched: m.as_str().to_string(),
                })
            })
            .collect()
    }

    fn is_whole_word(&self, start: usize, end: usize) -> bool {
        let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
        let bytes = self.text.as_bytes();
        let left = if start == 0 {
            false
        } else {
            bytes
                .get(start.saturating_sub(1))
                .map_or(false, |b| is_word(*b))
        };
        let right = if end >= bytes.len() {
            false
        } else {
            bytes.get(end).map_or(false, |b| is_word(*b))
        };
        !(left || right)
    }
}

/// Executes a search over many files, producing a summarised report.
pub fn search_in_files<'a, I>(
    inputs: I,
    options: &SearchOptions,
) -> Result<SearchReport, SearchError>
where
    I: IntoIterator<Item = FileSearchInput<'a>>,
{
    options.validate()?;
    let mut results = Vec::new();
    for input in inputs {
        let mut scoped_options = options.clone();
        scoped_options.scope = SearchScope::EntireDocument;
        let engine = SearchEngine::new(&input.contents);
        let matches = engine.find_all(&scoped_options)?;
        if matches.is_empty() {
            continue;
        }
        results.push(FileSearchResult::new(Some(input.path), matches));
    }
    Ok(SearchReport::new(results))
}

#[derive(Clone)]
struct PreparedMatch {
    rel_start: usize,
    rel_end: usize,
    abs_start: usize,
    abs_end: usize,
    matched: String,
}

fn build_regex(options: &SearchOptions) -> Result<Regex, SearchError> {
    let mut builder = RegexBuilder::new(&translate_pattern(options));
    builder.case_insensitive(!options.case_sensitive);
    builder.multi_line(true);
    builder.dot_matches_new_line(options.dot_matches_newline);
    builder
        .build()
        .map_err(|err| SearchError::InvalidPattern(err.to_string()))
}

fn translate_pattern(options: &SearchOptions) -> String {
    match options.mode {
        SearchMode::Plain => regex::escape(&options.pattern),
        SearchMode::Regex => options.pattern.clone(),
    }
}

#[derive(Clone)]
struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        let mut starts = Vec::new();
        starts.push(0);
        for (idx, ch) in text.char_indices() {
            if ch == '\n' {
                if idx + ch.len_utf8() <= text.len() {
                    starts.push(idx + ch.len_utf8());
                }
            }
        }
        Self { text, starts }
    }

    fn line_and_column(&self, index: usize) -> (usize, usize) {
        let pos = match self.starts.binary_search(&index) {
            Ok(line_zero) => line_zero,
            Err(insert) => insert.saturating_sub(1),
        };
        let line_start = self.starts.get(pos).copied().unwrap_or(0);
        let column = self.text[line_start..index]
            .chars()
            .count()
            .saturating_add(1);
        (pos + 1, column)
    }

    fn line_text(&self, line: usize) -> String {
        let zero_based = line.saturating_sub(1);
        let start = *self.starts.get(zero_based).unwrap_or(&0);
        let end = self
            .starts
            .get(zero_based + 1)
            .copied()
            .unwrap_or_else(|| self.text.len());
        self.text[start..end]
            .trim_end_matches(|c| c == '\n' || c == '\r')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn opts(pattern: &str) -> SearchOptions {
        SearchOptions::new(pattern)
    }

    #[test]
    fn find_forward_plain_case_insensitive() {
        let engine = SearchEngine::new("Hello world\nHELLO WORLD");
        let mut options = opts("hello");
        options.case_sensitive = false;
        let first = engine.find(0, &options).unwrap().unwrap();
        assert_eq!(first.start, 0);
        assert_eq!(first.line, 1);

        let second = engine.find(first.end, &options).unwrap().unwrap();
        assert_eq!(second.line, 2);
        assert_eq!(second.column, 1);
    }

    #[test]
    fn find_respects_case_sensitivity() {
        let engine = SearchEngine::new("Hello world");
        let mut options = opts("hello");
        options.case_sensitive = true;
        assert!(engine.find(0, &options).unwrap().is_none());
    }

    #[test]
    fn whole_word_skips_partial_matches() {
        let engine = SearchEngine::new("cat scatter catalog");
        let mut options = opts("cat");
        options.whole_word = true;
        let matches = engine.find_all(&options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 1);
    }

    #[test]
    fn backward_search_returns_previous_match() {
        let text = "alpha beta gamma beta";
        let engine = SearchEngine::new(text);
        let mut options = opts("beta");
        options.direction = SearchDirection::Backward;
        let result = engine.find(text.len(), &options).unwrap().unwrap();
        assert_eq!(result.line, 1);
        assert_eq!(result.column, 18);
    }

    #[test]
    fn regex_search_supports_groups() {
        let engine = SearchEngine::new("fn add(a: i32, b: i32) {}");
        let mut options = opts(r"\b[a-z]+\(");
        options.mode = SearchMode::Regex;
        options.case_sensitive = true;
        let matches = engine.find_all(&options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched, "add(");
    }

    #[test]
    fn search_within_selection_limits_scope() {
        let engine = SearchEngine::new("one two two three");
        let mut options = opts("two");
        options.scope = SearchScope::Selection { start: 4, end: 7 };
        let matches = engine.find_all(&options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start, 4);
    }

    #[test]
    fn replace_all_plain() {
        let engine = SearchEngine::new("foo bar foo");
        let options = opts("foo");
        let outcome = engine.replace_all("baz", &options).unwrap();
        assert_eq!(outcome.replacements, 2);
        assert_eq!(outcome.replaced_text, "baz bar baz");
    }

    #[test]
    fn replace_all_regex_with_captures() {
        let engine = SearchEngine::new("let x = 10;\nlet y = 20;");
        let mut options = opts(r"let (\w+) = (\d+);");
        options.mode = SearchMode::Regex;
        let outcome = engine.replace_all("const $1: i32 = $2;", &options).unwrap();
        assert_eq!(outcome.replacements, 2);
        assert!(outcome
            .replaced_text
            .contains("const x: i32 = 10;\nconst y: i32 = 20;"));
    }

    #[test]
    fn multi_file_search_collects_summary() {
        let options = opts("needle");
        let report = search_in_files(
            [
                FileSearchInput::new(Path::new("a.txt"), "no match"),
                FileSearchInput::new(Path::new("b.txt"), "find the needle\nanother needle"),
            ],
            &options,
        )
        .unwrap();

        assert_eq!(report.total_matches, 2);
        assert_eq!(report.summary().files_with_matches, 1);
    }

    #[test]
    fn search_in_results_filters_entries() {
        let mut options = opts("world");
        let base = SearchReport::new(vec![FileSearchResult::new(
            Some(PathBuf::from("file.txt")),
            vec![
                SearchMatch {
                    start: 0,
                    end: 5,
                    line: 1,
                    column: 1,
                    matched: "hello".into(),
                    line_text: "hello friend".into(),
                    is_marked: false,
                },
                SearchMatch {
                    start: 12,
                    end: 17,
                    line: 2,
                    column: 1,
                    matched: "world".into(),
                    line_text: "hello world".into(),
                    is_marked: false,
                },
            ],
        )]);

        options.case_sensitive = false;
        let filtered = base.search_in_results(&options).unwrap();
        assert_eq!(filtered.total_matches, 1);
        assert_eq!(filtered.results[0].matches[0].matched, "world");
    }

    #[test]
    fn mark_where_marks_expected_matches() {
        let report = SearchReport::new(vec![FileSearchResult::new(
            Some(PathBuf::from("example.txt")),
            vec![
                SearchMatch {
                    start: 0,
                    end: 3,
                    line: 1,
                    column: 1,
                    matched: "foo".into(),
                    line_text: "foo bar".into(),
                    is_marked: false,
                },
                SearchMatch {
                    start: 4,
                    end: 7,
                    line: 1,
                    column: 5,
                    matched: "bar".into(),
                    line_text: "foo bar".into(),
                    is_marked: false,
                },
            ],
        )]);

        let mut report = report;
        let count = report.mark_where(|m| m.matched == "foo");
        assert_eq!(count, 1);
        assert!(report.results[0].matches[0].is_marked);
        assert!(!report.results[0].matches[1].is_marked);
    }
}
