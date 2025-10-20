# Compatibility Notes – Feature 3.5（相容性備註 – 功能 3.5）

## Known differences / 已知差異
- Folding rules currently rely on delimiter heuristics; explicit UDL folding instructions and region markers from Notepad++ are not yet parsed.  
  目前的摺疊規則依賴分隔符啟發式，尚未解析 Notepad++ UDL 的摺疊指令與區域標記。
- Styler-specific overrides (per keyword set colours, operator styles) are ignored during XML import. Themes provide global categories instead.  
  XML 匯入時忽略造型師級別的覆寫（例如關鍵字組顏色、運算子樣式），改由主題提供全域分類。
- Tree-sitter grammars are not wired up yet; languages fall back to the regex/keyword engine until the incremental backend lands.  
  尚未接入 Tree-sitter 語法，語言暫時回退至正規表示式/關鍵字引擎，待增量後端推出。

## Validation checklist / 驗證檢查清單
- [ ] Built-in language highlighting parity validated  
  內建語言高亮行為尚待驗證
- [x] UDL import/export maintains semantics  
  UDL 匯入/匯出可保持語意
- [x] Theme colour differences documented  
  已記錄主題顏色差異
- [ ] Folding behaviour matches Notepad++ reference  
  摺疊行為尚未與 Notepad++ 原版對齊
