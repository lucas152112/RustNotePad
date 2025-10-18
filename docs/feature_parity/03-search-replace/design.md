# Design Draft – Feature 3.3

## Goals
- Provide a single search/replace engine shared by the editor, CLI, and future GUI panels.
- Support literal, regex, case-sensitive, whole-word, forward/backward, and selection-only searches.
- Aggregate multi-file results with metadata suitable for result panels, bookmarking, and “search in results”.
- Allow safe replace workflows that preserve original encodings, line endings, and BOM state.

## Architecture Overview

### `rustnotepad_search` crate
- Core type is `SearchEngine<'a>` which operates on an arbitrary `&str`.  
  - Accepts a `SearchOptions` struct (pattern, mode, flags, scope, direction, wrap behaviour).  
  - Internally translates patterns to a single `regex::Regex`, even for literal searches (`regex::escape`) so all features share one pipeline.  
  - `SearchScope` enforces document vs selection ranges and defaults to the full text.
- Matches are returned as `SearchMatch` objects that record absolute byte start/end offsets, 1-based line/column, trimmed line text, and a `is_marked` flag for bookmark toggling.
- `SearchReport` and `FileSearchResult` wrap vectors of `SearchMatch`. They provide:  
  - Aggregate summaries (`files_with_matches`, `total_matches`).  
  - `mark_where` for bulk bookmark toggling.  
  - `search_in_results` to re-run a query against existing match payloads.
- `ReplaceAllOutcome` packages the rewritten text, replacement count, and original match metadata so callers can apply changes (or preview) without recomputing search hits.
- `search_in_files` consumes an iterator of `FileSearchInput` (path + `Cow<str>`) to build reports for batch/project workflows.
- Whole-word mode uses ASCII word boundaries (`[A-Za-z0-9_]`) enforced post-match so both literal and regex patterns share identical semantics.
- The regex is compiled with `multi_line = true`; `dot_matches_new_line` defaults to `false` but can be toggled per query (`SearchOptions::dot_matches_newline`).

### `rustnotepad_core::search_session`
- `SearchSession` maintains cached matches for a `Document`, handles navigation (`find_next` / `find_previous`) with wrap-around semantics, and exposes the active match.
- Replacement helpers (`replace_current`, `replace_all`) update the underlying `Document` while preserving encoding/EOL metadata via the core document API.
- Bookmark integration: `mark_current`, `mark_all`, and `clear_marks` coordinate with `BookmarkManager`, tracking which lines were set by the search layer to avoid clobbering user bookmarks.
- `SearchSession::report` emits `SearchReport` instances for UI panels, while `search_in_results` chains additional filters without touching the document text.

### Replace workflow
- `SearchEngine::replace_all` performs replacements within the requested scope and returns the full replacement plan.  
- Callers decide whether to persist the new text; the function itself stays pure and side-effect free.
- The CLI reuses the existing `Document` type (from `rustnotepad_core`) so encoding, line-ending, and BOM choices are preserved when saving.

### Multi-file orchestration
- Directory traversal lives in the CLI (`walkdir`) to avoid coupling the crate to filesystem policy.  
- Each file is opened via `Document::open`, then fed into `SearchEngine`. Results are condensed into a single `SearchReport`.
- Search-in-results currently filters at match granularity. The GUI can reuse this by holding onto `SearchReport` between queries.

### CLI integration
- New command: `rustnotepad-cli search <pattern> [paths...]`  
  - Flags: `--regex`, `--case-sensitive`, `--whole-word`, `--dot-matches-newline`, `--replace <text>`, `--apply`.  
  - Prints `path:line:column: line_text` for each match plus an aggregate summary.  
  - When `--replace` is provided without `--apply`, the command performs a dry run; `--apply` writes back via `Document::save()`.
- CLI relies on the same `SearchOptions` struct, ensuring behaviour parity with editor components.

### Editor / GUI considerations
- The GUI can hold a `SearchEngine` per active document or feed selections through on demand.  
- `SearchReport` is designed to back the Find Results dock: each `FileSearchResult` maps to a tree node, each `SearchMatch` to a child leaf with bookmark state.
- Highlight overlays can be generated from `SearchMatch.start/end` offsets without re-running regex.

## Performance Notes
- `regex` handles literal and regex searches, enabling SIMD optimisations automatically.  
- `SearchScope` prevents scanning the rest of the buffer during selection-only searches.  
- Multi-file searches stream results; no indexing layer is introduced yet. We can layer caching/indexing later if performance targets require it.
- Column computations use Unicode-aware character counts to avoid off-by-one with multi-byte graphemes.

## Open Items
- Incremental search previews and live highlighting integration with the editor viewport.
- Background cancellation for large project searches (current API is synchronous).
- Persisting cached results for quick “search-again” across workspace histories.
- GUI affordances for replace preview (diff view, per-hit acceptance).

## Decision Log
- Use the Rust `regex` crate for both literal and regex paths to minimise duplicated logic and gain mature optimisations.
- Represent match locations with 1-based line/column metadata to align with Notepad++ UX and CLI output expectations.
- Keep file traversal outside the core crate so different front-ends (GUI, CLI, future daemon) can inject their own filtering rules.
- Reuse `rustnotepad_core::Document` for write-back to preserve encoding/EOL fidelity during replacements.
- Maintain search state via `SearchSession` so multiple front-ends (GUI panes, macros, plugins) can share identical behaviour and bookmark coordination.
