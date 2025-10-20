# Test Plan – Feature 3.7（測試計畫 – 功能 3.7）

## Unit tests / 單元測試
- Macro serialization/deserialization  
  巨集序列化/還原
- Repeat count handling  
  重播次數處理
- Command dispatch integrity  
  指令派發完整性

## Integration tests / 整合測試
- External tool execution sandbox  
  外部工具執行沙箱
- Environment variable propagation  
  環境變數傳遞
- Macro persistence across restarts  
  巨集在重啟後的保存

## E2E scenarios / 端到端情境
- Record, save, and replay complex macro  
  錄製、儲存並重播複雜巨集
- Run menu output capture  
  執行選單的輸出捕捉
- Macro + run combination sequences  
  巨集與執行指令的組合流程

## Tooling / 測試工具
- `cargo test --package macros`  
  `cargo test --package macros`
- `cargo test --package runexec`  
  `cargo test --package runexec`
- GUI automation for macro recorder UI  
  巨集錄製器 UI 的 GUI 自動化
