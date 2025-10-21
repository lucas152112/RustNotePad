# Compatibility Notes – Feature 3.6

## Known differences
- Preview build ships with simulated LSP responses; production build will connect to real servers.（預覽版提供模擬的 LSP 建議，正式版會改為連線實際伺服器。）

## Validation checklist
- [x] Completion candidate ordering parity（補全候選排序與 Notepad++ 一致）
- [x] Function list accuracy vs Notepad++ for key languages（主要語言的函式清單準確度符合 Notepad++）
- [x] LSP optional behaviour documented（已記錄 LSP 可選行為）
- [x] Offline operation when LSP disabled（停用 LSP 時可在離線環境正常運作）
