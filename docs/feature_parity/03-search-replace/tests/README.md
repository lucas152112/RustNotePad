# Test Plan – Feature 3.3

## Unit tests
- Pattern parsing and flag combinations
- Regex engine edge cases (lookaround, multi-line)
- Result deduplication utilities

## Integration tests
- Project-wide search indexing and caching
- Search-in-results workflow
- Bookmark toggling from search panel

## E2E scenarios
- Large workspace search performance benchmark
- Incremental search responsiveness
- Replace in selection with multi-caret

## Tooling
- `cargo test --package search`
- CLI smoke tests via `rustnotepad-cli search`
- GUI automation for search panels
