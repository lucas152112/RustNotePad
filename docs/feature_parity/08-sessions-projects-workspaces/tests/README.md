# Test Plan – Feature 3.8（測試計畫 – 功能 3.8）

## Unit tests / 單元測試
- Session serializer/deserializer  
  工作階段序列化/還原
- Project filter matching  
  專案過濾條件比對
- Workspace metadata operations  
  工作區後設資料操作

## Integration tests / 整合測試
- Session restore with split views and caret positions  
  包含分割視圖與游標位置的工作階段還原
- Cross-project search index coherence  
  跨專案搜尋索引一致性
- Workspace switch persistence  
  工作區切換後的狀態保存

## E2E scenarios / 端到端情境
- Multi-project workflow switching  
  多專案工作流程切換
- Project panel drag/drop interactions  
  專案面板的拖放互動
- Session auto-save and crash recovery  
  工作階段自動儲存與當機復原

## Tooling / 測試工具
- `cargo test --package project`  
  `cargo test --package project`
- GUI automation for session/project UI  
  工作階段/專案介面的 GUI 自動化
