# Test Plan – Feature 3.12（測試計畫 – 功能 3.12）

## Unit tests / 單元測試
- Localization loader/parsing with plural rules  
  含複數規則的在地化載入/解析
- Theme conversion utilities  
  主題轉換工具
- Preference schema validation  
  偏好結構驗證

## Integration tests / 整合測試
- Runtime locale switch with live UI  
  即時 UI 語系切換
- Theme import/export cross-check  
  主題匯入/匯出交叉驗證
- Preference sync between GUI and config files  
  GUI 與設定檔間的偏好同步
- CLI smoke tests (`apps/cli/tests/settings.rs`) for localization/themes/preferences flows  
  CLI 在地化／主題／偏好情境測試（`apps/cli/tests/settings.rs`）

## E2E scenarios / 端到端情境
- Language installer workflow  
  語言套件安裝流程
- Theme editor full cycle (create/edit/share)  
  主題編輯器全流程（建立/編修/分享）
- Preference import/export via UI  
  透過 UI 進行偏好匯入/匯出

## Tooling / 測試工具
- `cargo test --package settings`  
  `cargo test --package settings`
- Localization snapshot tests  
  在地化快照測試
- GUI automation for preference dialogs  
  偏好對話框的 GUI 自動化
- `cargo run --manifest-path scripts/dev/l10n-compiler/Cargo.toml -- --reference docs/feature_parity/12-localization-preferences/reference/notepadpp_en_reference.json --fail-on-missing`  
  使用 Notepad++ 參考鍵清單驗證在地化覆蓋率
