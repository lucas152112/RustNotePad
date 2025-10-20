# Test Plan – Feature 3.2（測試計畫 – 功能 3.2）

## Unit tests / 單元測試
- Multi-caret primitives (`crates/core/src/editor.rs`)  
  多游標基礎操作（`crates/core/src/editor.rs`）
- Column/rectangular editing (`crates/core/src/column_ops.rs`)  
  欄位/矩形編輯（`crates/core/src/column_ops.rs`）
- Line utilities (trim, sort, dedup, case conversion)  
  行級工具（修剪、排序、去重、大小寫轉換）
- Bookmark/folding/bookend state management  
  書籤、摺疊與邊界狀態管理

## Integration tests / 整合測試
- Split view orchestration across panes (`split_view` module)  
  分割視窗協同流程（`split_view` 模組）
- Document map metrics and recovery  
  文件地圖的統計與復原
- Safe-save & recovery via `RecoveryManager`  
  `RecoveryManager` 管理的安全儲存與復原

## E2E scenarios / 端到端情境
- Headless regression: `cargo test -p rustnotepad_core`  
  無頭回歸：`cargo test -p rustnotepad_core`
- (Preview) GUI harness hooks via Playwright/Tauri stub commands in `scripts/`  
  （預覽）透過 `scripts/` 內的 Playwright/Tauri stub 指令串接 GUI 測試

## Tooling / 測試工具
- `cargo test -p rustnotepad_core`  
  `cargo test -p rustnotepad_core`
- `cargo test -p rustnotepad_core --lib -- --ignored` (reserved for future stress suites)  
  `cargo test -p rustnotepad_core --lib -- --ignored`（保留給未來壓力測試）
- Playwright/Tauri automation harness (wired into CI once GUI shell lands)  
  Playwright/Tauri 自動化框架（GUI 上線後接入 CI）
