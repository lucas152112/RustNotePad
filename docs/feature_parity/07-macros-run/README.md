# Feature 3.7 – Macros & Run

## Scope
- Macro recording, naming, shortcut assignment, save/load
- Playback with repeat counts and scripting hooks
- Run menu for external tools (working directory, env vars, I/O piping)
- Output console integration

## Status Checklist
- [ ] `design.md` drafted and reviewed
- [ ] Macro recorder and player implemented
- [ ] Run/external tool executor implemented
- [ ] Unit tests for macro serialization
- [ ] Integration tests for process execution sandbox
- [ ] E2E coverage for macro/run UI
- [ ] `compatibility.md` updated

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `crates/macros`, `crates/runexec`, `apps/gui-tauri`

## Open Questions
- TBD
