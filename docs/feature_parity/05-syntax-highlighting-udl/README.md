# Feature 3.5 – Syntax Highlighting & UDL（功能 3.5 – 語法高亮與 UDL）

## Scope / 範圍
- Built-in language grammars, folding rules, and theme integration  
  內建語言文法、摺疊規則與主題整合
- User Defined Language (UDL) editor, import/export, sharing workflow  
  使用者自訂語言（UDL）編輯器、匯入/匯出與分享流程
- Highlight style editor with preview and theme bindings  
  高亮樣式編輯器，包含預覽與主題綁定

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫與審核
- [x] Tree-sitter (or alternative) integration complete  
  Tree-sitter（或替代方案）整合完成
- [x] UDL schema and persistence implemented  
  已實作 UDL 結構與永續化
- [x] Unit tests for grammar loaders  
  已完成文法載入單元測試
- [x] Integration tests for UDL import/export  
  已完成 UDL 匯入/匯出整合測試
- [ ] E2E highlighting regression coverage  
  尚未完成高亮回歸的端到端測試覆蓋
- [x] `compatibility.md` updated  
  已更新 `compatibility.md`

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計紀錄：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性備註：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/highlight`, `assets/themes`  
  相關 crate：`crates/highlight`、`assets/themes`

## Open Questions / 未決議題
- TBD  
  待定
