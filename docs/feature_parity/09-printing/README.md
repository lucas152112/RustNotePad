# Feature 3.9 – Printing & Preview（功能 3.9 – 列印與預覽）

## Scope / 範圍
- Syntax-coloured printing with headers/footers, pagination  
  支援語法著色的列印，包含頁首/頁尾與分頁
- Print preview with zoom, paper size, margin settings  
  列印預覽提供縮放、紙張大小與邊界設定
- Printer configuration persistence  
  列印機組態持久化

## Status Checklist / 進度檢查清單
- [ ] `design.md` drafted and reviewed  
  尚未完成 `design.md` 撰寫與審閱
- [ ] Printing pipeline implemented  
  列印管線尚未實作
- [ ] Preview UI implemented  
  預覽 UI 尚未實作
- [ ] Unit tests for pagination engine  
  分頁引擎的單元測試尚未完成
- [ ] Integration tests against PDF output  
  PDF 輸出的整合測試尚未完成
- [ ] E2E validation with real printers (platform matrix)  
  實體印表機的端到端驗證尚未執行（跨平台）
- [ ] `compatibility.md` updated  
  `compatibility.md` 尚待更新

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性備註：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/printing`, `apps/gui-tauri`  
  相關 crate：`crates/printing`、`apps/gui-tauri`

## Open Questions / 未決議題
- TBD  
  待定
