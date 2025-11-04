# Develop Diary / 開發日誌

## 2025-11-04

### 完成 / Completed
- 建立 `rustnotepad_plugin_admin` crate，提供外掛安裝、更新、移除的後端流程（WASM 與 DLL） / Added the `rustnotepad_plugin_admin` crate with install/update/remove backends for both WASM and DLL plugins.
- 在 Windows 版本連結外掛指令按鈕，透過 `WindowsMessage` 轉送 `WM_COMMAND` 以觸發 DLL 回呼 / Wired Windows command buttons to dispatch `WM_COMMAND` via `WindowsMessage`, invoking DLL callbacks.
- 更新 Feature 3.11 文件與測試計畫，記錄 Plugin Admin 後端與 Windows 橋接狀態 / Updated Feature 3.11 docs and test plan to capture the Plugin Admin backend and Windows bridge progress.

### 未完成 / Pending
- 外掛管理 UI 與 CLI 尚未串接新的安裝/更新/移除 API / Plugin Admin UI/CLI still need to integrate the new install/update/remove APIs.
- Windows 訊息轉譯與 Scintilla 互動尚未實作，命令回傳仍為初步版本 / Windows message translation and Scintilla interaction remain unimplemented; command feedback is still minimal.
- 缺少整合與端到端測試，以及使用熱門外掛的實際相容性驗證 / Integration and end-to-end tests, plus validation with popular plugins, are still outstanding.
