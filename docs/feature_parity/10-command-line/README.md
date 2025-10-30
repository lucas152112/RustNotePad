# Feature 3.10 – Command-Line Interface（功能 3.10 – 命令列介面）

## Scope / 範圍
- Support Notepad++ legacy switches (`-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>`, ...)  
  支援 Notepad++ 傳統參數（`-multiInst`、`-nosession`、`-ro`、`-n<line>`、`-c<col>`、`-l<Lang>` 等）
- Extended RustNotePad options (`--session`, `--project`, `--theme`, ...)  
  RustNotePad 擴充參數（`--session`、`--project`、`--theme` 等）
- Cross-platform invocation and shell integration  
  跨平台啟動與 shell 整合

## Usage examples / 使用範例
- `rustnotepad -n42 -c5 src/lib.rs` – jump to line 42, column 5 in `src/lib.rs` on launch.  
  `rustnotepad -n42 -c5 src/lib.rs` – 啟動時直接跳至 `src/lib.rs` 的第 42 行第 5 欄。
- `rustnotepad --session sessions/last-good.rnsession --project work/project.json` – restore a saved session and project relative to the workspace.  
  `rustnotepad --session sessions/last-good.rnsession --project work/project.json` – 以工作區相對路徑還原既有工作階段與專案。
- `rustnotepad --workspace ~/code/docs --theme "Nordic Daylight" notes/todo.md` – switch workspace root, apply theme, and open a note.  
  `rustnotepad --workspace ~/code/docs --theme "Nordic Daylight" notes/todo.md` – 切換工作區、套用主題並開啟筆記。

## Status Checklist / 進度檢查清單
- [x] `design.md` drafted and reviewed  
  已完成 `design.md` 撰寫與盤點
- [x] CLI parser implemented  
  CLI 解析器已實作
- [x] Integration with session/project subsystems  
  已整合工作階段與專案子系統
- [x] Unit tests for argument parsing  
  參數解析單元測試已覆蓋
- [ ] Integration tests for launch scenarios  
  啟動情境的整合測試尚待補齊
- [ ] Documentation updated (`rustnotepad --help`)  
  `rustnotepad --help` 文件尚未更新
- [x] `compatibility.md` updated  
  `compatibility.md` 已更新

## Artifacts / 產出清單
- Design notes: `design.md`  
  設計筆記：`design.md`
- Compatibility notes: `compatibility.md`  
  相容性備註：`compatibility.md`
- Tests: `tests/`  
  測試資料：`tests/`
- Related crates: `crates/cmdline`, `apps/cli`, `apps/gui-tauri`  
  相關 crate：`crates/cmdline`、`apps/cli`、`apps/gui-tauri`

## Open Questions / 未決議題
- TBD  
  待定
