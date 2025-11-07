# Feature 3.12 – Localization, Themes & Preferences（功能 3.12 – 在地化、主題與偏好）

## Scope / 範圍
- Localization system (plural rules, string catalogs)  
  在地化系統（複數規則、字串目錄）
- Theme management and editor (import `.xml`, `.tmTheme`, `.sublime-syntax` mappings)  
  主題管理與編輯器（可匯入 `.xml`、`.tmTheme`、`.sublime-syntax` 映射）
- Preference UI with import/export  
  偏好設定 UI，支援匯入/匯出

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫與審閱
- [x] Localization pipeline implemented  
  在地化流程已實作（含 JSON 驗證與 CLI 匯入）
- [x] Theme manager implemented  
  主題管理器已加入多來源載入與跨格式匯入
- [x] Preference storage implemented  
  偏好儲存支援匯入/匯出與版本遷移
- [x] Unit/integration/E2E tests in place  
  已補齊單元、整合與 CLI 端到端測試
- [x] `compatibility.md` updated  
  `compatibility.md` 已更新
- [x] Documentation updates for localization/theme creation  
  已更新在地化與主題製作文件

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性備註：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/settings`, `assets/langs`, `assets/themes`  
  相關 crate：`crates/settings`、`assets/langs`、`assets/themes`

## Open Questions / 未決議題
- TBD  
  待定
