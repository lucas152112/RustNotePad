# Design Draft – Feature 3.6（設計草稿 – 功能 3.6）

## Overview / 概述
Feature 3.6 stitches together three subsystems that can be evolved independently.  
功能 3.6 整合三個可獨立演進的子系統。

1. **Autocomplete engine** – merges suggestions from document analysis, language dictionaries, user snippets, and (optionally) LSP completions.  
   **自動完成引擎** —— 合併文件分析、語言字典、使用者片段，與（可選）LSP 補全提供的建議。
2. **Function list service** – produces a navigable outline of the active document using structured parsers with regex fallbacks.  
   **函式清單服務** —— 透過結構化解析器產生可導覽的大綱，並提供正規表示式備援。
3. **LSP client** – speaks the Language Server Protocol, exposing completion, navigation, diagnostics, and formatting as optional providers.  
   **LSP 用戶端** —— 實作 LSP 協定，提供補全、導覽、診斷與格式化等可選功能提供者。

Each subsystem must stay low-latency, tolerate missing language support, and remain responsive offline. Interfaces are asynchronous yet deterministic so the GUI never blocks on worker tasks.  
每個子系統都需要維持低延遲、容忍語言支援缺口，並在離線時保持回應。介面採用非同步且可預期的行為，避免 GUI 因背景工作而阻塞。

## Autocomplete Engine / 自動完成引擎
- **Core API**: `AutocompleteEngine::request(request: CompletionRequest) -> CompletionSet`. Requests carry document id, cursor position, prefix, and context flags (e.g. in-comment, string literal, trigger kind). The engine fans out to providers and merges their output.  
  **核心 API**：`AutocompleteEngine::request(request: CompletionRequest) -> CompletionSet`。請求包含文件識別碼、游標位置、前綴字串與情境旗標（例如註解內、字串內、觸發類型），引擎會呼叫已註冊的提供者並合併結果。
- **Provider abstraction**: `CompletionProvider` trait returns `ProviderResult`（items + metadata）. Built-in providers shipping now:  
  **提供者抽象**：`CompletionProvider` trait 回傳 `ProviderResult`（項目與後設資料）。本階段內建提供者如下：
  - `DocumentWordsProvider` – incremental index of unique tokens from open buffers (powered by the `core` rope change feed).  
    `DocumentWordsProvider` —— 使用 `core` 的 rope 變更事件，為開啟中的緩衝區建立增量索引。
  - `LanguageDictionaryProvider` – static keywords/snippets pulled from highlight definitions or JSON dictionaries.  
    `LanguageDictionaryProvider` —— 從語法高亮定義或 JSON 字典載入靜態關鍵字與片段。
  - `SnippetProvider` – user-defined templates persisted by the `settings` crate.  
    `SnippetProvider` —— 由 `settings` crate 儲存的使用者自訂樣板。
  - `LspProvider` – bridges to the LSP client when available; skipped gracefully otherwise.  
    `LspProvider` —— 在啟用時轉接至 LSP 用戶端，未啟用時會安靜退出。
- **Ranking**: Outputs are merged with a scored comparator. Weighting:  
  **排名**：合併結果時使用加權排序，權重如下：
  - Provider priority (default LSP > snippets > language dictionary > document words, overridable per language).  
    提供者優先順序（預設 LSP > 片段 > 語言字典 > 文件字詞，可依語言覆寫）。
  - Prefix match quality (exact case-sensitive > exact case-insensitive > fuzzy).  
    前綴匹配品質（區分大小寫完全匹配 > 不分大小寫完全匹配 > 模糊匹配）。
  - Usage frequency (workspace-level counts with decay).  
    使用頻率（工作區層級計數並具備衰減）。
  - Scope relevance (same file > project scope > global dictionary).  
    範圍相關性（同一檔案 > 專案範圍 > 全域字典）。
  Scores normalise to `[0, 1]`; ties break alphabetically for deterministic ordering.  
  最終分數會正規化到 `[0, 1]`，若分數相同則以字母排序確保結果穩定。
- **Performance**:  
  **效能考量**：
  - Document index updates run on a worker subscribed to buffer diffs; tokens live in `HashMap<String, WordStats>` keyed by lowercase form.  
    文件索引在背景工作執行，訂閱緩衝差異；標記以小寫字串為鍵儲存在 `HashMap<String, WordStats>`。
  - Providers may respond synchronously (documents/dictionaries/snippets) or via `Future`. The engine waits up to a configurable timeout (default 50 ms) before streaming partial results.  
    提供者可同步（文件/字典/片段）或經由 `Future` 回應；引擎會等待可調的逾時（預設 50 ms），之後將部份結果推送給 UI。
- **Configuration**:  
  **設定項目**：
  - Global enablement toggles. / 全域啟用開關。
  - Per-language provider ordering and dictionary mapping. / 各語言提供者排序與字典對應。
  - Size limits (per-provider cap, aggregate cap, minimum prefix). / 數量限制（單一提供者上限、合併上限、最小前綴長度）。

## Function List Service / 函式清單服務
- **Parsing pipeline**:  
  **解析流程**：
  1. `FunctionListService` receives incremental updates from the same change feed used by autocomplete.  
     `FunctionListService` 從自動完成使用的同一變更事件取得增量更新。
  2. The service resolves the active language via `highlight::LanguageRegistry`.  
     透過 `highlight::LanguageRegistry` 決定作用中的語言。
  3. It chooses a `FunctionParser` implementation (tree-sitter preferred, regex fallback).  
     選擇對應的 `FunctionParser` 實作（優先 tree-sitter，備援正規表示式）。
  4. Parsed symbols become `FunctionEntry { name, kind, range, parent }`, feeding observable state for the GUI.  
     解析出的符號轉換為 `FunctionEntry { name, kind, range, parent }`，提供 GUI 觀察使用。
- **Parser abstractions**:  
  **解析器抽象**：
  - Trait `FunctionParser` exposes `parse(document: &SyntaxSnapshot) -> Vec<FunctionEntry>`.  
    Trait `FunctionParser` 定義 `parse(document: &SyntaxSnapshot) -> Vec<FunctionEntry>`。
  - Built-in `RegexParser` consumes TOML/YAML definitions (pattern, name capture, optional kind) covering Notepad++ UDL rules.  
    內建 `RegexParser` 讀取 TOML/YAML 規則（樣式、名稱擷取、可選型別），對應 Notepad++ UDL 行為。
  - Tree-sitter integration is deferred; the trait expects async workers so grammars can drop in later.  
    Tree-sitter 整合延後完成；該 trait 預留非同步工人的介面，未來可直接掛載語法。
- **Incremental updates**: Regex parsers rescan entire buffers (fast for small/medium files). Tree-sitter will later apply incremental diffs.  
  **增量更新**：正規表示式解析器會重新掃描整份緩衝（對小型/中型檔案仍夠快）；tree-sitter 完成後將支援增量差異。
- **Output model**: Results publish through `Arc<dashmap::DashMap<DocumentId, FunctionOutline>>`, allowing GUI panels to observe without blocking workers.  
  **輸出模型**：結果透過 `Arc<dashmap::DashMap<DocumentId, FunctionOutline>>` 公佈，GUI 面板可非阻塞地訂閱。

## LSP Client / LSP 用戶端
- **Transport**: JSON-RPC 2.0 over stdio pipes with `ProcessTransport` handling spawn/restart/shutdown; other transports (TCP/WebSocket) can implement the same trait.  
  **傳輸層**：採用 JSON-RPC 2.0 透過 stdio 管道，`ProcessTransport` 負責啟動、重啟與關閉；未來可用 TCP/WebSocket 實作相同 trait。
- **Concurrency model**:  
  **併發模型**：
  - Dedicated IO thread per server handles reads/writes. / 每個伺服器配置專屬 IO 執行緒處理讀寫。
  - Requests correlate via `futures::channel::oneshot`. / 請求以 `futures::channel::oneshot` 對應回應。
  - Notifications broadcast to listeners using `tokio::broadcast`. / 通知透過 `tokio::broadcast` 分發給監聽者。
- **Capabilities**:  
  **能力範圍**：
  - Completion (`textDocument/completion`) powering `LspProvider`. / 提供 `textDocument/completion` 以支援 `LspProvider`。
  - Diagnostics reused across future UI features. / 診斷資訊供後續 UI 功能重用。
  - Definition/reference jumps, rename, formatting as async operations callable from the UI. / 定義/參考跳轉、重新命名與格式化提供 UI 呼叫的非同步操作。
  - Server-specific configuration via `settings` (command, args, env). / 伺服器設定由 `settings` 管理（指令、參數、環境變數）。
- **Resilience**:  
  **韌性策略**：
  - Offline: soft-fail when binaries are missing, gracefully downgrade to non-LSP providers. / 離線時如缺少伺服器二進位檔會回報輕量錯誤並降級至非 LSP 提供者。
  - Auto-restart with exponential backoff to avoid spin. / 自動重啟採指數回退，避免頻繁重啟。
  - Timeout/cancellation propagate from UI (closing documents cancels in-flight work). / UI 可傳遞逾時與取消（關閉文件會取消進行中的請求）。

## Configuration & Settings / 設定整合
- Centralised in `settings` crate under `AutocompleteSettings`, `FunctionListSettings`, `LspSettings`.  
  由 `settings` crate 集中管理，包含 `AutocompleteSettings`、`FunctionListSettings`、`LspSettings`。
- Workspace defaults with per-language overrides (TOML/JSON).  
  提供工作區預設值與各語言覆寫（TOML/JSON 格式）。
- GUI toggles exposed via `Settings → Auto-Completion` and `Settings → Function List`.  
  GUI 透過「設定 → 自動完成」、「設定 → 函式清單」顯示對應開關。
- Multiple LSP server definitions per language (command, root detection, init options).  
  每個語言可設定多組 LSP 伺服器（指令、根目錄偵測、初始化選項）。

## Data Ownership & Integration Points / 資料擁有權與整合點
- `core` crate emits `BufferDelta` events consumed by autocomplete and function list services.  
  `core` crate 產生的 `BufferDelta` 事件被自動完成與函式清單服務共同使用。
- `highlight` crate supplies language metadata reused by providers.  
  `highlight` crate 提供語言後設資料供各提供者重用。
- `project` crate offers workspace context (open files, project roots) to scope dictionaries.  
  `project` crate 提供工作區背景資訊（開啟檔案、專案根目錄），用於限定字典範圍。
- GUI receives completion/function list updates asynchronously to keep the main thread responsive.  
  GUI 以非同步方式接收補全與函式清單更新，確保主執行緒維持反應。

## Incremental Delivery Plan / 漸進交付計畫
1. Ship dictionary + document-word completion with settings toggles.  
   發佈字典與文件字詞補全，並提供設定開關。
2. Introduce regex-based function list parsing using existing UDL definitions.  
   以既有 UDL 定義導入正規表示式函式清單解析。
3. Integrate the LSP client for completion and diagnostics.  
   整合 LSP 用戶端提供補全與診斷。
4. Expand to navigation commands and tree-sitter-backed outlines.  
   擴充導覽指令與 tree-sitter 支援的大綱。

## Decision Log / 決策紀錄
- ✅ Providers merge via scored ranking to make cross-language priorities explicit.  
  ✅ 提供者以分數排名合併，明確呈現跨語言優先順序。
- ✅ Start with regex-based function list; tree-sitter remains additive.  
  ✅ 先從正規表示式函式清單著手，tree-sitter 後續再加入。
- ✅ LSP client runs out-of-process over stdio, matching common servers.  
  ✅ LSP 用戶端以外部程序形式透過 stdio 通訊，符合主流伺服器習慣。
- ✅ Settings drive provider toggles/order to support offline or privacy-sensitive workflows.  
  ✅ 由設定控制提供者開關與排序，以支援離線或重視隱私的流程。
