# Feature 3.7 – Macros & Run（功能 3.7 – 巨集與執行）

## Scope / 範圍
- Macro recording, naming, shortcut assignment, save/load  
  巨集錄製、命名、快捷鍵配置與存取
- Playback with repeat counts and scripting hooks  
  支援重播次數與腳本掛勾的播放功能
- Run menu for external tools (working directory, env vars, I/O piping)  
  外部工具的執行選單（工作目錄、環境變數、I/O 管線）
- Output console integration  
  與輸出主控台整合

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  `design.md` 已撰寫並更新
- [x] Macro recorder and player implemented  
  巨集錄製與播放已實作
- [x] Run/external tool executor implemented  
  執行/外部工具執行器已實作
- [x] Unit tests for macro serialization  
  巨集序列化單元測試已完成
- [x] Integration tests for process execution sandbox  
  進程執行沙箱整合測試已建立
- [ ] E2E coverage for macro/run UI  
  巨集/執行 UI 端到端測試尚未覆蓋
- [x] `compatibility.md` updated  
  `compatibility.md` 已更新

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性筆記：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/macros`, `crates/runexec`, `apps/gui-tauri`  
  相關 crate：`crates/macros`、`crates/runexec`、`apps/gui-tauri`

## Open Questions / 未決議題
- TBD  
  待定
