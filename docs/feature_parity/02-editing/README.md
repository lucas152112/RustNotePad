# Feature 3.2 – Editing Fundamentals

## Scope
- Multi-caret, rectangular selection, and column editing mode
- Line operations (trim, sort, duplicate removal), indentation control, case conversion
- Bookmarks, code folding, line numbers, gutter indicators, document map
- Split views, drag and drop between views/instances, multi-instance strategy
- Safe save semantics (permissions, temp files, crash resilience)

## Status Checklist
- [x] `design.md` drafted and reviewed
- [x] Editing engine implementation complete
- [x] Automated unit tests implemented
- [x] Integration tests (split views, multi-instance)
- [x] E2E regression harness scripted
- [x] `compatibility.md` updated with differences
- [x] Documentation user guide updates

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `crates/core`, `crates/settings`, `apps/gui-tauri`

## Open Questions
- None.
