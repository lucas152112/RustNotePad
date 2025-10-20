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
- [ ] `design.md` drafted and reviewed  
  尚未完成 `design.md` 撰寫與審閱
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

## Open Questions / 未決議題
- TBD  
  待定
