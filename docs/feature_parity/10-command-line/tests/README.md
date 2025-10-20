# Test Plan – Feature 3.10（測試計畫 – 功能 3.10）

## Unit tests / 單元測試
- Argument parsing permutations  
  參數解析的各種組合
- Conflict and validation rules  
  衝突與驗證規則

## Integration tests / 整合測試
- Launch scenarios with sessions/projects/themes  
  搭配工作階段/專案/主題的啟動情境
- Multi-instance vs single-instance enforcement  
  多執行個體與單執行個體的管控

## E2E scenarios / 端到端情境
- Shell integration tests on Windows/macOS/Linux  
  Windows/macOS/Linux 的 shell 整合測試
- CLI automation for file opening and encoding selection  
  使用 CLI 自動化開啟檔案與選擇編碼

## Tooling / 測試工具
- `cargo test --package cmdline`  
  `cargo test --package cmdline`
- Cross-platform CI scripts under `scripts/ci`  
  位於 `scripts/ci` 的跨平台 CI 腳本
