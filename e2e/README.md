# E2E Testing with Tauri Driver

此專案使用 `tauri-driver` + `webdriverio` 進行 Tauri 桌面應用程式的端對端測試。

## 前置需求

1. 安裝 `tauri-driver` CLI 工具：
   ```bash
   cargo install tauri-driver --locked
   ```

2. 安裝 npm 依賴：
   ```bash
   npm install
   ```

## 執行測試

### 步驟 1: 建置應用程式
```bash
cargo build
```

### 步驟 2: 在背景啟動 tauri-driver
```bash
tauri-driver &
```

或在另一個終端機視窗執行：
```bash
tauri-driver
```

預設 tauri-driver 會監聽 `http://localhost:4444`。

### 步驟 3: 執行 Playwright 測試
```bash
npx playwright test
```

## 測試架構

- **Playwright**: 測試執行器與斷言框架
- **WebdriverIO**: WebDriver 客戶端，用於與 tauri-driver 通訊
- **tauri-driver**: Tauri 應用程式的 WebDriver 服務

## 注意事項

- 測試會尋找 `./target/debug/rustnotepad_gui` 執行檔
- 確保在執行測試前已經建置應用程式
- tauri-driver 必須在測試執行前啟動
