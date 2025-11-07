# Compatibility Notes – Feature 3.12（相容性備註 – 功能 3.12）

## Known differences / 已知差異
- CLI 主題匯出目前僅輸出 RustNotePad JSON；後續版本將補上 tmTheme/其他格式。  
  Theme exports currently emit the unified JSON package; tmTheme output is planned.
- 語言包安裝採用 `<workspace>/.rustnotepad/langs` 使用者目錄，與 Notepad++ 全域資料夾不同。  
  Localization installs are workspace-scoped (`.rustnotepad/langs`) instead of a global shared folder.

## Validation checklist / 驗證檢查清單
- [ ] Localization coverage compared against Notepad++ language packs  
  與 Notepad++ 語言包的覆蓋率比較尚未完成
- [x] Theme import/export compatibility verified  
  主題匯入/匯出的 CLI/GUI 驗證已完成（tmTheme/XML/Sublime 轉換與匯出流程）
- [ ] Preference migration between releases tested  
  版本間偏好設定遷移尚未測試
- [ ] RTL and CJK UI layouts validated  
  RTL 與 CJK 介面配置尚未驗證
