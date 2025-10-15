# 功能 3.1 – 檔案、編碼與行尾 / Feature 3.1 – Files, Encoding & Line Endings

## 範疇 / Scope
- 開啟、建立、儲存、另存、還原未儲存文件。 / Open, create, save, save-as, and restore unsaved documents.
- 編碼偵測與轉換（UTF-8/UTF-16/多位元舊編碼）含 BOM 處理。 / Encoding detection and conversion (UTF-8/UTF-16/multi-byte legacy encodings) with BOM handling.
- 行尾偵測與 LF / CRLF / CR 切換。 / Line ending detection and switching between LF/CRLF/CR.
- 最近文件清單、檔案關聯、外部檔案變動監控與重新載入提示。 / Recent file list, file associations, and external monitoring with reload prompts.
- CLI 同步：透過 `rustnotepad-cli convert --from <enc> --to <enc>` 批次轉換。 / CLI parity: batch conversion via `rustnotepad-cli convert --from <enc> --to <enc>`.

## CLI 輔助工具 / CLI Helpers
- `rustnotepad-cli convert <files...> --to <encoding> [--line-ending <lf|crlf|cr>] [--bom <true|false>]` — 單指令完成編碼與行尾轉換。 / Single command to transform encoding and line endings.
- 支援 `--output <path>` 單檔輸出與 `--output-dir <dir>` 批次輸出。 / Supports single-file `--output <path>` and batch `--output-dir <dir>` workflows.
- 可選 `--from <encoding>` 以避免誤判錯誤轉換。 / Optional `--from <encoding>` guard prevents accidental conversions when detection disagrees.

## 狀態核對表 / Status Checklist
- [x] 已撰寫並審閱 `design.md`（第一階段聚焦 UTF-8/UTF-16）。 / `design.md` drafted and reviewed (Milestone 1 focuses on UTF-8/UTF-16).
- [x] 核心模組實作完成（第一階段範圍）。 / Core module implementation complete (Milestone 1 scope).
- [x] 自動化單元測試完成。 / Automated unit tests implemented.
- [x] 整合 / CLI 測試完成。 / Integration / CLI tests implemented.
- [ ] 尚未撰寫 E2E 回歸流程。 / E2E regression still pending.
- [ ] `compatibility.md` 尚待補完行為差異。 / `compatibility.md` needs remaining behaviour diffs.
- [ ] 使用者文件與指引尚待更新。 / Documentation & user guidance updates pending.

## 產出物 / Artifacts
- 設計筆記：`design.md`。 / Design notes: `design.md`.
- 相容性筆記：`compatibility.md`。 / Compatibility notes: `compatibility.md`.
- 測試規劃：`tests/`。 / Test plans: `tests/`.
- 相關 crate：`crates/core`, `crates/settings`, `apps/cli`。 / Related crates: `crates/core`, `crates/settings`, `apps/cli`.

## 未決議題 / Open Questions
- 如何擴充至其他舊式編碼且維持效能？ / How do we extend beyond UTF-8/UTF-16 while preserving performance?
- 跨平台檔案監控應該採用何種 API？ / Which cross-platform file monitoring API should we adopt?
