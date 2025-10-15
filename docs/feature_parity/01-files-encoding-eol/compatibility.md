# 相容性備註 – 功能 3.1 / Compatibility Notes – Feature 3.1

使用此文件紀錄與 Notepad++ v8.8.6 行為不同之處與其原因。 / Use this document to log intentional deviations from Notepad++ v8.8.6 behaviour and their justification.

## 已知差異 / Known Differences
- 目前支援 UTF-8 與 UTF-16（LE/BE，可含 BOM）；Notepad++ 另提供廣泛 ANSI / 東亞舊編碼（尚未支援）。 / UTF-8 and UTF-16 (LE/BE, optional BOM) are supported; Notepad++ includes many extra ANSI/East Asian codepages that remain TODO.
- 檔案監控、最近文件提示與檔案關聯功能尚未實作。 / File monitoring, recent file prompts, and file association handling are not yet implemented.

## 驗證清單 / Validation Checklist
- [ ] 已於 Windows 10/11 驗證。 / Behaviour verified on Windows 10/11.
- [ ] 已於 Ubuntu LTS 驗證。 / Behaviour verified on Ubuntu LTS.
- [ ] 已於 macOS（Intel 與 Apple Silicon）驗證。 / Behaviour verified on macOS (Intel & Apple Silicon).
- [ ] 編碼轉換結果與 Notepad++ 參考案例比對。 / Encoding conversions validated against Notepad++ reference cases.
