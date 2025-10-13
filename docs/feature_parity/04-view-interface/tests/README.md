# Test Plan – Feature 3.4

## Unit tests
- Layout serialization/deserialization
- Theme parsing and validation
- Status bar data providers

## Integration tests
- Layout persistence across restarts
- Theme switching with live documents
- Multi-language UI rendering sanity checks

## E2E scenarios
- Split/dock interactions via GUI automation
- Document map navigation accuracy
- Theme import/export workflows

## Tooling
- `cargo test --package settings`
- GUI automation (Playwright/Tauri)
- Snapshot testing for theming
