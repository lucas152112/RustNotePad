# Design Draft – Feature 3.8（設計草稿 – 功能 3.8）

## 1. Session Snapshot Format / 工作階段快照格式
- 採 `Serde` 序列化的 JSON 檔 (`.rnsession`)，含 `format_version` 以支援未來演進。  
  Use `Serde`-backed JSON (`.rnsession`) with a `format_version` envelope to enable forward-compatible upgrades.
- 核心欄位：`windows`（多工作視窗）、`tabs`（含 `path`、`encoding`、`cursor`、`selection`、`scroll`、`folds`、`unsaved_hash`）、以及 `pane_layout`。  
  Main fields: `windows`, each carrying `tabs` with `path`, `encoding`, `cursor`, `selection`, `scroll`, `folds`, `unsaved_hash`, plus `pane_layout`.
- `unsaved_hash` 以 64-bit 雜湊（Rust `DefaultHasher`）表示緩衝快照，提供崩潰後恢復/衝突檢測；實際文字另行儲存在 `autosave/` 目錄。  
  `unsaved_hash` stores a 64-bit fingerprint (Rust `DefaultHasher`) for crash restore & conflict detection; raw text is persisted separately in an `autosave/` staging folder.
- Session 檔案與 autosave 內容放置於預設工作區下的 `sessions/` 目錄，維持跨平臺一致（例如 `~/.config/rustnotepad/workspaces/` 或 Windows `AppData\\Roaming\\RustNotePad\\Workspaces`）。  
  Session files and autosaves live under the workspace `sessions/` directory, aligned across platforms (e.g. `~/.config/rustnotepad/workspaces/` or Windows `AppData\\Roaming\\RustNotePad\\Workspaces`).
- `SessionSnapshot` 與 `SessionTab` 等結構放在 `rustnotepad_project::session`，為 GUI / CLI 共用。  
  `SessionSnapshot`, `SessionTab`, etc. live under `rustnotepad_project::session` to serve GUI/CLI clients.
- 支援局部載入：若任一分頁無法開啟（缺檔或編碼錯誤），會標記於 `CompatibilityIssue`，並在 `compatibility.md` 紀錄差異。  
  Partial restore is allowed: tabs failing to open due to missing files/encoding errors surface a `CompatibilityIssue`, logged in `compatibility.md`.

## 2. Project Tree Model / 專案樹模型
- `ProjectTree` 採不可變結構（透過 clone-on-write 更新節點），節點分 `Folder`、`File`、`Virtual`。  
  `ProjectTree` uses a copy-on-write persistent layout with node variants `Folder`, `File`, `Virtual`.
- `Virtual` 節點提供書籤、範本或搜尋結果節點，僅存在於記憶體；序列化時以 `kind = "virtual"` 記錄。  
  `Virtual` nodes support bookmarks/templates/search result leafs that are memory-only; serialization records them via `kind = "virtual"`.
- 檔案節點維護 `metadata`（標籤、語言 override、最近開啟時間）與 `filters`（後綴/regex）；用於快速開啟與狀態列顯示。  
  File nodes maintain `metadata` (color tags, language override, last-open timestamp) and `filters` (suffix/regex) for quick-open & status bar hints.
- 全部定義於 `rustnotepad_project::tree`。提供增刪改 API，返回新樹與差異摘要，方便 GUI patch UI。  
  Implemented in `rustnotepad_project::tree`, exposing CRUD APIs that return updated trees plus a diff summary for GUI patch updates.

## 3. Workspace Registry / 工作區註冊
- `Workspace` 代表一組專案與全域設定（預設搜尋範圍、匯出路徑、臨時變數）。  
  `Workspace` captures associated projects plus shared preferences (default search scope, export paths, temp variables).
- `WorkspaceStore`（位於 `rustnotepad_project::workspace`）負責：  
  `WorkspaceStore` (within `rustnotepad_project::workspace`) handles:
  - `workspaces.json` 管理工作區清單（ID、顯示名稱、最近使用時間）。  
    `workspaces.json` indexes workspace entries (id, display name, last accessed).
  - 每個工作區對應一個 `workspace_<id>.json`，保存專案指向與自訂屬性。  
    Each workspace persists to `workspace_<id>.json` capturing project pointers and user-defined metadata.
  - 儲存路徑預設為 `~/.config/rustnotepad/workspaces/` 或 Windows `AppData\Roaming\RustNotePad\Workspaces`。  
    Storage roots default to `~/.config/rustnotepad/workspaces/` or `AppData\Roaming\RustNotePad\Workspaces` on Windows.
- 提供快取層：近期載入的工作區使用 LRU (`WorkspaceCache`) 避免頻繁 I/O。  
  Adds an LRU-backed `WorkspaceCache` for recently used workspaces to cut I/O churn.

## 4. Cross-Project Search Bridge / 跨專案搜尋橋接
- `ProjectSearchProvider` 實作 `rustnotepad_search::FileEnumerator`（新增 trait），用來枚舉符合過濾條件的檔案集合。  
  `ProjectSearchProvider` implements a new `rustnotepad_search::FileEnumerator` trait to yield files filtered by project/workspace rules.
- GUI 或 CLI 可傳入 `WorkspaceScope`（單專案/多專案/全部）與 `SearchOverrides`（忽略 `.gitignore` 等）。  
  Callers provide a `WorkspaceScope` (single project, multi-project, or all) and optional `SearchOverrides` (ignore `.gitignore`, include hidden).
- 搜尋結果回寫至 `Virtual` 節點（類型 `virtual:search-results`），便於在專案面板顯示並允許再次搜尋。  
  Search results materialize as `Virtual` nodes (`virtual:search-results`) so the project panel can display and support search-in-results.
- 對於大型專案，提供 incremental walk：初次投遞啟動 async 任務（透過 `tokio` features），並支援進度更新 Channel。  
  Large workspaces trigger an async traversal (feature-gated with `tokio`) feeding progress updates over a channel for responsive UI feedback.

## 5. GUI Sync Strategy / GUI 同步策略
- Tauri 前端透過命令 `session.save`、`session.restore` 與 `workspace.apply_diff` 操作後端。  
  Tauri frontend invokes backend commands `session.save`, `session.restore`, and `workspace.apply_diff` to manipulate state.
- 後端擁有 `SessionController`，監聽編輯器事件（開啟/關閉/焦點/換 pane）並更新內部快照；定期（每 30 秒）與關閉時強制 flush。  
  Backend exposes a `SessionController` that subscribes to editor events (open/close/focus/pane switch) to update snapshots, flushing every 30s and on shutdown.
- 專案樹更新採「意圖式 patch」：GUI 發送 `ProjectCommand`（新增資料夾、掛載 Git、套用篩選器），後端驗證並回傳新的 `ProjectTree`。  
  Project tree mutations follow an intent-based patch flow: frontend issues `ProjectCommand` (add folder, mount Git, set filter); backend validates and returns the updated `ProjectTree`.
- 同步狀態差異（例如外部檔案變更）透過 `FileWatcher` 觸發 `SessionReloadPrompt`，並在 session 快照標記 `dirty_external = true`。  
  External file changes detected via `FileWatcher` trigger a `SessionReloadPrompt` and mark `dirty_external = true` in the session snapshot.

## 6. Incremental Implementation Plan / 分階段實作規劃
- **M1**：完成 `SessionSnapshot` 序列化/反序列化，整合 autosave 儲存。  
  **M1**: Implement `SessionSnapshot` (serde serde) and autosave persistence.
- **M2**：導入 `ProjectTree` 結構與基本 CRUD + 快速開啟索引。  
  **M2**: Land `ProjectTree` data structures with CRUD and quick-open index builder.
- **M3**：建立 `WorkspaceStore` + LRU 快取，支援多工作區切換。  
  **M3**: Ship `WorkspaceStore` with LRU cache for multi-workspace switching.
- **M4**：`ProjectSearchProvider` 串接 `rustnotepad_search`，附整合測試與 GUI 事件。  
  **M4**: Wire `ProjectSearchProvider` into `rustnotepad_search` with integration tests and GUI plumbing.
- **M5**：E2E 測試覆蓋「啟動 -> 還原 session -> 切換工作區 -> 跨專案搜尋」。  
  **M5**: E2E tests cover "launch → restore session → switch workspace → cross-project search".

## 7. Decision Log / 決策紀錄
- 選擇 JSON + 版本欄位而非自訂二進位格式，優先確保除錯與手動修復便利。  
  Picked JSON with explicit versioning instead of a bespoke binary format to favour debuggability and manual recovery.
- Session 文本改以 autosave 目錄儲存，避免出現過大的 session 檔案並允許比對。  
  Persist unsaved text snapshots to an autosave folder to keep session files lightweight and diff-friendly.
- 專案樹採不可變結構，降低多執行緒同步負擔且易於生成差異事件供 GUI 使用。  
  Made the project tree persistent/immutable to simplify multi-thread sync and emit UI-friendly diffs.
- Workspace 設定拆成索引 + 每工作區 JSON，兼顧快速列舉與細節儲存。  
  Split workspace data into an index + per-workspace JSON to balance fast enumeration with rich metadata payloads.
- 搜尋橋接抽象成 trait，確保未來可擴充（例如 CLI 或 headless search）且不綁定 GUI。  
  Abstracted search enumeration behind a trait to keep future CLI/headless search integrations decoupled from the GUI.
