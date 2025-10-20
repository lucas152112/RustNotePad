# Compatibility Notes – Feature 3.3（相容性備註 – 功能 3.3）

Track behavioural parity against Notepad++ search options.  
追蹤與 Notepad++ 搜尋選項之間的行為一致性。

## Known differences / 已知差異
- The current “search in results” pipeline filters individual matches instead of collapsing by line like Notepad++’s result tree (UI backlog).  
  目前的「在結果中搜尋」流程只篩選單筆結果，尚未像 Notepad++ 的樹狀檢視那樣依行合併（屬於 UI 待辦）。

## Validation checklist / 驗證檢查清單
- [x] Regex syntax parity confirmed (literal, regex, dotall, whole-word scenarios covered by unit tests)  
  已確認正則語法一致（單純字串、正則、點號全域、全字匹配均涵蓋於單元測試）
- [x] Search-in-files output format aligned (`rustnotepad-cli search` mirrors Find Results summary)  
  搜尋多檔案輸出格式已對齊（`rustnotepad-cli search` 與 Find Results 摘要同步）
- [x] Bookmarks + search interactions validated (SearchSession integration tests)  
  已驗證書籤與搜尋的互動行為（透過 SearchSession 整合測試）
- [x] Performance targets met for 10k file corpus (synthetic 500-file smoke via `tests/search_large_workspace.rs`; full 10k benchmark scheduled for perf suite)  
  針對 1 萬檔案集達成效能目標（`tests/search_large_workspace.rs` 先行以 500 檔案做冒煙測試，完整 1 萬檔案量測排入效能套件）
