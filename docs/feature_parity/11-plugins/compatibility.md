# Compatibility Notes – Feature 3.11（相容性備註 – 功能 3.11）

## Known differences / 已知差異
- Only manifest discovery is available; plugins are not executed yet.  
  目前僅支援清單掃描，尚未實際執行外掛。
- Windows DLL compatibility layer is limited to filesystem enumeration on non-Windows builds.  
  Windows DLL 相容層在非 Windows 平台僅進行檔案列舉。
- Plugin management UI exposes discovery results and enable toggles but does not yet handle install/update/remove flows.  
  外掛管理介面僅顯示掃描結果與啟用切換，尚未支援安裝、更新或移除流程。
- Windows builds can load DLLs and validate mandatory exports, but message translation and Scintilla bridging are still pending.  
  Windows 版本可載入 DLL 並檢查必要匯出，但訊息轉換與 Scintilla 橋接仍待完成。

## Validation checklist / 驗證檢查清單
- [ ] Windows ABI compatibility tested with top 3 plugins  
  使用前三大外掛驗證 Windows ABI 相容性尚未完成
- [ ] WASM host parity documented  
  WASM 宿主的相容性紀錄尚未完成
- [ ] Security review completed  
  安全性檢視尚未結束
- [ ] Cross-platform plugin install/update validated  
  跨平台外掛安裝/更新尚未驗證
