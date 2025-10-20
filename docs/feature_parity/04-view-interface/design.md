# Design Draft – Feature 3.4（設計草稿 – 功能 3.4）

## Goals / 目標
- Provide a reusable layout description that captures pane configuration, pin/lock state, and dock visibility across sessions.  
  提供可重複使用的版面描述，涵蓋窗格配置、釘選/鎖定狀態與停駐面板顯示，並可跨工作階段保存。
- Deliver a theme system with palette + font metadata, loadable from disk and reusable by both GUI and future headless tooling.  
  建立含調色盤與字型後設資料的主題系統，可由 GUI 與未來的無頭工具共用並自磁碟載入。
- Elevate the UI preview so that it reflects tab pinning, color tags, split panes, document map, bottom dock, and status bar metrics.  
  強化 UI 預覽，能呈現分頁釘選、顏色標記、分割窗格、文件地圖、底部停駐面板與狀態列指標。
- Expose the theme and locale selectors through the toolbar to mirror Notepad++ customisability.  
  透過工具列提供主題與語系選擇器，對齊 Notepad++ 的自訂能力。

## Architecture Overview / 架構概覽

### Layout management / 版面管理
- Introduced `rustnotepad_settings::layout` with `LayoutConfig`, `PaneLayout`, `TabView`, and `DockLayout`.  
  新增 `rustnotepad_settings::layout` 模組，內含 `LayoutConfig`、`PaneLayout`、`TabView` 與 `DockLayout`。
  - `LayoutConfig` serialises to/from JSON (validated against duplicate IDs and split ratios).  
    `LayoutConfig` 支援 JSON 序列化與還原，並檢查重複 ID 與分割比例是否合法。
  - `PaneLayout` tracks pinned vs regular tabs, active tab ID, and lightweight colour tags (`TabColorTag`).  
    `PaneLayout` 管理釘選/一般分頁、目前活躍分頁 ID，以及輕量顏色標籤（`TabColorTag`）。
  - `DockLayout` records visible bottom panels and the currently focused panel.  
    `DockLayout` 紀錄底部面板的顯示狀態與目前焦點面板。
- `LayoutConfig::default()` seeds the GUI preview with representative files (mirroring the search feature work).  
  `LayoutConfig::default()` 會載入代表性檔案，讓 GUI 預覽與搜尋功能維持一致。
- Helper APIs:  
  輔助 API：
  - `LayoutConfig::set_active_tab` mutates the in-memory layout when the user switches tabs.  
    `LayoutConfig::set_active_tab` 在使用者切換分頁時更新記憶體中的版面資料。
  - `LayoutConfig::pinned_tabs` is consumed by UI/tooling for quick counts.  
    `LayoutConfig::pinned_tabs` 提供 UI/工具快速取得釘選分頁數。
  - `LayoutConfig::validate_split_ratio` guards view split adjustments.  
    `LayoutConfig::validate_split_ratio` 保護視窗分割比例的調整不超出範圍。

### Theme system / 主題系統
- Added `rustnotepad_settings::theme` with `ThemeDefinition`, `ThemeManager`, `ResolvedPalette`, `FontSettings`, and low-level `Color`.  
  新增 `rustnotepad_settings::theme` 模組，包含 `ThemeDefinition`、`ThemeManager`、`ResolvedPalette`、`FontSettings` 與底層 `Color`。
  - Themes are JSON documents stored under `assets/themes/` (see `midnight_indigo.json`, `nordic_daylight.json`).  
    主題為儲存在 `assets/themes/` 下的 JSON 檔案（例如 `midnight_indigo.json`、`nordic_daylight.json`）。
  - `ThemeDefinition::resolve_palette` parses hex colours into RGBA and validates fonts.  
    `ThemeDefinition::resolve_palette` 會將十六進位色碼解析為 RGBA，並檢查字型設定。
  - `ThemeManager::load_from_dir` discovers theme JSON files; falls back to built-in dark/light definitions when no files are found.  
    `ThemeManager::load_from_dir` 掃描主題目錄，若找不到檔案則回退到內建的深色/淺色主題。
  - `ThemeManager` caches resolved palettes for cheap lookups (`active_palette`, `theme_names`, etc.).  
    `ThemeManager` 快取解析後的調色盤，供 `active_palette`、`theme_names` 等查詢快速存取。
- Colour parsing supports `#RRGGBB` and `#RRGGBBAA` forms; tests validate success/failure cases.  
  顏色解析支援 `#RRGGBB` 與 `#RRGGBBAA` 格式，並以測試覆蓋成功/失敗情境。

### GUI integration (eframe preview) / GUI 整合（eframe 預覽）
- `RustNotePadApp` now owns:  
  `RustNotePadApp` 目前負責：
  - `LayoutConfig` (drives tab strips, split panes, bottom dock).  
    `LayoutConfig`（控制分頁列、分割窗格與底部停駐面板）
  - `ThemeManager` & `ResolvedPalette` (applied to `egui::Context` + `Style` when changed).  
    `ThemeManager` 與 `ResolvedPalette`（變更時套用至 `egui::Context` 與 `Style`）
  - Locale list + status bar state.  
    語系清單與狀態列狀態。
- Toolbar enhancements:  
  工具列增強功能：
  - Theme selector (backed by `ThemeManager::set_active_index`).  
    主題選擇器（使用 `ThemeManager::set_active_index`）
  - Locale selector (updates status bar).  
    語系選擇器（更新狀態列顯示）
  - Split ratio slider invoking `LayoutConfig::validate_split_ratio`.  
    分割比例滑桿會呼叫 `LayoutConfig::validate_split_ratio`。
- Tab strip rendering:  
  分頁列渲染：
  - Pinned tabs rendered ahead of regular tabs.  
    釘選分頁顯示在一般分頁之前。
  - Lock state annotated with `[RO]`.  
    以 `[RO]` 標註唯讀狀態。
  - Colour tags painted via `TabColorTag::hex`.  
    透過 `TabColorTag::hex` 呈現顏色標記。
- Split panes:  
  分割窗格：
  - Primary pane hosts the editable buffer.  
    主窗格載入可編輯緩衝。
  - Secondary pane displays read-only preview metadata.  
    次窗格顯示唯讀預覽資訊。
- Bottom dock renders panel tabs + sample content for Find Results, Console, Notifications, and LSP diagnostics; controlled via `DockLayout`.  
  底部停駐面板展示各面板分頁與範例內容（搜尋結果、主控台、通知、LSP 診斷），由 `DockLayout` 控制。
- Status bar aggregates cursor metrics, encoding/EOL, active document language, theme, and UI language.  
  狀態列統整游標資訊、編碼/行尾、目前文件語言、主題與介面語言。

### Document map & project panel / 文件地圖與專案面板
- Document map now streams lines from the live editor buffer.  
  文件地圖會從即時編輯器緩衝逐行擷取資料。
- Project tree reuses static `Lazy<Vec<ProjectNode>>` but reflects layout modules (core/search/docs/tests).  
  專案樹沿用靜態 `Lazy<Vec<ProjectNode>>`，並反映核心、搜尋、文件、測試等模組。

## Open items / 待辦事項
- Wire the layout/theme modules into a persistence layer once Tauri shell is introduced.  
  Tauri shell 引入後，需要將版面與主題模組接入持久化層。
- Add per-tab close buttons and drag/drop simulation in preview.  
  在預覽中加入每個分頁的關閉按鈕與拖放模擬。
- Integrate actual language detection and document statistics when core editor is ready.  
  待核心編輯器完成後，整合語言偵測與文件統計資訊。

## Decision log / 決策紀錄
- Adopted JSON for both layout and theme assets (matches ecosystem expectations and permits CLI tooling).  
  版面與主題資產均採 JSON 格式，符合生態系預期並方便 CLI 工具使用。
- Kept layout/theme logic inside `rustnotepad_settings` so CLI, GUI, and future daemons share a single source of truth.  
  將版面/主題邏輯集中在 `rustnotepad_settings`，讓 CLI、GUI 與未來守護程序共用單一來源。
- eframe preview applies themes lazily (only when selection changes) to avoid redundant style churn.  
  eframe 預覽僅在選擇變更時套用主題，避免重複刷新樣式。
- Theme palette intentionally minimal (background/panel/accent/editor/status) until further UX studies.  
  調色盤目前維持精簡（背景/面板/強調/編輯器/狀態列），待後續 UX 研究再擴充。
