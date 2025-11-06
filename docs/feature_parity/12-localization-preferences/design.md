# Design Draft – Feature 3.12（設計草稿 – 功能 3.12）

RustNotePad 需要在地化、主題管理與偏好設定三大子系統達到 Notepad++ 相容性。本文件描述資料格式、運作流程、相依模組與測試方針，確保 GUI 與 CLI 均能覆蓋主要使用情境。

## Localization System / 在地化系統

### File Format / 檔案格式
- 採用 JSON 語言包，放置於 `assets/langs/<locale>.json`。基本結構：
  ```json
  {
    "locale": "zh-TW",
    "display_name": "繁體中文",
    "strings": {
      "menu.file": "檔案",
      "toolbar.pinned_tabs": {
        "type": "plural",
        "one": "已釘選 1 個分頁",
        "other": "已釘選 {count} 個分頁"
      }
    }
  }
  ```
- `strings` 支援兩種類型：
  - 字串（單數形式）。
  - 複數物件：`{ "type": "plural", "<category>": "<message>" }`，分類鍵符合 CLDR (`zero`, `one`, `two`, `few`, `many`, `other`)。
- 非必要欄位可設為 `null`；未宣告之鍵回退內建英文。

### Plural & Parameter Rules / 複數與參數規則
- 使用 `icu_plurals` 提供的 `PluralRules` 判斷複數類別。
- 允許 `{count}`、`{0}` 兩種 placeholder。`{count}` 對應複數運算參數；`{0}`/`{1}`…對應 GUI 傳入的索引化參數（維持與現有格式一致）。
- `LocalizationManager` 讀取 JSON 後建立 `Message` 枚舉：
  ```rust
  enum Message {
      Simple(String),
      Plural { rules: LocalePluralRules, forms: BTreeMap<PluralCategory, String> },
  }
  ```
- 在 `text_with_args(key, params)` API 中根據 active locale 決定訊息，缺值時落回英文，再回退至 key 本身。

### Runtime Switching / 執行時切換
- `LocalizationManager::set_active_by_code(code)` 允許 GUI 或 CLI 在不中斷的情況下切換語系。
- GUI 透過發送 `LocalizationChanged` 訊息刷新所有快取字串（選單、狀態列等）。
- `UserProfileStore` 存放 `locale=<code>` 以便下次啟動時套用。

### Tooling & Pipelines / 工具與流程
- 新增 `scripts/dev/l10n-compile.rs` 檢查 JSON 結構、確保複數類別齊備（至少提供 `other`）。
- 建立 CI 工作在 PR 中驗證所有 `assets/langs/*.json`。

## Theme Management / 主題管理

### Storage Model / 儲存模型
- 官方主題以 JSON 定義，與 `ThemeDefinition` 結構對應。
- 匯入流程支援三種來源：
  1. Notepad++ `.xml`（`stylers.xml`/`themes/*.xml`）。
  2. TextMate `.tmTheme`。
  3. Sublime `.sublime-color-scheme` 與 `.sublime-syntax`（語法 / 配色）。
- 匯入器將外部格式轉成統一的 `ThemeDefinition`（顏色、字型、語法 palette）。產生的 JSON 儲存於 `assets/themes/<slug>.json`。
- `ThemeManager::load_from_dir` 讀取內建與使用者匯入的主題目錄（GUI 預設 `workspace/themes`，CLI 可指定路徑）。

### Syntax Highlight Integration / 語法高亮整合
- 匯入 `.tmTheme` / `.sublime-syntax` 時，利用現有 `rustnotepad_highlight` 轉換成 `HighlightPalette`。
- 對於不支援的 scope / 樣式，記錄於 `compatibility.md`。

### Theme Editor / 主題編輯器
- GUI 提供可視化編輯 palette、字型與語法顏色。操作完成後輸出 JSON，並在儲存前重新驗證（`ThemeDefinition::validate`）。
- Editor 支援「另存新檔」與「匯出 `.tmTheme`」以維持與 N++ 相容。

## Preferences Persistence / 偏好儲存

### Schema & Storage / 結構與儲存
- 偏好設定儲存於 `<workspace>/.rustnotepad/preferences.json`，採用語意化版本。
  ```json
  {
    "version": 1,
    "editor": {
      "autosave": { "enabled": true, "interval_minutes": 5 },
      "line_numbers": true,
      "highlight_active_line": true,
      "word_wrap": false
    },
    "ui": {
      "locale": "en-US",
      "theme": "Midnight Indigo"
    }
  }
  ```
- `PreferencesStore`（新模組）負責：
  - 讀寫 JSON 並處理遷移：`version` 改變時執行升級函式。
  - 與 `UserProfileStore` 整合，避免重複欄位（例如 locale / theme）。

### Import / Export Flows / 匯入與匯出
- GUI：Preferences 頁新增「匯出設定」、「匯入設定」按鈕。
  - 匯出：選擇儲存路徑，寫出目前偏好 JSON。
  - 匯入：讀取外部 JSON，驗證 `version` 與 schema，套用至 `PreferencesState`，並將結果持久化。
- CLI：新增 `rustnotepad-cli --workspace <path> preferences export/import` 子命令，以便腳本化。

### Defaults & Overrides / 預設值與覆寫
- 預設值由程式碼定義；若偏好檔缺少欄位，載入時補齊。
- 允許 `--override <key>=<value>` 命令列旗標針對單次執行覆寫偏好（例如在 CI 中強制 `theme=Midnight Indigo`）。

## Import/Export Workflows / 匯入與匯出流程

| Workflow | GUI | CLI | Notes |
|----------|-----|-----|-------|
| Theme import | 設定 → Style Configurator → 匯入 | `rustnotepad-cli themes import <file>` | 產生 JSON + 更新 `ThemeManager`. |
| Theme export | 設定 → Style Configurator → 匯出 | `rustnotepad-cli themes export <name> --format tmTheme` | 支援 N++ XML、tmTheme。 |
| Preferences import | 設定 → Preferences → 匯入 | `rustnotepad-cli preferences import <file>` | 驗證 schema，備份舊檔。 |
| Preferences export | 設定 → Preferences → 匯出 | `rustnotepad-cli preferences export --output <file>` | 預設輸出到工作區根目錄。 |
| Language pack install | 設定 → 語言 → 匯入 | `rustnotepad-cli localization install <file>` | 將 JSON 放入 `assets/langs` 或使用者專屬資料夾。 |

所有匯入操作需提供復原點：
- 匯入前備份原檔（`*.bak`）。
- 匯入失敗時復原備份並顯示錯誤。

## Testing Strategy / 測試策略

### Unit Tests
- `crates/settings`：
  - `LocalizationManager`：單元測試涵蓋複數規則、fallback、參數格式化（`crates/settings/tests/localization_tests.rs`）。
  - `ThemeDefinition`：顏色解析、字型尺寸驗證、匯入器轉換。
  - `PreferencesStore`：載入、儲存、遷移、匯入/匯出。
- 使用 snapshot 測試（`insta`）確保語言包載入與主題轉換輸出穩定。

### Integration Tests
- 新增 `tests/localization_runtime.rs`：模擬 GUI 切換語系並檢查 UI 字串。
- `tests/theme_roundtrip.rs`：從 `.tmTheme` 轉換成 JSON，再轉回 `.tmTheme`，驗證顏色一致。
- `tests/preferences_sync.rs`：模擬 GUI 編輯偏好 → 寫入檔案 → CLI 匯出 → 重新載入。

### E2E
- Playwright 自動化腳本：
  - 語言換成 `zh-TW`，確認選單顯示更新。
  - 匯入自訂主題並套用，擷取螢幕快照對比。
  - 匯入偏好檔，驗證 autosave 選項變更。

## Decision Log / 決策紀錄
- 2024-05-09：語言包採 JSON + ICU 複數規則，落實於 `LocalizationManager`。
- 2024-05-09：主題匯入支援 Notepad++ XML / tmTheme / Sublime，統一轉成 `ThemeDefinition`。
- 2024-05-09：偏好儲存以 JSON schema + 版本遷移為主，CLI 與 GUI 共用同一套 API。
