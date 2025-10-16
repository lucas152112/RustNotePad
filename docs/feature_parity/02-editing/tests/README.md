# Test Plan – Feature 3.2

## Unit tests
- Multi-caret primitives (`crates/core/src/editor.rs`)
- Column/rectangular editing (`crates/core/src/column_ops.rs`)
- Line utilities (trim, sort, dedup, case conversion)
- Bookmark/folding/bookend state management

## Integration tests
- Split view orchestration across panes (`split_view` module)
- Document map metrics and recovery
- Safe-save & recovery via `RecoveryManager`

## E2E scenarios
- Headless regression: `cargo test -p rustnotepad_core`
- (Preview) GUI harness hooks via Playwright/Tauri stub commands in `scripts/`

## Tooling
- `cargo test -p rustnotepad_core`
- `cargo test -p rustnotepad_core --lib -- --ignored` (reserved for future stress suites)
- Playwright/Tauri automation harness (wired into CI once GUI shell lands)
