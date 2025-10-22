# Design Draft – Feature 3.7（設計草稿 – 功能 3.7）

## Macro Recording Pipeline / 巨集錄製流程
- Capture actions as semantic events referencing command identifiers or literal text insertions.  
  以語意事件擷取動作，事件可參照指令識別碼或原文插入。
- `rustnotepad_macros::MacroRecorder` maintains recorder state; starting twice raises `RecorderAlreadyActive`.  
  `rustnotepad_macros::MacroRecorder` 維護錄製狀態；重複啟動會回傳 `RecorderAlreadyActive`。
- Events serialize through `serde` into JSON to ensure deterministic storage (`MacroStore`).  
  事件透過 `serde` 序列化成 JSON，由 `MacroStore` 確保可重現的儲存格式。
- Playback delegates to a host-provided `MacroExecutor` trait, enabling UI/engine integration without tight coupling.  
  回放透過宿主提供的 `MacroExecutor` 介面委派執行，避免與 UI/引擎緊密耦合。

## Macro Persistence & Interop / 巨集保存與互通
- Stored as sorted `BTreeMap<String, RecordedMacro>` for stable ordering and dedupe guarantees.  
  以排序後的 `BTreeMap<String, RecordedMacro>` 儲存，確保讀寫順序與名稱唯一性。
- JSON schema mirrors Notepad++ simple macro definition: name, optional shortcut, event array.  
  JSON 結構與 Notepad++ 巨集概念相近：名稱、可選捷徑、事件陣列。
- Compatibility with legacy `.macro` XML remains future work; dedicated converter planned.  
  與傳統 `.macro` XML 相容性留待後續，預計額外提供轉換器。

## Run Command Specification / 執行指令規格
- `rustnotepad_runexec::RunSpec` models deterministic executions with args, working dir, env overrides, stdin payload.  
  `rustnotepad_runexec::RunSpec` 模型化可重現的執行，涵蓋參數、工作目錄、環境覆寫與標準輸入。
- Environment clearing is opt-in (`clear_env`) to satisfy sandbox requirements before applying overrides.  
  沙箱需求透過 `clear_env` 選項選擇性清除既有環境再套用覆寫。
- Results surface as `RunResult` with raw stdout/stderr buffers and millisecond duration for telemetry hooks.  
  `RunResult` 回傳原始 stdout/stderr 與執行毫秒數，便於後續遙測掛勾。
- Watchdogs exposed via `timeout`/`kill_on_timeout`; completed results mark `timed_out` when enforcement occurs.  
  透過 `timeout` 與 `kill_on_timeout` 提供監控機制，觸發時以 `timed_out` 標示於結果。

## Execution Safety Notes / 執行安全備註
- Optional timeout with automatic kill captures partial output; GUI preview now surfaces global presets.  
  可選擇啟用逾時並自動終止，同步保留輸出；GUI 預覽提供全域預設調整。
- Current sandbox inherits parent permissions; process isolation tracked under security backlog.  
  目前沙箱承襲父程序權限，真正的程序隔離列入安全性待辦。
- Input/output captured in-memory only; large payload streaming handled in future chunked API.  
  輸入輸出暫以記憶體保存，大型資料後續以分塊 API 處理。

## Decision Log / 決策紀錄
- Store macros as JSON (vs. binary) for readability and cross-tool editing.  
  使用 JSON 儲存巨集（而非二進位），便於閱讀與跨工具編輯。
- Rely on host executor trait instead of direct UI callbacks to simplify testing.  
  以執行器介面取代直接呼叫 UI 回呼，簡化測試並提升解耦度。
