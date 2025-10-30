# Compatibility Notes – Feature 3.10（相容性備註 – 功能 3.10）

## Known differences / 已知差異
- File locking uses a `.rustnotepad/instance.lock` sentinel created via `create_new`; crashes may leave the file behind until the next launch removes it.  
  使用 `create_new` 於 `.rustnotepad/instance.lock` 建立鎖定檔，如果異常終止可能遺留檔案，待下次啟動時才會清除。
- `--theme` treats a file argument as a theme name stem; external JSON themes are not parsed dynamically yet.  
  `--theme` 的檔案參數會以檔名（不含副檔名）對應現有主題，尚未支援即時解析外部 JSON 主題。
- `-noPlugin` and other plugin-related switches are accepted and logged but no plugin host is wired yet.  
  `-noPlugin` 等外掛旗標目前僅接受並記錄，尚未實作外掛載入/停用邏輯。
- No `--help` banner is emitted; users should consult `docs/feature_parity/10-command-line/README.md` for usage.  
  尚未提供 `--help` 自動說明，請參考 `docs/feature_parity/10-command-line/README.md`。

## Validation checklist / 驗證檢查清單
- [ ] Legacy flag parity validated  
  傳統參數相容性尚未驗證
- [ ] Extended options documented with examples  
  擴充選項與範例尚未撰寫
- [ ] Behaviour verified across Windows/macOS/Linux shells  
  各平臺 shell 行為尚未比對
- [ ] Error messaging aligned with Notepad++ expectations  
  錯誤訊息是否符合 Notepad++ 預期尚待確認
