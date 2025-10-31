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
- [ ] Windows ABI bridge implemented  
  Windows ABI 橋接尚未實作
- [ ] WASM host implemented  
  WASM 宿主尚未實作
- [ ] Plugin admin UI implemented  
  外掛管理 UI 尚未實作
- [ ] Unit/integration/E2E tests in place  
  單元/整合/端到端測試尚未到位
- [ ] `compatibility.md` updated  
  `compatibility.md` 尚待更新
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
- Runtime execution, sandboxing, and management UI remain pending.  
  外掛執行、沙箱與管理介面仍待實作。

## Open Questions / 未決議題
- TBD  
  待定
