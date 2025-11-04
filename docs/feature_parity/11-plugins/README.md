# Feature 3.11 – Plugin System（功能 3.11 – 外掛系統）

## Scope / 範圍
- Windows Notepad++ plugin ABI compatibility layer for DLL plugins  
  Windows 平台提供 Notepad++ 外掛 ABI 相容層（DLL 外掛）
- Cross-platform WASM plugin host with permission sandbox  
  跨平台 WASM 外掛宿主與權限沙箱
- Plugin management UI (install/update/disable/remove, dependency checks)  
  外掛管理介面（安裝/更新/停用/移除、相依檢查）
- Plugin signature verification and trust model  
  外掛簽章驗證與信任模型

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫並初審
- [x] Windows ABI bridge implemented  
  已完成 Windows ABI 橋接
- [x] WASM host implemented  
  已完成 WASM 宿主實作
- [ ] Plugin admin UI implemented  
  外掛管理 UI 尚未實作
- [ ] Unit/integration/E2E tests in place  
  單元/整合/端到端測試尚未到位
- [x] `compatibility.md` updated  
  `compatibility.md` 已更新
- [ ] Documentation for plugin authors  
  外掛開發者文件尚未完成

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性備註：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/plugin_winabi`, `crates/plugin_wasm`, `apps/gui-tauri`  
  相關 crate：`crates/plugin_winabi`、`crates/plugin_wasm`、`apps/gui-tauri`

## Progress notes / 進度說明
- Added discovery crates for WASM and Windows plugins; GUI logs loaded plugins and honours `-noPlugin`.  
  新增 WASM 與 Windows 外掛掃描 crate；GUI 會記錄外掛並支援 `-noPlugin` 停用。
- Capability policy defaults to read-only/editor-safe operations; denying elevated permissions surfaces warnings.  
  能力政策預設僅允許讀取與編輯安全操作，拒絕進階權限會顯示警示。
- Settings window now lists discovered plugins with enable/disable toggles, persisting the `-noPlugin` state.  
  設定視窗現可列出已偵測的外掛並提供啟用/停用切換，並沿用 `-noPlugin` 的停用狀態。
- WASM runtime crate (`rustnotepad_plugin_host`) instantiates enabled plugins, exposes command execution, and streams plugin logs back into the GUI.  
  WASM 執行期 crate（`rustnotepad_plugin_host`）會實例化啟用的外掛、提供命令執行，並將外掛輸出回傳至 GUI。
- Trust policy enforces Ed25519 signatures (`signature.json`), ships with default signer, and disables unsigned plugins unless users opt in.  
  信任策略要求 `signature.json` 內的 Ed25519 簽章，提供預設簽署者，未簽章外掛預設停用並需使用者另行允許。
- Windows bridge loads DLL plugins, surfaces command metadata in the GUI, and flags load failures (non-Unicode, missing exports) for review.  
  Windows 橋接可載入 DLL 外掛，在 GUI 呈現命令資訊，並針對非 Unicode 或缺匯出等錯誤顯示警示。
- Windows message scaffolding (`WindowsMessage`, `dispatch_message`) is in place to wire future command execution.  
  已建立 `WindowsMessage` / `dispatch_message` 訊息骨架，為後續命令執行鋪路。
- Added `rustnotepad_plugin_admin` crate to script plugin install/update/remove operations (UI wiring pending).  
  新增 `rustnotepad_plugin_admin` crate 提供外掛安裝/更新/移除腳本（GUI 串接尚未完成）。
- Windows message translation/Scintilla shims and Plugin Admin UI remain pending.  
  Windows 訊息轉譯 / Scintilla 介面與外掛管理 UI 串接仍待完成。

## Open Questions / 未決議題
- TBD  
  待定
