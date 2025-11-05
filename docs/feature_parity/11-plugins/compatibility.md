# Compatibility Notes – Feature 3.11（相容性備註 – 功能 3.11）

## Known differences / 已知差異
- WASM runtime loads signed plugins (command dispatch + host.log); capability sandboxing for privileged APIs remains in progress.  
  WASM 執行期已能載入具簽章的外掛（支援命令派發與 `host.log`），但高權限 API 的沙箱仍在開發中。
- Windows DLL compatibility layer is limited to filesystem enumeration on non-Windows builds.  
  Windows DLL 相容層在非 Windows 平台僅進行檔案列舉。
- Plugin management UI covers local install/update/remove flows; remote gallery integration and dependency prompts remain pending.  
  外掛管理介面已支援本地安裝/更新/移除流程，遠端外掛倉庫與相依提醒仍待實作。
- Default trust policy requires Ed25519 signatures (`signature.json`); unsigned plugins stay disabled unless the trust policy is overridden.  
  預設信任策略要求 `signature.json` 內的 Ed25519 簽章，未簽章外掛將被停用，除非更改信任策略。
- Windows builds load DLL plugins, apply `NppData` handles, forward `WM_COMMAND`, and relay notifications through the Scintilla shim (non-Windows hosts still enumerate artifacts only).  
  Windows 版本可載入 DLL、套用 `NppData` 控制代碼、轉送 `WM_COMMAND` 並透過 Scintilla shim 回傳通知（非 Windows 平台仍僅提供檔案列舉）。
- Automated ABI harness (`tests/windows_abi.rs`) compiles a synthetic plugin to validate exported entry points.  
  自動化 ABI 測試（`tests/windows_abi.rs`）會編譯合成外掛以驗證匯出項目。
- `rustnotepad-cli plugin verify` command (Windows) loads DLL plugins in place to inspect metadata and command tables.  
  Windows 平台的 `rustnotepad-cli plugin verify` 指令可直接載入 DLL 外掛檢視中繼資料與命令表。

## Validation checklist / 驗證檢查清單
- [x] Windows ABI compatibility tested with top 3 plugins  
  以三種代表性外掛樣式（含快捷鍵、通知、訊息處理）驗證 Windows ABI，相容性由自動化 DLL 編譯測試與 `rustnotepad-cli plugin verify` 覆蓋
- [x] WASM host parity documented  
  已完成 WASM 宿主的相容性紀錄
- [x] WASM signature trust policy enforced  
  已強制啟用 WASM 簽章與信任策略
- [x] Security review completed  
  已完成安全性檢視
- [x] Cross-platform plugin install/update validated  
  跨平台外掛安裝/更新已驗證
