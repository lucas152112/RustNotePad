# Test Plan – Feature 3.3

## Unit tests
- [x] Pattern parsing and flag combinations (`crates/search` unit suite)
- [x] Regex engine lookaround / multi-line coverage (`replace_all_regex_with_captures`)
- [x] Result deduplication utilities (`search_in_results_filters_entries`, `mark_where_marks_expected_matches`)

## Integration tests
- [x] Multi-file search via CLI traversal (`apps/cli/tests/search.rs`)
- [x] Search-in-results workflow (`crates/core::search_session::tests::search_in_results_filters_matches`)
- [x] Bookmark toggling orchestration (`crates/core::search_session::tests::mark_all_tracks_bookmarks`)

## E2E scenarios
- [x] Large workspace search sanity (`tests/search_large_workspace.rs`)
- [x] Incremental search responsiveness (`crates/core::search_session::tests::session_find_next_and_previous_wraps`)
- [x] Replace in selection with multi-caret (`crates/core::search_session::tests::replace_all_applies_within_scope`)

## Tooling
- `cargo test --package search`
- CLI smoke tests via `rustnotepad-cli search`
- Workspace regression scenarios under `tests/`
