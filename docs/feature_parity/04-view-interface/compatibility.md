# Compatibility Notes – Feature 3.4

## Known differences
- Tab drag/drop and split reordering are mocked; actual docking interactions will follow once integrated with Tauri.
- Toolbar theme selector does not yet import/export Notepad++ `.xml` themes (JSON only in this milestone).
- UI language dropdown updates metadata but does not load translation files (strings remain English placeholders).
- Document map renders text lines only (no minimap scaling or syntax colour overlays).

## Validation checklist
- [x] Tab pin/lock behaviour mirrored in layout state (pending drag/drop)
- [ ] Theme import/export compatibility with Notepad++ `.xml`
- [ ] UI translations cross-checked with localisation assets
- [ ] Document map zoom/scroll fidelity
