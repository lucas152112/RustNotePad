# Compatibility Notes – Feature 3.11（相容性備註 – 功能 3.11）

## Known differences / 已知差異
- WASM runtime loads signed plugins (command dispatch + host.log); capability sandboxing for privileged APIs remains in progress.  
  WASM 執行期已能載入具簽章的外掛（支援命令派發與 `host.log`），但高權限 API 的沙箱仍在開發中。
- Windows DLL compatibility layer is limited to filesystem enumeration on non-Windows builds.  
  Windows DLL 相容層在非 Windows 平台僅進行檔案列舉。
- Plugin management UI exposes discovery results and enable toggles but does not yet handle install/update/remove flows.  
  外掛管理介面僅顯示掃描結果與啟用切換，尚未支援安裝、更新或移除流程。
- Default trust policy requires Ed25519 signatures (`signature.json`); unsigned plugins stay disabled unless the trust policy is overridden.  
  預設信任策略要求 `signature.json` 內的 Ed25519 簽章，未簽章外掛將被停用，除非更改信任策略。
- Windows builds load DLL plugins and expose command metadata, yet command execution (message translation / Scintilla bridge) remains unimplemented.  
  Windows 版本可載入 DLL 並顯示命令資訊，但命令執行（訊息轉譯 / Scintilla 橋接）仍待實作。

## Validation checklist / 驗證檢查清單
- [ ] Windows ABI compatibility tested with top 3 plugins  
  使用前三大外掛驗證 Windows ABI 相容性尚未完成
- [ ] WASM host parity documented  
  WASM 宿主的相容性紀錄尚未完成
- [x] WASM signature trust policy enforced  
  已強制啟用 WASM 簽章與信任策略
- [ ] Security review completed  
  安全性檢視尚未結束
- [ ] Cross-platform plugin install/update validated  
  跨平台外掛安裝/更新尚未驗證
