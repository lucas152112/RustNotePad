# Compatibility Notes – Feature 3.12（相容性備註 – 功能 3.12）

## Known differences / 已知差異
- CLI 主題匯出目前僅輸出 RustNotePad JSON；後續版本將補上 tmTheme/其他格式。  
  Theme exports currently emit the unified JSON package; tmTheme output is planned.
- 語言包安裝採用 `<workspace>/.rustnotepad/langs` 使用者目錄，與 Notepad++ 全域資料夾不同。  
  Localization installs are workspace-scoped (`.rustnotepad/langs`) instead of a global shared folder.

## Validation checklist / 驗證檢查清單
- [x] Localization coverage compared against Notepad++ language packs  
  透過 `cargo run --manifest-path scripts/dev/l10n-compiler/Cargo.toml -- --reference docs/feature_parity/12-localization-preferences/reference/notepadpp_en_reference.json --fail-on-missing` 驗證，參考鍵清單取自 Notepad++ v8.8.6 `english_customizable.xml`。
- [x] Theme import/export compatibility verified  
  主題匯入/匯出的 CLI/GUI 驗證已完成（tmTheme/XML/Sublime 轉換與匯出流程）
- [x] Preference migration between releases tested  
  `crates/settings/tests/preferences_store.rs:40` 覆蓋 legacy `version=0` 偏好檔載入與欄位補強流程。
- [x] RTL and CJK UI layouts validated  
  `rustnotepad_gui/src/main.rs:6346`、`:6366`、`:6400` 的自動化測試涵蓋 zh-TW/CJK 字型流程與使用者自訂 `ar-SA` 語系的 RTL 切換。
