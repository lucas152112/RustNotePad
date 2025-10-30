# Design Draft – Feature 3.10（設計草稿 – 功能 3.10）

## 1. Parser & Library Structure / 解析器與函式庫結構
- Implemented a dedicated crate `rustnotepad_cmdline` so both GUI and future headless tools can share parsing logic.  
  建立獨立的 `rustnotepad_cmdline` crate，供圖形介面與未來的命令列工具共用解析邏輯。
- Parsing uses a single pass over `OsString` arguments with a small pending state to collect modifiers (`-n`, `-c`, `-l`, `-ro`) before the next path.  
  解析流程採一次遍歷 `OsString`，並以暫存狀態收集 `-n`、`-c`、`-l`、`-ro` 等修飾後再套用到下一個檔案參數。
- Long options (`--session`, `--project`, `--theme`, `--workspace`) accept both `--opt=value` and `--opt value` forms; unknown options are captured for diagnostics but do not abort launch.  
  長參數同時支援 `--opt=value` 與 `--opt value`，未知選項會被記錄以供診斷，但不會中止啟動程序。
- Parser errors normalise to user-friendly messages (missing value, invalid numbers, dangling directives) surfaced before GUI bootstrapping.  
  解析器會將缺值、數字格式錯誤、孤立修飾等狀況統一回報，於 GUI 啟動前即顯示給使用者。

## 2. Launch Flow Integration / 啟動流程整合
- `main` now parses arguments first, resolves the workspace root (CLI override or default to CWD), and acquires a lightweight instance lock before creating any GUI state.  
  入口程式會先解析參數、決定工作區根目錄（可被 CLI 覆寫，否則落在目前目錄），再嘗試取得簡易的執行個體鎖定。
- The eframe factory receives both the resolved workspace root and the parsed `LaunchConfig`, ensuring deterministic state even if multiple windows were ever spawned in the future.  
  eframe 的工廠閉包會取得已解析的工作區與 `LaunchConfig`，即使未來啟動多個視窗，亦能確保狀態一致。
- Unknown long options and benign toggles (e.g. `-noPlugin`) are logged so users can verify what was ignored.  
  對於未知長參數或暫未實作的旗標（如 `-noPlugin`），系統會在啟動時記錄告警訊息，方便使用者核對。

## 3. Session / Project / Theme Application / 工作階段、專案、主題套用
- The GUI initialiser has been refactored into `RustNotePadApp::new_with_workspace_root`, allowing all subsequent configuration to happen through a dedicated `apply_launch_config`.  
  GUI 初始化重構為 `RustNotePadApp::new_with_workspace_root`，並透過 `apply_launch_config` 串接後續設定，利於測試與重用。
- Session restore order: explicit `--session` overrides default storage; otherwise, the persisted workspace session loads unless `-nosession` is specified.  
  工作階段載入順序：若指定 `--session` 便優先採用；否則在未設定 `-nosession` 的情況下載入工作區預設快照。
- Project trees may be swapped with `--project` (paths resolve relative to the chosen workspace when not absolute), and the store is rebound to the provided file so future saves persist to the same location.  
  透過 `--project` 可替換專案樹；若為相對路徑則相對於工作區解析，並更新儲存器路徑以維持往後的保存行為。
- Theme selection accepts either a registered theme name or a file path. When a file is provided, the stem is treated as the theme identifier; non-existent entries generate warnings while retaining the previous theme.  
  主題可用既有名稱或檔案路徑指定，若輸入檔案則以檔名（不含副檔名）當作名稱查找，找不到時會發出警示並沿用原主題。

## 4. File Targets & Caret Handling / 檔案開啟與游標定位
- Every `FileTarget` carries optional line, column, language and read-only directives. CLI modifiers apply only to the next path, mirroring Notepad++ semantics.  
  每個 `FileTarget` 會保存行、欄、語言與唯讀設定，且修飾旗標僅套用到下一個檔案，模擬 Notepad++ 的用法。
- Paths are resolved against the workspace root so scripts can use relative paths safely; caret positioning converts 1-based CLI coordinates to internal character indices.  
  檔案路徑會相對於工作區解析，方便腳本使用相對路徑；游標定位則將 1-based 參數換算為內部字元索引。
- Placeholder preview / untitled tabs created during boot are pruned once session/files are applied to avoid polluting the workspace.  
  啟動時產生的預覽或未命名分頁在套用工作階段或文件後會被移除，以避免汙染實際工作環境。

## 5. Multi-Instance Guard / 多重執行個體控制
- Default behaviour enforces single-instance by creating a sentinel file (`.rustnotepad/instance.lock`) using `create_new`; `-multiInst` switches to permit concurrent launches.  
  預設會透過 `create_new` 建立 `.rustnotepad/instance.lock` 來確保單一執行個體；`-multiInst` 可解除限制允許多重開啟。
- The guard cleans up the lock file on drop; if the file already exists, the process exits with an explanatory message before the GUI initialises.  
  鎖定器在釋放時會刪除鎖檔，若檔案已存在則於 GUI 啟動前顯示提示並離開。

## 6. Current Limitations / 目前限制
- `--theme` path handling relies on existing theme names; full on-the-fly JSON ingestion is deferred to future work.  
  目前 `--theme` 仍以既有主題名稱為主，尚未支援即時載入外部 JSON 主題。
- Column/selection restore only targets the active tab; secondary window layouts from session files are not yet reflected in the UI.  
  工作階段僅對焦於作用中分頁的游標與選取範圍，尚未還原多窗格佈局。
- `rustnotepad --help` does not emit an auto-generated usage banner; documentation lives in `docs/feature_parity/10-command-line/`.  
  目前沒有自動生成的 `--help` 說明，相關文件可於 `docs/feature_parity/10-command-line/` 查閱。

## Decision log / 決策紀錄
- Adopted a standalone parser crate to keep `rustnotepad_gui` lean and enable reuse by future CLI binaries.  
  採用獨立解析 crate 以維持 GUI 簡潔並利於未來命令列工具重用。
- Chose file-based locking (`create_new`) over platform-specific primitives to avoid new dependencies under sandboxed environments.  
  在沙盒環境下為避免額外依賴，選擇以 `create_new` 檔案鎖替代平台特定的鎖定機制。
- Prioritised incremental feature parity: legacy flags affecting session/theme/file opening were implemented first, while advanced flags (plugin control, help banner) are documented as follow-up work.  
  優先完成影響工作階段、主題與檔案開啟的傳統旗標；進階旗標（如外掛控制、說明輸出）暫列在後續規劃中。
