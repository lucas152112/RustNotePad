# Design Draft – Feature 3.5（設計草稿 – 功能 3.5）

## Overview / 概述
Syntax highlighting is delivered by a lightweight rule engine that consumes language definitions generated from either built-in metadata or imported User Defined Language (UDL) files.  
語法高亮由輕量級的規則引擎負責，該引擎會讀取內建語言定義或匯入的使用者自訂語言（UDL）檔案產生的定義。  
The engine is intentionally modular so that we can plug in an incremental tree-sitter backend later without rewriting callers.  
此引擎刻意維持模組化，以便後續可以接入增量式的 tree-sitter 後端而不需重寫呼叫端。

## Parser / Grammar Strategy（解析器 / 語法策略）
- **Phase 1 (current)**: Regex-based tokenizer with explicit precedence order (comments → strings → keywords → numbers → operators → custom rules). This provides parity with Notepad++ UDL behaviour and keeps the implementation small enough to ship now.  
  **第一階段（目前）**：使用具明確優先順序的正規表示式 token 分割器（註解 → 字串 → 關鍵字 → 數字 → 運算子 → 自訂規則），確保行為與 Notepad++ UDL 對齊，並保持實作精簡可立即出貨。
- **Phase 2 (planned)**: Allow a language definition to wrap a tree-sitter grammar. The registry API already abstracts over the implementation so a future `LanguageBackend` trait can sit behind `LanguageDefinition`.  
  **第二階段（規劃）**：讓語言定義可包裝 tree-sitter 語法。註冊中心 API 已抽象化實作，因此未來可在 `LanguageDefinition` 背後加入 `LanguageBackend` trait。
- Large documents are processed in a single pass; viewport caching is postponed until the tree-sitter integration lands because the regex backend is fast enough for initial targets (<1MB files).  
  大型文件目前以單次掃描完成；在 tree-sitter 整合完成前暫不實作視窗快取，因為正規表示式後端對於初期目標（<1MB 檔案）表現足夠。

## Language Registry / 語言註冊中心
- `rustnotepad_highlight` exposes a `LanguageRegistry` with default definitions for Rust, JSON, and Plain Text.  
  `rustnotepad_highlight` 提供 `LanguageRegistry`，預設包含 Rust、JSON 與純文字的語言定義。
- UDL files convert into `LanguageDefinition` instances. The registry merges keyword sets, comment markers, delimiters, custom number patterns, and operator lists.  
  UDL 檔案會轉換成 `LanguageDefinition` 實體，註冊中心會合併關鍵字集合、註解符號、分隔符、自訂數字樣式與運算子列表。
- Highlight tokens are emitted as byte ranges with semantic categories (`Keyword`, `String`, `Comment`, `Number`, `Operator`, `Identifier`, `Custom`).  
  高亮 token 以位元組範圍搭配語意分類（`Keyword`、`String`、`Comment`、`Number`、`Operator`、`Identifier`、`Custom`）輸出。
- Registry is synchronous and `Send + Sync` friendly; the GUI can hold one instance per workspace.  
  註冊中心採同步實作，並支援 `Send + Sync`，GUI 可為每個工作區保留一個實例。

## UDL Schema & Migration / UDL 結構與轉換
- `UdlDefinition` maps directly to the subset of Notepad++ XML we support (name, extensions, keyword lists, comments, delimiters, operators, number regex, case-sensitivity).  
  `UdlDefinition` 直接對應我們支援的 Notepad++ XML 子集（名稱、副檔名、關鍵字列表、註解、分隔符、運算子、數字正則、大小寫敏感設定）。
- Import/export helpers (`from_notepad_xml`, `to_notepad_xml`) round-trip the subset we need today. Unsupported constructs (styler overrides, folding instructions) are ignored for now and can be appended later as optional fields.  
  匯入/匯出工具（`from_notepad_xml`、`to_notepad_xml`）可往返處理目前所需子集；不支援的結構（造型覆寫、摺疊指令）暫時忽略，未來可做為選用欄位補上。
- Internally we use `serde`-driven JSON for persistence. Crate consumers can serialise to the same schema to persist workspaces.  
  內部透過 `serde` 將資料持久化為 JSON；使用者可序列化成相同結構以保存工作區。

## Theme Integration / 主題整合
- Theme JSON now contains a `syntax` section with per-category colours and simple font attributes (bold/italic/underline).  
  主題 JSON 現包含 `syntax` 區段，定義各分類顏色與基本字型屬性（粗體/斜體/底線）。
- `HighlightPalette` parses that section, validates hex colours, and exposes lookup helpers keyed by `HighlightKind`. Unknown keys fall back to the `Custom` bucket, enabling plugins to map bespoke categories.  
  `HighlightPalette` 解析該區段、驗證十六進位顏色，並依 `HighlightKind` 提供查詢；未知鍵值會回退到 `Custom` 類別，讓外掛可對應自訂類別。
- The GUI pipeline consumes both the layout palette (still handled by `rustnotepad_settings`) and the syntax palette exposed by the highlight crate.  
  GUI 流程同時使用佈局調色盤（仍由 `rustnotepad_settings` 管理）與 highlight crate 提供的語法調色盤。

## Decision Log / 決策紀錄
- ✅ Ship regex/UDL backend first; tree-sitter becomes an additive backend behind the same registry API.  
  ✅ 先出貨正規表示式/UDL 後端，未來在相同註冊 API 後方加入 tree-sitter。
- ✅ Store highlight palette data inside existing theme files to keep configuration unified.  
  ✅ 將高亮調色資料存放於既有主題檔以維持配置一致。
- ✅ Keep the highlight crate independent of the settings crate to avoid circular dependencies; palette parsing lives in `rustnotepad_highlight::theme`.  
  ✅ 維持 highlight crate 與 settings crate 的獨立性以避免循環依賴；調色盤解析實作於 `rustnotepad_highlight::theme`。
