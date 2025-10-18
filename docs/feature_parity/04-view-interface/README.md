# Feature 3.4 – View & Interface

## Scope
- Tab management, pin/lock, color tagging
- Document map, status bar, sidebar panels
- Theme and font settings, UI language switching
- Layout persistence, split panes, docking behaviours

## Status Checklist
- [x] `design.md` drafted and reviewed
- [x] UI layout system implementation
- [x] Theme/appearance management implemented
- [x] Unit tests for layout serialization
- [x] E2E UI regression coverage (`crates/settings/tests/ui_layout_regression.rs`)
- [x] `compatibility.md` updated with differences
- [x] Documentation / screenshots updated

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `apps/gui-tauri`, `crates/settings`, `assets/themes`

## Open Questions
- How should interactive docking (drag/drop panes) be modelled once the Tauri shell is wired to real window handles?
- Which additional palette entries are required for plugin panels and diff viewers?
