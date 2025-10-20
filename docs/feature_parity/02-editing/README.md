# Feature 3.2 – Editing Fundamentals（功能 3.2 – 編輯基礎）

## Scope / 範圍
- Multi-caret, rectangular selection, and column editing mode  
  多游標、矩形選取與欄位編輯模式
- Line operations (trim, sort, duplicate removal), indentation control, case conversion  
  行操作（修剪、排序、移除重複）、縮排控制與大小寫轉換
- Bookmarks, code folding, line numbers, gutter indicators, document map  
  書籤、程式碼摺疊、行號、邊欄指示器、文件地圖
- Split views, drag and drop between views/instances, multi-instance strategy  
  視窗分割、跨檢視/執行個體拖放、多執行個體策略
- Safe save semantics (permissions, temp files, crash resilience)  
  安全儲存語意（權限、暫存檔、當機耐受）

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫與審核
- [x] Editing engine implementation complete  
  編輯引擎實作完成
- [x] Automated unit tests implemented  
  已實作自動化單元測試
- [x] Integration tests (split views, multi-instance)  
  已涵蓋整合測試（分割視窗、多執行個體）
- [x] E2E regression harness scripted  
  已建立端到端回歸測試腳本
- [x] `compatibility.md` updated with differences  
  已更新 `compatibility.md` 差異說明
- [x] Documentation user guide updates  
  使用者指南已更新

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性筆記：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/core`, `crates/settings`, `apps/gui-tauri`  
  相關 crate：`crates/core`、`crates/settings`、`apps/gui-tauri`

## Open Questions / 未決議題
- None.  
  無。
