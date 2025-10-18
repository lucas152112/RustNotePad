# Tutorial – Feature 3.1 (Files, Encoding & Line Endings)

This guide shows how to use `rustnotepad-cli` to convert files safely across encodings and line endings.  
本指南示範如何利用 `rustnotepad-cli` 在不同編碼與行尾格式間安全轉換。

## 1. Convert UTF-8 → UTF-16 with BOM / 行尾轉換

```bash
rustnotepad-cli convert notes.txt --to utf16le --line-ending crlf --bom true --output notes-utf16.txt
```

- Preserves the original text while加入 UTF-16 LE BOM 與 CRLF 行尾。  
  保留原始內容、加入 UTF-16 LE BOM 並將行尾改為 CRLF。

## 2. Verify round-trip

```bash
rustnotepad-cli convert notes-utf16.txt --from utf16le --to utf8 --line-ending lf --output notes-back.txt
diff notes.txt notes-back.txt
```

- Converts back to UTF-8 with LF 行尾，利用 `diff` 確認無差異。  
  將檔案轉回 UTF-8 並改為 LF 行尾，使用 `diff` 驗證內容沒改變。

## 3. Batch conversions / 批次轉檔

```bash
rustnotepad-cli convert ./legacy/*.txt --to utf8 --line-ending lf --output-dir ./normalized
```

- Processes every file under `legacy/`, writing結果到 `normalized/`。  
  將 `legacy/` 內所有檔案批次轉為 UTF-8 + LF，結果輸出至 `normalized/`。

## 4. Additional checks / 進階檢驗

- The integration test `apps/cli/tests/e2e_roundtrip.rs` performs a自動 round-trip（UTF-8 ↔ UTF-16 + BOM + 行尾），可執行 `cargo test --workspace --offline` 驗證。  
- 亦可搭配 `iconv`、`file` 等系統工具再次確認編碼資訊。
