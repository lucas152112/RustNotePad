# Feature 3.1 – Files, Encoding & Line Endings

## Scope
- Open, create, save, save-as, and restore unsaved documents
- Encoding detection and conversion (UTF-8/UTF-16/multi-byte legacy encodings) with BOM handling
- Line ending detection and switching between LF/CRLF/CR
- Recent file list, file associations, and external file monitoring with reload prompts
- CLI parity: batch conversion via `rustnotepad-cli convert --from <enc> --to <enc>`

## Status Checklist
- [x] `design.md` drafted and reviewed (M1 UTF-8 focus)
- [x] Core module implementation complete (M1 UTF-8 focus)
- [x] Automated unit tests implemented
- [ ] Integration / CLI tests implemented
- [ ] E2E regression scripted
- [ ] `compatibility.md` populated with behaviour diffs
- [ ] Documentation & user guidance updated

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Test plans: `tests/`
- Related crates: `crates/core`, `crates/settings`, `apps/cli`

## Open Questions
- How to extend encoding detection beyond UTF-8 while retaining performance guarantees?
- Cross-platform file monitoring API selection.
