# Develop Diary / 開發日誌

## 2025-11-29

### 完成 / Completed
- 修正預覽視窗（Document Map）內容顯示：將空白行與非空白字元的顯示邏輯調整為以 `.` 代表非空白字元，保留空白字元，解決了原先顯示 `…` 或空白的問題，提供更清晰的縮圖預覽效果。 / Fixed the preview window (Document Map) content display: adjusted the logic to render non-whitespace characters as `.` and preserve whitespace, replacing the previous `…` or empty output for a clearer thumbnail preview.
- 重構主視窗佈局：將文件預覽視窗與專案面板改為獨立的 `SidePanel`（分別位於右側與左側），編輯視窗維持在 `CentralPanel`，確保三個視窗在視覺與邏輯上完全獨立，不再發生重疊或佈局錯亂。 / Refactored the main window layout: moved the document preview and project panel into independent `SidePanel`s (right and left respectively), keeping the editor in the `CentralPanel`, ensuring the three panes are visually and logically distinct without overlap or layout issues.
- 優化狀態列顯示層級：調整 `App::update` 中的渲染順序，將 `show_status_bar` 移至 `show_editor_area` 之前呼叫，確保狀態列優先佔據底部空間，徹底解決被左右面板或編輯視窗遮蓋的問題，實現狀態列的獨立與置底顯示。 / Optimized status bar layering: adjusted the render order in `App::update` to call `show_status_bar` before `show_editor_area`, ensuring the status bar claims the bottom space first, completely resolving coverage issues by side panels or the editor, achieving a truly independent and docked status bar.
- 實作圖示工具列：新增「新增、開啟、儲存、復原、重做、剪下、複製、貼上、搜尋、執行、設定」等常用功能圖示，並使用 FontAwesome 字型呈現現代化外觀。 / Implemented Icon Toolbar: Added icons for common actions (New, Open, Save, Undo, Redo, Cut, Copy, Paste, Find, Run, Settings) using FontAwesome for a modern look.
- 實作工作階段持久化：改進 `persist_session` 與 `restore_session_snapshot` 邏輯，現在能正確儲存並還原所有開啟的分頁，而不僅僅是當前分頁。 / Implemented Session Persistence: Improved `persist_session` and `restore_session_snapshot` logic to correctly save and restore all open tabs, not just the active one.
- 現代化 UI 設計：
    - 移除編輯器邊框，改採全寬度無邊框設計。 / Modernized UI: Removed editor borders for a full-width, borderless design.
    - 啟用自訂視窗裝飾，移除作業系統標題列，改用自訂的最小化、最大化/還原、關閉按鈕。 / Enabled custom window decorations, removing the OS title bar in favor of custom Minimize, Maximize/Restore, and Close buttons.
    - 統一視窗圓角設計（上方與下方皆為圓角）。 / Unified rounded corner design (both top and bottom).
    - 為設定與說明視窗加入模態遮罩（Modal Overlay），聚焦使用者注意力。 / Added modal overlays for Settings and Help windows to focus user attention.
    - 調整狀態列文字間距，避免貼邊。 / Adjusted status bar text padding to prevent edge crowding.
- 修正編輯器行距：透過自訂 `layouter` 實作了 `egui::TextEdit` 的行距控制，提供更舒適的閱讀體驗。 / Fixed editor line spacing: Implemented line spacing control for `egui::TextEdit` via a custom `layouter` for a more comfortable reading experience.

## 2025-11-25

### 完成 / Completed
- 將狀態列獨立為固定底部面板，與底部停靠區分離並在主視圖佈局前先保留高度，避免檔案清單、文件視窗或其他面板遮蓋狀態列，確保主 panel 的所有操作只在其範圍內呈現。 / Split the status bar into its own fixed bottom panel and lay it out before the main areas so file lists, document views, or bottom docks can no longer overlap it; the main panel now always renders within its own bounds.
- 文件地圖面板改名為「視圖」，並新增關閉按鈕讓使用者能快速收合右側視圖，避免干擾主要編輯區。 / Renamed the Document Map panel to “View” and added a close button so users can quickly dismiss the right-side view without affecting the main editor.
- 調整底部佈局呈現順序，確保主編輯區在開啟/編輯文件時不會再蓋住狀態列，狀態列永遠保留在最下方。 / Reordered the bottom layout so the editor/content can no longer cover the status bar when opening or editing documents, keeping the bar pinned to the bottom at all times.
- 在中央主面板加入狀態列高度的內邊距，雙重保障編輯器與文件預覽不會越界覆蓋底部狀態列。 / Added bottom padding equal to the status bar height in the central panel to guarantee editors and previews cannot overflow onto the status bar.
- 再次拆分底部佈局：狀態列獨立固定在最底，底部停靠區僅在顯示時占用其上方空間，並將編輯器佈局限制在剩餘主面板內，避免任何視窗覆蓋狀態列。 / Re-split the bottom layout so the status bar is fixed at the very bottom, the bottom dock only consumes space above it when visible, and the editor is constrained to the remaining central area to prevent overlap.
- **【最終修復】調整 `App::update()` 中 UI 組件渲染順序，將 `show_status_bar()` 移至 `show_editor_area()` 之後，完全解決編輯區覆蓋狀態列的問題；此修改符合 egui 框架最佳實踐，確保 `TopBottomPanel::bottom` 在 `CentralPanel` 之後渲染以保持最上層顯示。建立完整的修復文檔、自動化驗證腳本、手動測試指南與技術細節說明於 `changes/20251125_status_bar_layout_fix/` 目錄，包含：README.md（主要說明）、apply_fix.sh（一鍵應用）、verify_fix.sh（自動驗證）、run_all_tests.sh（完整測試套件）、manual_test.md（10 項測試場景）、fix.patch（Git 補丁）、TECHNICAL_DETAILS.md（egui 原理解析）、INDEX.md（文件索引）及 CHANGELOG.md（版本記錄）。所有 18 個單元測試通過，編譯成功。** / **[Final Fix]** Adjusted UI component render order in `App::update()`, moving `show_status_bar()` after `show_editor_area()` to completely resolve the editor-covering-status-bar issue; this change follows egui framework best practices, ensuring `TopBottomPanel::bottom` renders after `CentralPanel` to stay on top. Created comprehensive documentation in `changes/20251125_status_bar_layout_fix/` including: README.md (main docs), apply_fix.sh (one-click apply), verify_fix.sh (auto verification), run_all_tests.sh (full test suite), manual_test.md (10 test scenarios), fix.patch (Git patch), TECHNICAL_DETAILS.md (egui principles), INDEX.md (file index), and CHANGELOG.md (version history). All 18 unit tests passed, compilation successful.

### 未完成 / Pending
- 視圖（文件地圖）面板仍無法與編輯窗高度一致，且預期的「…」內容預覽未恢復；需調整共享 frame 與渲染順序以確保右側視窗與主編輯區同步縮放並顯示簡化內容。 / The View (document map) panel still doesn’t match the editor’s height and the expected “…” content preview is missing; need to revisit the shared frame and render sequencing so the right-side panel scales with the editor and shows the simplified preview.

## 2025-11-18

### 完成 / Completed
- 調整狀態列資料模型與佈局：新增字元/行數統計與游標位置同步邏輯，並依 Notepad++ 排列顯示「語言、長度、行列、選取、EOL、編碼、INS」，同時補齊中英文字串與測試，讓 UI 與參考截圖一致。 / Rebuilt the status bar state/layout with live char/line metrics and caret tracking so it now mirrors Notepad++ (language, length, Ln/Col/Sel, EOL, encoding, INS), plus refreshed the en/zh localization strings and tests to keep the UI aligned with the reference shot.
- 新增 Notepad++ 風格佈景 `Notepad++ Classic` 並調整 egui 外觀（工具列/選單/狀態列框線、圓角與選取色），讓預設 UI 更貼近參考截圖；設定預設主題與偏好預設值，並修正相依測試。 / Added the Notepad++ Classic theme and retuned egui chrome (toolbar/menu/status bar framing, rounding, selection colors) to align with the reference shots; set it as the default theme with updated prefs and tests.
- 導入 FontAwesome 圖示字型並將工具列、標籤列關閉鍵、專案面板快捷等改為圖示化顯示；在字型缺失時自動回退避免 panic。 / Integrated FontAwesome icon font and switched toolbar/tree/tabs close controls to icons, with graceful fallback when the font is unavailable.
- 語法選單文案改為「程式語言／語法」，選項列出程式語言與文件格式（Auto, Plain Text, C/C++, Python, Go, Rust, JSON, Markdown）並更新狀態列標籤。 / Renamed the Language menu to “Language / Syntax,” listing real languages/formats (Auto, Plain Text, C/C++, Python, Go, Rust, JSON, Markdown) and updated the status bar label accordingly.

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
