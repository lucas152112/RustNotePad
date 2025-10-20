# Feature 3.10 – Command-Line Interface（功能 3.10 – 命令列介面）

## Scope / 範圍
- Support Notepad++ legacy switches (`-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>`, ...)  
  支援 Notepad++ 傳統參數（`-multiInst`、`-nosession`、`-ro`、`-n<line>`、`-c<col>`、`-l<Lang>` 等）
- Extended RustNotePad options (`--session`, `--project`, `--theme`, ...)  
  RustNotePad 擴充參數（`--session`、`--project`、`--theme` 等）
- Cross-platform invocation and shell integration  
  跨平台啟動與 shell 整合

## Status Checklist / 進度檢查清單
- [ ] `design.md` drafted and reviewed  
  尚未完成 `design.md` 撰寫與審閱
- [ ] CLI parser implemented  
  CLI 解析器尚未實作
- [ ] Integration with session/project subsystems  
  與工作階段/專案子系統的整合尚未完成
- [ ] Unit tests for argument parsing  
  參數解析單元測試尚未完成
- [ ] Integration tests for launch scenarios  
  啟動情境的整合測試尚待補齊
- [ ] Documentation updated (`rustnotepad --help`)  
  `rustnotepad --help` 文件尚未更新
- [ ] `compatibility.md` updated  
  `compatibility.md` 尚待更新

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
