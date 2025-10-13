# Feature 3.10 – Command-Line Interface

## Scope
- Support Notepad++ legacy switches (`-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>`, ...)
- Extended RustNotePad options (`--session`, `--project`, `--theme`, ...)
- Cross-platform invocation and shell integration

## Status Checklist
- [ ] `design.md` drafted and reviewed
- [ ] CLI parser implemented
- [ ] Integration with session/project subsystems
- [ ] Unit tests for argument parsing
- [ ] Integration tests for launch scenarios
- [ ] Documentation updated (`rustnotepad --help`)
- [ ] `compatibility.md` updated

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `crates/cmdline`, `apps/cli`, `apps/gui-tauri`

## Open Questions
- TBD
