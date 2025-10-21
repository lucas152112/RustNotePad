# Feature 3.6 – Auto-completion & Function List

## Scope
- Word and syntax completion using language dictionaries（使用語言字典提供單字與語法補全）
- Function list parsing from syntax rules and UDL/regex fallbacks（以語法規則與 UDL/正則備援解析函式清單）
- LSP integration for jump-to-definition, diagnostics, rename, formatting（整合 LSP 以支援跳轉、診斷、重新命名與格式化）
- Toggleable LSP client with configuration UI（提供可切換的 LSP 用戶端設定介面）

## Status Checklist
- [x] `design.md` drafted and reviewed（`design.md` 草擬並完成審查）
- [x] Completion engine implemented（補全引擎已實作）
- [x] Function list parsers implemented（函式清單解析器已實作）
- [x] LSP client integrated and configurable（LSP 用戶端整合並可設定）
- [x] Automated unit/integration tests added（新增自動化單元與整合測試）
- [x] E2E verification for completion panels（完成補全面板 E2E 驗證）
- [x] `compatibility.md` updated（已更新 `compatibility.md`）

## Artifacts
- Design notes: `design.md`
- Compatibility notes: `compatibility.md`
- Tests: `tests/`
- Related crates: `crates/autocomplete`, `crates/function_list`, `crates/lsp_client`

## Open Questions
- None / 無
