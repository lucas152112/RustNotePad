# Test Plan – Feature 3.11（測試計畫 – 功能 3.11）

## Unit tests / 單元測試
- Message translation logic  
  訊息轉換邏輯
- WASM host capability enforcement  
  WASM 宿主的能力限制檢查
- Plugin metadata validation  
  外掛後設資料驗證
- Signature trust policy enforcement (`cargo test -p rustnotepad_plugin_wasm`)  
  簽章信任策略驗證（`cargo test -p rustnotepad_plugin_wasm`）
- Plugin discovery state toggles (`cargo test -p rustnotepad_gui`)  
  外掛掃描與啟用狀態切換（`cargo test -p rustnotepad_gui`）
- Windows ABI command metadata extraction (`cargo test -p rustnotepad_plugin_winabi`)  
  Windows ABI 命令中繼資料解析（`cargo test -p rustnotepad_plugin_winabi`）
- Plugin install/remove routines (`cargo test -p rustnotepad_plugin_admin`)  
  外掛安裝/移除流程（`cargo test -p rustnotepad_plugin_admin`）

## Integration tests / 整合測試
- Load/unload cycles for DLL & WASM plugins  
  DLL 與 WASM 外掛的載入/卸載流程
- Plugin admin operations (install/update/remove)  
  外掛管理操作（安裝/更新/移除）
- Sandbox permission enforcement  
  沙箱權限控管

## E2E scenarios / 端到端情境
- Install and run sample DLL plugin on Windows  
  在 Windows 安裝並執行範例 DLL 外掛
- Install and run sample WASM plugins across platforms  
  跨平台安裝並執行範例 WASM 外掛
- Plugin update with signature verification failure  
  簽章驗證失敗時的外掛更新流程

## Tooling / 測試工具
- `cargo test --package plugin_winabi`  
  `cargo test --package plugin_winabi`
- `cargo test --package plugin_wasm`  
  `cargo test --package plugin_wasm`
- `cargo test --package plugin_host`  
  `cargo test --package plugin_host`
- Automated plugin harness scripts under `scripts/dev`  
  `scripts/dev` 下的外掛自動化腳本
