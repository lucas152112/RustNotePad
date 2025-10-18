# Feature 3.3 – Search and Replace

## Scope
- In-file, selection-only, multi-file, and project-wide search workflows
- Regex, reverse search, case sensitivity, whole word matching options
- Marking results, search-in-results, and result panel summaries
- Jump-to-line/column helpers, bookmarking integration

## Status Checklist
- [x] `design.md` drafted and reviewed
- [x] Search engine implementation complete
- [x] Unit tests for pattern handling
- [x] Integration tests across workspace/project
- [x] E2E search panel coverage
- [x] `compatibility.md` updated
- [x] Documentation & tutorials refreshed

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Tutorial: `tutorial.md`
- Related crates: `crates/search`, `crates/project`, `apps/gui-tauri`

## Quickstart
- **CLI**: `rustnotepad-cli search <pattern> [paths...] [--regex] [--case-sensitive] [--whole-word] [--dot-matches-newline] [--replace <text>] [--apply]`
  - Omit `--apply` for a dry-run diff; include it to persist replacements.
  - Directory arguments recurse automatically (WalkDir); mix files and directories freely.
- **Programmatic API**: `rustnotepad_core::SearchSession`
  - Instantiate with `SearchOptions`, call `refresh(&Document)` to populate matches.
  - Navigate via `find_next/find_previous`, replace with `replace_current` / `replace_all`.
  - Integrate bookmarks using `mark_current` / `mark_all` / `clear_marks`.
  - Generate result panels through `SearchSession::report()` or chain `search_in_results`.

## Open Questions
- GUI integration timeline for incremental/highlighted search.
- Background cancellation strategy for project-wide scans.
