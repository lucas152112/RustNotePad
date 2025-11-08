# Develop Diary / 開發日誌

## 2025-11-05

### 完成 / Completed
- Windows 平台新增 CLI `plugin verify` 指令與自動化 ABI 測試，驗證 DLL 外掛的命令、訊息與通知流程 / Added the Windows `plugin verify` CLI command and automated ABI tests that validate DLL plugin commands, message dispatch, and notifications.
- `RustNotePadApp` 暴露 Windows 控制代碼設定並在載入時套用 `NppData`，使 DLL 命令透過 `WM_COMMAND` 與 Scintilla shim 正常回傳 / `RustNotePadApp` now exposes session handle injection and applies `NppData` during plugin load so DLL commands round-trip through `WM_COMMAND` and the Scintilla shim.
- 完成 WASM 相容性報告與安全性檢視文件，記錄跨平台端到端測試成果與後續風險緩解計畫 / Finalized the WASM parity report and security review, logging the cross-platform E2E runs and outlining follow-up mitigations.

### 未完成 / Pending
- 持續強化 DLL 命令回呼的遙測與異常記錄 / Improve telemetry and logging around DLL command callbacks.
- 建立外掛來源信譽與簽章輪替的自動化流程 / Automate plugin provenance tracking and signer rotation.

## 2025-11-04

### 完成 / Completed
- 完成 Feature 3.12 的 GUI 菜單啟用：檢視/編碼/語言/工具/外掛/視窗/說明皆可呼叫對應指令，並在通知面板顯示結果；同時新增跨平臺打包腳本 `scripts/package-platform-binaries.sh`。 / Finished enabling all Feature 3.12 GUI menus (View/Encoding/Language/Tools/Plugins/Window/Help) with executable commands and notification feedback, plus added the multi-platform packaging script `scripts/package-platform-binaries.sh`.
- 建立 `rustnotepad_plugin_admin` crate，提供外掛安裝、更新、移除的後端流程（WASM 與 DLL） / Added the `rustnotepad_plugin_admin` crate with install/update/remove backends for both WASM and DLL plugins.
- 在 Windows 版本連結外掛指令按鈕，透過 `WindowsMessage` 轉送 `WM_COMMAND` 以觸發 DLL 回呼 / Wired Windows command buttons to dispatch `WM_COMMAND` via `WindowsMessage`, invoking DLL callbacks.
- 完成 Feature 3.12 Localization/Theme/Preferences 初步實作：新增複數規則與參數化字串、TMTheme 匯入流程、偏好設定 JSON 儲存；同步新增對應測試與 GUI 整合。 / Delivered the first Feature 3.12 localization/theme/preferences milestone: plural-aware localization, TextMate theme import, JSON-backed preferences store, and associated tests plus GUI integration.
- 完成 GUI 外掛管理頁的安裝/更新/移除流程並新增 CLI `plugin install/remove` 指令；補齊對應測試 / Enabled install/update/remove flows in the GUI plugin page, added CLI `plugin install/remove` commands, and filled in the accompanying tests.

### 未完成 / Pending
- Windows 訊息轉譯與 Scintilla 互動尚未實作，命令回傳仍為初步版本 / Windows message translation and Scintilla interaction remain unimplemented; command feedback is still minimal.
- 仍需完成熱門外掛的相容性驗證與安全性檢視 / Compatibility runs with popular plugins and a security review remain.
