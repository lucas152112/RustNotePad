# Feature 3.11 – Plugin System

## Scope
- Windows Notepad++ plugin ABI compatibility layer for DLL plugins
- Cross-platform WASM plugin host with permission sandbox
- Plugin management UI (install/update/disable/remove, dependency checks)
- Plugin signature verification and trust model

## Status Checklist
- [ ] `design.md` drafted and reviewed
- [ ] Windows ABI bridge implemented
- [ ] WASM host implemented
- [ ] Plugin admin UI implemented
- [ ] Unit/integration/E2E tests in place
- [ ] `compatibility.md` updated
- [ ] Documentation for plugin authors

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `crates/plugin_winabi`, `crates/plugin_wasm`, `apps/gui-tauri`

## Open Questions
- TBD
