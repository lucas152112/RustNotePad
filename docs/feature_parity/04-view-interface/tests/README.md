# Test Plan – Feature 3.4（測試計畫 – 功能 3.4）

## Unit tests / 單元測試
- [x] Layout serialization/deserialization (`crates/settings::layout::tests`)  
  版面序列化/還原測試（`crates/settings::layout::tests`）
- [x] Theme parsing and validation (`crates/settings::theme::tests`)  
  主題解析與驗證（`crates/settings::theme::tests`）
- [ ] Status bar data providers (requires wiring to editor metrics)  
  狀態列資料提供者（需接上編輯器指標）

## Integration tests / 整合測試
- [x] Layout + theme regression via `crates/settings/tests/ui_layout_regression.rs`（確保預設視圖、釘選標籤與主題切換狀態）  
  `crates/settings/tests/ui_layout_regression.rs` 進行版面與主題回歸測試，確認預設視圖、釘選標籤與主題切換。
- [ ] Layout persistence across restarts (pending storage layer)  
  重啟後版面持久化（待儲存層完成）
- [ ] Multi-language UI rendering sanity checks  
  多語系 UI 渲染檢查

## E2E scenarios / 端到端情境
- Split/dock interactions via GUI automation  
  透過 GUI 自動化驗證分割/停駐互動
- Document map navigation accuracy  
  文件地圖導覽精確度
- Theme import/export workflows  
  主題匯入/匯出流程

## Tooling / 測試工具
- `cargo test --package settings`  
  `cargo test --package settings`
- GUI automation (Playwright/Tauri)  
  GUI 自動化（Playwright/Tauri）
- Snapshot testing for theming  
  主題快照測試
