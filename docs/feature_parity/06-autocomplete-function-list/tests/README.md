# Test Plan – Feature 3.6

## Unit tests
- Completion ranking heuristics（補全排序啟發式）
- Function list parser fixtures per language（各語言函式清單解析測資）
- LSP message handling parsers（LSP 訊息處理解析器）

## Integration tests
- Mixed-language workspace with UDL fallback（多語言工作區與 UDL 備援）
- LSP reconnect and diagnostics flow（LSP 重新連線與診斷流程）
- Completion latency under load（負載下的補全延遲）

## E2E scenarios
- User toggles between built-in and LSP completion（使用者在內建與 LSP 補全間切換）
- Function list navigation in large file（大型檔案中的函式清單導覽）
- Offline editing without LSP（停用 LSP 時的離線編輯）

## Tooling
- `cargo test -p rustnotepad_autocomplete`（補全功能單元測試）
- `cargo test -p rustnotepad_function_list`（函式清單單元測試）
- `cargo test -p rustnotepad_lsp_client`（LSP 用戶端測試）
- Mock LSP server harness（模擬 LSP 伺服器測試工具）

## Verification summary / 驗證摘要
- Executed GUI build check: `cargo check -p rustnotepad_gui`（執行 GUI 編譯檢查）
- Validated settings crate regression suite: `cargo test -p rustnotepad_settings`（驗證設定 crate 測試）
