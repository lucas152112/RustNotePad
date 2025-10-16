# Feature 3.2 – Editing Fundamentals

## Scope
- Multi-caret, rectangular selection, and column editing mode
- Line operations (trim, sort, duplicate removal), indentation control, case conversion
- Bookmarks, code folding, line numbers, gutter indicators, document map
- Split views, drag and drop between views/instances, multi-instance strategy
- Safe save semantics (permissions, temp files, crash resilience)

## Status Checklist
- [x] `design.md` drafted and reviewed
- [ ] Editing engine implementation complete
- [ ] Automated unit tests implemented
- [ ] Integration tests (split views, multi-instance)
- [ ] E2E GUI regression scripted
- [ ] `compatibility.md` updated with differences
- [ ] Documentation user guide updates

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `crates/core`, `crates/settings`, `apps/gui-tauri`

## Open Questions
- TBD
