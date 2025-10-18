# Test Plan – Feature 3.4

## Unit tests
- [x] Layout serialization/deserialization (`crates/settings::layout::tests`)
- [x] Theme parsing and validation (`crates/settings::theme::tests`)
- [ ] Status bar data providers (requires wiring to editor metrics)

## Integration tests
- [x] 布局與主題回歸測試 (`crates/settings/tests/ui_layout_regression.rs`) 確保預設視圖、釘選標籤與主題切換狀態。 / Layout + theme regression via `crates/settings/tests/ui_layout_regression.rs`.
- [ ] Layout persistence across restarts (pending storage layer)
- [ ] Multi-language UI rendering sanity checks

## E2E scenarios
- Split/dock interactions via GUI automation
- Document map navigation accuracy
- Theme import/export workflows

## Tooling
- `cargo test --package settings`
- GUI automation (Playwright/Tauri)
- Snapshot testing for theming
