# Feature 3.3 – Search and Replace（功能 3.3 – 搜尋與取代）

## Scope / 範圍
- In-file, selection-only, multi-file, and project-wide search workflows  
  檔案內、僅限選取、多檔案與專案層級搜尋流程
- Regex, reverse search, case sensitivity, whole word matching options  
  正規表示式、反向搜尋、區分大小寫、全字匹配等選項
- Marking results, search-in-results, and result panel summaries  
  標記結果、在結果中再次搜尋、結果面板摘要
- Jump-to-line/column helpers, bookmarking integration  
  跳行/跳欄工具與書籤整合

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫與審核
- [x] Search engine implementation complete  
  搜尋引擎實作完成
- [x] Unit tests for pattern handling  
  已覆蓋樣式處理單元測試
- [x] Integration tests across workspace/project  
  已完成工作區/專案層級整合測試
- [x] E2E search panel coverage  
  搜尋面板端到端測試已就緒
- [x] `compatibility.md` updated  
  已更新 `compatibility.md`
- [x] Documentation & tutorials refreshed  
  文件與教學已更新

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性筆記：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Tutorial: `tutorial.md`  
  教學文件：`tutorial.md`
- Related crates: `crates/search`, `crates/project`, `apps/gui-tauri`  
  相關 crate：`crates/search`、`crates/project`、`apps/gui-tauri`

## Quickstart / 快速上手
- **CLI**: `rustnotepad-cli search <pattern> [paths...] [--regex] [--case-sensitive] [--whole-word] [--dot-matches-newline] [--replace <text>] [--apply]`  
  **CLI**：`rustnotepad-cli search <pattern> [paths...] [--regex] [--case-sensitive] [--whole-word] [--dot-matches-newline] [--replace <text>] [--apply]`
  - Omit `--apply` for a dry-run diff; include it to persist replacements.  
    省略 `--apply` 時執行預覽 diff，加入後才會寫入取代結果。
  - Directory arguments recurse automatically (WalkDir); mix files and directories freely.  
    目錄參數會自動遞迴（WalkDir），可自由混合檔案與資料夾。
- **Programmatic API**: `rustnotepad_core::SearchSession`  
  **程式介面**：`rustnotepad_core::SearchSession`
  - Instantiate with `SearchOptions`, call `refresh(&Document)` to populate matches.  
    以 `SearchOptions` 建立實例，呼叫 `refresh(&Document)` 產生符合結果。
  - Navigate via `find_next/find_previous`, replace with `replace_current` / `replace_all`.  
    使用 `find_next/find_previous` 導覽，透過 `replace_current` / `replace_all` 進行取代。
  - Integrate bookmarks using `mark_current` / `mark_all` / `clear_marks`.  
    透過 `mark_current` / `mark_all` / `clear_marks` 與書籤整合。
  - Generate result panels through `SearchSession::report()` or chain `search_in_results`.  
    以 `SearchSession::report()` 產生結果面板，或串接 `search_in_results`。

## Open Questions / 未決議題
- GUI integration timeline for incremental/highlighted search.  
  GUI 增量與高亮搜尋的整合時程。
- Background cancellation strategy for project-wide scans.  
  專案層級掃描的背景取消策略。
