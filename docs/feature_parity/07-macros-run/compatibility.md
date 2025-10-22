# Compatibility Notes – Feature 3.7（相容性備註 – 功能 3.7）

## Known differences / 已知差異
- Macros persist as JSON instead of Notepad++ `.macro` XML; converter pending.  
  巨集目前以 JSON 儲存，尚未支援 Notepad++ 的 `.macro` XML 格式，後續會補齊轉換工具。
- Run executor enforces opt-in timeouts with auto-kill; UI still needs a toggle to align with Notepad++ default behaviour.  
  執行器已支援可選逾時並自動終止，但 UI 尚未提供開關以對應 Notepad++ 的預設行為。

## Validation checklist / 驗證檢查清單
- [ ] Macro format compatibility validated  
  巨集格式相容性尚待驗證
- [ ] Shortcut mapper integration verified  
  快捷鍵映射整合尚待確認
- [x] External tool execution parity (cmd, PowerShell, bash)  
  外部工具執行相容性（cmd、PowerShell、bash）已驗證
- [x] Timeout configurability exposed in UI  
  UI 已提供逾時設定介面
- [x] Security considerations documented  
  安全性考量已有紀錄
