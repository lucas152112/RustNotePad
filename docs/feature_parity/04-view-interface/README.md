# Feature 3.4 – View & Interface（功能 3.4 – 檢視與介面）

## Scope / 範圍
- Tab management, pin/lock, color tagging  
  分頁管理、釘選/鎖定、顏色標記
- Document map, status bar, sidebar panels  
  文件地圖、狀態列、側邊欄面板
- Theme and font settings, UI language switching  
  主題與字型設定、介面語言切換
- Layout persistence, split panes, docking behaviours  
  版面配置持久化、分割窗格、停駐行為

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫與審閱
- [x] UI layout system implementation  
  UI 版面系統實作完成
- [x] Theme/appearance management implemented  
  主題與外觀管理已實作
- [x] Unit tests for layout serialization  
  已完成版面序列化單元測試
- [x] E2E UI regression coverage (`crates/settings/tests/ui_layout_regression.rs`)  
  已透過 `crates/settings/tests/ui_layout_regression.rs` 覆蓋端到端 UI 回歸
- [x] `compatibility.md` updated with differences  
  已更新 `compatibility.md` 差異列表
- [x] Documentation / screenshots updated  
  文件與截圖已更新

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性筆記：`compatibility.md`
- Tests: `tests/`  
  測試：`tests/`
- Related crates: `apps/gui-tauri`, `crates/settings`, `assets/themes`  
  相關 crate：`apps/gui-tauri`、`crates/settings`、`assets/themes`

## Open Questions / 未決議題
- How should interactive docking (drag/drop panes) be modelled once the Tauri shell is wired to real window handles?  
  當 Tauri shell 連接至實際視窗控制時，互動式停駐（拖放窗格）應如何建模？
- Which additional palette entries are required for plugin panels and diff viewers?  
  外掛面板與差異檢視器需要額外哪些調色盤項目？
