# Test Plan – Feature 3.5（測試計畫 – 功能 3.5）

## Unit tests / 單元測試
- Grammar loader success/failure cases (`LanguageRegistry` highlighting helpers)  
  文法載入成功/失敗情境（`LanguageRegistry` 高亮輔助函式）
- Theme parser validation (`parse_highlight_palette`)  
  主題解析驗證（`parse_highlight_palette`）
- UDL schema serialization and Notepad++ XML round-trip  
  UDL 結構序列化與 Notepad++ XML 往返測試

## Integration tests / 整合測試
- Highlight diffs between standard and custom themes (pending GUI hook-up)  
  標準與自訂主題間的高亮差異（待 GUI 串接）
- UDL import/export round trip from Notepad++ XML (`udl::tests::round_trip_udl_xml`)  
  基於 Notepad++ XML 的 UDL 匯入/匯出往返（`udl::tests::round_trip_udl_xml`）
- Folding behaviour across languages (pending)  
  各語言摺疊行為（待完成）

## E2E scenarios / 端到端情境
- Large document highlighting performance  
  大型文件的高亮效能
- Real-time UDL edits with live preview  
  UDL 即時編輯與即時預覽
- Theme editor UI regression  
  主題編輯器的 UI 回歸

## Tooling / 測試工具
- `cargo test -p rustnotepad_highlight`  
  `cargo test -p rustnotepad_highlight`
- Snapshot tests for syntax highlighting  
  語法高亮的快照測試
