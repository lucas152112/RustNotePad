# 相容性備註 – 功能 3.1 / Compatibility Notes – Feature 3.1

使用此文件紀錄與 Notepad++ v8.8.6 行為不同之處與其原因。 / Use this document to log intentional deviations from Notepad++ v8.8.6 behaviour and their justification.

## 已知差異 / Known Differences
- 目前支援 UTF-8、UTF-16（LE/BE）與部分主流 ANSI / 東亞編碼（Windows-1252、Shift-JIS、GBK、Big5）；Notepad++ 仍額外提供更多編碼（如 KOI8/EUC-KR 等）尚未納入。 / UTF-8, UTF-16 (LE/BE), and key ANSI/East Asian encodings (Windows-1252, Shift-JIS, GBK, Big5) are supported; Notepad++ still ships additional codepages (e.g., KOI8, EUC-KR) that we have not implemented yet.
- 檔案監控、最近文件提示與檔案關聯功能尚未實作。 / File monitoring, recent file prompts, and file association handling are not yet implemented.
- 目前僅具備後端快照與監視元件，GUI 層的自動重新載入提示仍待實裝。 / Only backend snapshot/monitor components exist; GUI-layer auto-reload prompts remain TODO.

## 驗證清單 / Validation Checklist
- [ ] 已於 Windows 10/11 驗證。 / Behaviour verified on Windows 10/11.
- [ ] 已於 Ubuntu LTS 驗證。 / Behaviour verified on Ubuntu LTS.
- [ ] 已於 macOS（Intel 與 Apple Silicon）驗證。 / Behaviour verified on macOS (Intel & Apple Silicon).
- [ ] 編碼轉換結果與 Notepad++ 參考案例比對。 / Encoding conversions validated against Notepad++ reference cases.
