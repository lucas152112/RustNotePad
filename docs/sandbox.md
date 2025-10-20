# Sandbox 測試環境指南

為了在無污染的環境中驗證 Rust 與 Playwright 端到端測試，我們改採用 Docker 建立可重現的 sandbox。此方案整合了：

- 官方 Playwright 基底映像（含 Chromium/Firefox/WebKit 與 Linux sandbox 依賴）
- 完整 Rust toolchain 以及 `tauri-driver`
- 虛擬顯示（`Xvfb`），讓 GUI 在容器中可啟動
- 單一入口腳本，負責編譯、啟動 driver 與執行測試

## 1. 需求

- Docker 或 Podman（支援 `docker build`/`docker run` 指令）
- 外網環境以便拉取 Playwright 映像與 crates/npm 套件
- 專案根目錄下的程式碼（無需預先建置）

## 2. 建置 sandbox 映像

```bash
docker build -f docker/sandbox/Dockerfile -t rustnotepad-sandbox .
```

> 若使用 Podman，將指令中的 `docker` 改為 `podman` 即可。

## 3. 在 sandbox 內執行測試

```bash
docker run --rm -it \
  --ipc=host \
  -v "$PWD":/work \
  -w /work \
  rustnotepad-sandbox \
  ./scripts/dev/run_sandbox_tests.sh
```

腳本流程：

1. `npm ci --no-optional`：安裝 Playwright/Webdriver 依賴。
2. `cargo build --locked`、`cargo test --locked`：編譯並執行 Rust 單元測試。
3. 透過 `Xvfb` 啟動虛擬顯示。
4. 啟動 `tauri-driver --port 4444` 供 E2E 測試連線。
5. 執行 `npx playwright test`。

成功後會在主控台看到 `==> Tests completed successfully`。

### 常用參數

- `XVFB_DISPLAY`：預設 `:99`。若本機已占用，可在 `docker run` 時以 `-e XVFB_DISPLAY=:199` 覆寫。
- `XVFB_RESOLUTION`：預設 `1280x720x24`，適用於大多數 GUI 測試。
- `npx playwright test` 的附加參數可直接接在腳本後方，例如：

  ```bash
  docker run ... ./scripts/dev/run_sandbox_tests.sh --project=chromium --grep "status bar"
  ```

## 4. 為何此方法有效

過去若僅在裸機環境安裝 Playwright：

- Chromium 內建 sandbox 需要額外的 setuid helper 與 namespace 設定。
- `tauri-driver`、GUI 執行檔需要一套完整的 GTK/X11 依賴。
- 沒有一致的虛擬顯示，CI 或 headless 伺服器常因缺少 `$DISPLAY` 而失敗。

新流程在容器內一次準備所有相依性，確保 Playwright、tauri-driver、Xvfb 彼此協作，測試可直接執行。

## 5. 疑難排解

- **Playwright 下載瀏覽器失敗**：確認外網連線；也可在主機預先執行 `npx playwright install`，再重跑腳本。
- **`tauri-driver` 無法啟動**：檢查容器內 `logs`（`/tmp/tauri-driver.log`）；通常是 `target/debug/rustnotepad_gui` 尚未產生，可先手動 `cargo build`.
- **需要保留容器以除錯**：去除 `--rm`，並在腳本失敗後進入同一容器重試。

若仍無法解決，建議貼上指令輸出與 `/tmp/*.log` 內容以利定位。
