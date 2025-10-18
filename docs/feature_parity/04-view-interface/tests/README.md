# Test Plan – Feature 3.4

## Unit tests
- [x] Layout serialization/deserialization (`crates/settings::layout::tests`)
- [x] Theme parsing and validation (`crates/settings::theme::tests`)
- [ ] Status bar data providers (requires wiring to editor metrics)

## Integration tests
- [ ] Layout persistence across restarts (pending storage layer)
- [ ] Theme switching with live documents
- [ ] Multi-language UI rendering sanity checks

## E2E scenarios
- Split/dock interactions via GUI automation
- Document map navigation accuracy
- Theme import/export workflows

## Tooling
- `cargo test --package settings`
- GUI automation (Playwright/Tauri)
- Snapshot testing for theming
