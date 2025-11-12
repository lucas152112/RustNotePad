# Develop Diary / 開發日誌

## 2025-11-12

### 完成 / Completed
- 增強 GUI 啟動診斷流程：加入 Wayland/X11 連線健康檢查、在記錄檔與標準輸出同步呈現英文/中文訊息，並於 `instance.lock` 寫入 PID 以自動清除失效鎖定；確保在無法啟動圖形環境時能提供足夠的除錯資訊。 / Strengthened GUI startup diagnostics with Wayland/X11 connectivity probes, bilingual logging, and PID-aware `instance.lock` handling so stale locks are auto-removed, giving clearer guidance whenever the compositor is unavailable.
- 調整 UI 預設版面：僅保留專案樹、主要編輯分頁、預設預覽區與狀態列，並提供新的「底部面板」檢視切換來顯示/隱藏搜尋結果與主控台，滿足 UI Preview 初始畫面的需求。 / Simplified the default UI layout to show only the project tree, editor tabs, preview pane, and status bar, while adding a “Bottom Panels” view toggle for users who want the find results/console dock.
- 補充英文/正體中文雙語註解與字串資源，確保日誌、通知與檢視選單皆提供對應翻譯，且 GUI 仍依照使用者設定語系呈現單一語言。 / Expanded bilingual comments and string resources so logs, notifications, and the View menu include both Traditional Chinese and English context, while the GUI continues to render solely in the selected locale.
- 重新切分底部版面：將底部面板與狀態列收納在單一 `TopBottomPanel`，並確保中央編輯區、側邊欄與狀態列互不遮擋，同時移除暫時性的除錯日誌，保持乾淨的 UI 行為與輸出。 / Refactored the bottom layout so the dock and status bar share one `TopBottomPanel`, ensuring the editor/sidebars never overlap the status bar, and removed temporary debug logging to keep the UI and logs tidy.
- 修正「檢視 → 狀態列」在 zh-TW 語系仍顯示英文的問題：更新 `assets/langs`、內建 fallback 與 CLI 安裝測試，並新增 `zh_tw_locale_translates_status_bar` 回歸測試確保選單即時顯示「狀態列」。 / Fixed the View → Status Bar menu showing English under the zh-TW locale by aligning `assets/langs`, the fallback catalog, and localization installer tests, plus added the `zh_tw_locale_translates_status_bar` regression test so the menu consistently renders 「狀態列」.

### 未完成 / Pending
- 針對其餘歷史註解與文件補齊雙語內容，並擴充自動化測試覆蓋更多 GUI 互動情境。 / Backfill bilingual text across older comments/docs and extend automated coverage for additional GUI interactions.

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
