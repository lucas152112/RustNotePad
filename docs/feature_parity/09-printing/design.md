# Design Draft – Feature 3.9（設計草稿 – 功能 3.9）

## 1. Rendering Pipeline & Document Flow / 渲染流程與文件管線
- **Vector-first pipeline**: layout is produced as a device-independent display list (our `PrintDisplayList`) that maps glyph runs, line boxes, decorations, and background spans.  
  **向量優先管線**：排版結果輸出為裝置無關的顯示清單（`PrintDisplayList`），記錄字形區段、行框、底紋與背景區塊。
- **Raster fallback**: if a platform target cannot consume vector instructions (e.g. legacy Windows GDI printers), we rasterize per page at 600 DPI using Skia via `rust-skia`.  
  **點陣備援**：若平台目標無法處理向量指令（如舊版 Windows GDI 印表機），以 Skia（`rust-skia` 綁定）將每頁以 600 DPI 點陣化。
- Layout happens in three phases: text shaping (reuse `rustnotepad_highlight::shaper`), visual line segmentation (folding, wrap, end-of-line markers), and printable box generation (margins, header/footer gutters, page breaks).  
  排版分為三階段：文字塑形（重用 `rustnotepad_highlight::shaper`）、視覺行切分（摺疊、換行、行尾記號）以及可列印方塊產生（邊界、頁首/頁尾、分頁）。
- `PrintJobController` owns the pipeline state machine (`Idle → Layout → Render → Spool/Cached`) and surfaces progress callbacks for UI progress bars.  
  `PrintJobController` 掌管管線狀態機（`Idle → Layout → Render → Spool/Cached`），同時提供進度回呼以更新 UI 進度列。

## 2. Syntax Highlighting & Styling Reuse / 語法著色與樣式沿用
- `PrintTheme` translates GUI theme tokens into CMYK-friendly values (spot colors + fallback grayscale).  
  `PrintTheme` 會將 GUI 主題色轉換為 CMYK 友善的色票（含專色與灰階備援）。
- Highlighting is resolved through `rustnotepad_highlight::snapshot(theme, buffer)` producing a tree of spans; printing layer consumes spans but recalculates background overlays so that dark themes remain legible on white paper.  
  高亮結果透過 `rustnotepad_highlight::snapshot(theme, buffer)` 取得範圍樹；列印層使用這些範圍並重新計算背景覆蓋，使深色主題在白紙上仍具可讀性。
- Provide `PrintStyleResolver` trait; GUI passes the currently active theme; headless CLI print commands can load a dedicated “Print” preset.  
  定義 `PrintStyleResolver` trait，GUI 傳入目前主題；CLI 無頭列印可載入專用「Print」樣式組。

## 3. Pagination Engine / 分頁引擎
- `Paginator` computes line heights based on font metrics, tab expansion, and wrap width derived from printable area (`paper_width - margins`).  
  `Paginator` 依字型度量、Tab 展開與列印區域寬度（`紙寬 - 邊界`）計算行高。
- Supports fixed-line-height and variable-line-height (for mixed fonts or zoomed preview); output is `Vec<PageLayout>` with `PageLayout::glyph_runs`, `header`, `footer`, `page_number`.  
  支援固定行高與變動行高（混用字型或預覽縮放）；輸出 `Vec<PageLayout>`，包含 `glyph_runs`、`header`、`footer`、`page_number`。
- Pagination logic is unit-tested against golden fixtures stored under `docs/feature_parity/09-printing/tests/pagination/*.ron`.  
  分頁邏輯以 `docs/feature_parity/09-printing/tests/pagination/*.ron` 的黃金檔進行單元測試。

## 4. Header/Footer Templating / 頁首頁尾模板
- Notepad++-compatible tokens: `&l/&c/&r` (alignment switches), `&f` (file name), `&F` (full path), `&p` (current page), `&P` (total pages), `&d` (localized date), `&t` (localized time), `&o` (active encoding), `&&` (escape).  
  與 Notepad++ 相容的語法：`&l/&c/&r`（對齊切換）、`&f`（檔名）、`&F`（完整路徑）、`&p`（當前頁）、`&P`（總頁）、`&d`（本地化日期）、`&t`（本地化時間）、`&o`（編碼）、`&&`（跳脫）。
- Parser produces `HeaderFooterTemplate { left, center, right }`, each a vector of `TemplateSegment::Literal` / `::Token`. Rendering receives `HeaderFooterContext` with lazily-resolved values (date/time cached per job).  
  解析器輸出 `HeaderFooterTemplate { left, center, right }`，內含 `TemplateSegment::Literal` 與 `::Token`；渲染時以 `HeaderFooterContext` 傳入延遲求值結果（日期/時間於作業層快取）。
- Template engine lives in `rustnotepad_printing::template`; reused by GUI preview controls to live-update when the user edits template fields.  
  範本引擎位於 `rustnotepad_printing::template`，GUI 預覽控制可共用此邏輯，讓使用者編修模板時即時更新。

## 5. Preview Architecture / 預覽架構
- Preview pane uses the same display list, rendered into multi-resolution caches (`96, 144, 192 DPI`) stored as WebP inside an LRU keyed by `PrintPreviewKey { job_id, page, zoom }`.  
  預覽窗格重用顯示清單，渲染為多解析度快取（`96, 144, 192 DPI`），以 WebP 格式存入 LRU，索引鍵為 `PrintPreviewKey { job_id, page, zoom }`。
- Zooming multiplies a DPI factor; when zoom exceeds cached range, renderer kicks off background rasterization and streams progressive updates to the UI.  
  縮放以 DPI 因子調整；超出快取範圍時觸發背景點陣化並漸進式更新 UI。
- Margin adjustments invalidates only the affected page subset thanks to diffing the printable box geometry.  
  邊界調整僅使受影響頁面的幾何重新計算，透過幾何差分避免全量重算。

## 6. Cross-Platform Integration / 跨平台整合
- **Windows**: use `windows-rs` to talk to `PrintDocumentPackageTarget`. Vector paths map to Direct2D; raster fallback uses `ID2D1RenderTarget::CreateBitmap`.  
  **Windows**：透過 `windows-rs` 操作 `PrintDocumentPackageTarget`，向量路徑使用 Direct2D；點陣備援採 `ID2D1RenderTarget::CreateBitmap`。
- **macOS**: integrate with `NSPrintOperation`/`NSPrintInfo`. Display list is bridged to Core Graphics contexts; color managed via `CGColorSpaceCreateDeviceCMYK`.  
  **macOS**：整合 `NSPrintOperation` 與 `NSPrintInfo`，顯示清單轉換為 Core Graphics 指令，色彩由 `CGColorSpaceCreateDeviceCMYK` 管理。
- **Linux**: GTK `PrintOperation` backend with Cairo contexts. Provide feature flag `printing-gtk` to gate GTK dependencies for headless builds.  
  **Linux**：GTK 的 `PrintOperation`（Cairo）後端；以 `printing-gtk` feature 開關 GTK 相依，保持無頭建置可行。
- A shared `PlatformAdapter` trait abstracts job submission, progress, and error mapping. Tests rely on fake adapters hosted in `printing/tests/platform_mock.rs`.  
  共同的 `PlatformAdapter` trait 抽象送件、進度與錯誤對應；測試透過 `printing/tests/platform_mock.rs` 的假介面實現。

## 7. Configuration Persistence / 組態持久化
- `PrintProfile` stored within `rustnotepad_settings::profiles`, keyed by printer name + media type.  
  `PrintProfile` 儲存在 `rustnotepad_settings::profiles`，以印表機名稱與紙材類型為鍵。
- Profiles capture paper size, margins, orientation, color/grayscale, duplex, header/footer templates.  
  配置內容包含紙張大小、邊界、方向、彩色/灰階、雙面列印、頁首/頁尾模板。
- `PrintJobOptions` merges profile defaults with per-invocation overrides; persisted alongside recent target list for quick selection.  
  `PrintJobOptions` 將設定檔預設值與此次呼叫的覆寫合併，同時儲存最近使用目標以利快速選取。

## 8. Implementation Roadmap / 實作路線
- **M1**: Deliver `rustnotepad_printing` crate scaffolding (templates, pagination structs, job controller skeleton) + unit tests for templating.  
  **M1**：完成 `rustnotepad_printing` crate 骨架（模板、分頁結構、作業控制器框架）與模板單元測試。
- **M2**: Implement display list builder using highlight spans and fonts pipeline.  
  **M2**：實作顯示清單建構器，串接高亮範圍與字型流程。
- **M2.5**: Provide mock platform adapter + RON snapshot harness for iterative testing.  
  **M2.5**：建立模擬平台介面與 RON 快照工具，以加速測試迭代。
- **M3**: Integrate platform adapters (Windows/macOS/Linux) with feature flags, provide mock adapter tests.  
  **M3**：整合各平台介面與 feature flag，並提供假介面測試。
- **M4**: Hook GUI preview (Tauri/egui panel) and CLI `--print` command; add snapshot comparison tests (PDF).  
  **M4**：串接 GUI 預覽（Tauri/egui 面板）與 CLI `--print` 指令，新增 PDF 快照測試。
- **M5**: Finalize persistence, E2E flows, high DPI validation and update `compatibility.md`.  
  **M5**：完成設定持久化、端到端流程、高 DPI 驗證並更新 `compatibility.md`。

## 9. Decision Log / 決策紀錄
- Chose vector-first approach with raster fallback to balance crisp output and broad printer compatibility.  
  選擇向量優先、點陣備援的方式，以兼顧印刷銳利度與廣泛印表機相容性。
- Reuse existing highlight spans instead of re-parsing, reducing duplication and guaranteeing theme parity.  
  重用現有高亮範圍，避免重複解析並確保主題一致。
- Adopt Notepad++ header/footer tokens for user familiarity; extend with encoding token `&o`.  
  採用 Notepad++ 樣式的頁首/頁尾代碼以維持使用者習慣，並補充 `&o` 編碼代碼。
- Cache preview bitmaps in WebP to minimize IPC payload between Rust backend and Tauri frontend.  
  預覽快取採 WebP，減少 Rust 後端與 Tauri 前端間的 IPC 負載。
