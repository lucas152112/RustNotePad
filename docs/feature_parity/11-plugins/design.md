# Design Draft – Feature 3.11（設計草稿 – 功能 3.11）

## 1. Architecture Overview / 架構概要
- Host is split into **platform-neutral discovery + lifecycle** and **platform bridges**.  
  主程式分為「平台無關的發現與生命週期」與「平台特定橋接」兩層。
- Discovery yields metadata that the GUI surfaces even when execution is disabled (e.g. `-noPlugin`).  
  發現階段會輸出後設資料，GUI 即使在停用外掛（如 `-noPlugin`）時仍可顯示狀態。
- Directory layout (per workspace):  
  - `plugins/wasm/<id>/plugin.json` – WASM packages  
    `plugins/wasm/<id>/plugin.json` — WASM 套件  
  - `plugins/win32/<dll>` or `plugins/win32/<name>/<name>.dll` – Windows DLL plugins  
    `plugins/win32/<dll>` 或 `plugins/win32/<name>/<name>.dll` — Windows DLL 外掛

## 2. WASM Host / WASM 宿主
- `rustnotepad_plugin_wasm` parses `plugin.json`, verifies identifiers, entry points, capability list.  
  `rustnotepad_plugin_wasm` 解析 `plugin.json`，檢查識別碼、入口檔與能力列表。
- Capability policy defaults to `buffer-read`, `register-command`, `ui-panels`, `event-subscriptions`.  
  能力政策預設僅允許 `buffer-read`、`register-command`、`ui-panels`、`event-subscriptions`。
- Disallowed capabilities (e.g. `fs-write`, `network`) produce warnings and skip the plugin.  
  若請求未允許的能力（如 `fs-write`、`network`），會記錄警告並略過載入。
- Discovery output keeps successes and failures so the GUI can report partial progress.  
  掃描結果會保留成功與失敗，GUI 可顯示部分成功的資訊。
- Runtime execution (instantiation, sandboxing via Wasmtime) lands in future work.  
  執行面（實例化、透過 Wasmtime 的沙箱）留待後續實作。
- `rustnotepad_plugin_host` boots Wasmtime, exposes `host.log`, and allows UI-triggered command execution; logs feed back into a status banner inside the GUI.  
  `rustnotepad_plugin_host` 啟動 Wasmtime、提供 `host.log` 主機函式，並允許 UI 觸發命令執行；輸出的紀錄會回饋到 GUI 狀態訊息。

## 3. Windows ABI Bridge / Windows ABI 橋接
- `rustnotepad_plugin_winabi` currently focuses on filesystem discovery to remain cross-platform.  
  `rustnotepad_plugin_winabi` 目前僅負責檔案系統掃描，以維持跨平台編譯。
- Descriptor captures DLL location and size; metadata-only directories surface as warnings.  
  描述器會記錄 DLL 的路徑與大小；僅含後設資料的資料夾會顯示警告。
- Actual ABI thunk (Scintilla/N++ message translation) remains scoped to the Windows layer.  
  實際的 ABI 轉譯（Scintilla/N++ 訊息轉換）留在 Windows 層處理。
- Added Windows-only loader using `libloading` that ensures DLLs contain required exports; non-Windows builds surface a clear unsupported error.  
  新增以 `libloading` 建立的 Windows 專屬載入器，檢查 DLL 是否具備必要匯出；非 Windows 平台則回報未支援錯誤。

## 4. Plugin Management UI / 外掛管理介面
- GUI keeps a `PluginSystem` instance that logs discovery and honours `-noPlugin`.  
  GUI 透過 `PluginSystem` 紀錄掃描結果並尊重 `-noPlugin` 旗標。
- Settings window lists discovered plugins, shows capabilities, and lets users toggle enablement when the runtime allows it.  
  設定視窗會列出已偵測到的外掛、顯示能力並在允許時提供啟用/停用切換。
- Actions (install/update/remove) will proxy to scriptable commands to keep Tauri bundle slim.  
  安裝/更新/移除將透過可腳本化指令實作，以維持 Tauri 套件精簡。

## 5. Security & Signing / 安全性與簽章
- Manifest contains `minimum_host_version`; signature metadata will be stored alongside (e.g. `signature.json`).  
  Manifest 內含 `minimum_host_version`，簽章資訊將另存（例如 `signature.json`）。
- Trust policy layers:  
  1. Manifest validation (well-formed metadata)  
     清單驗證（檢查後設資料）  
  2. Capability policy (host-level allow list)  
     能力政策（主程式許可清單）  
  3. Signature verification (future)  
     簽章驗證（後續加入）
- WASM runtime confines wall-clock, memory, host calls; resource quotas configured per capability.  
  WASM 執行期會限制時間、記憶體與主機呼叫，資源配額依能力設定。

## 6. Lifecycle / 生命週期
- Discovery runs at startup; hot reload hooks will re-scan when plugin directories change.  
  啟動時掃描，未來會在資料夾變動時重新掃描。
- Loading order:  
  1. Parse manifests → apply policy → register into manager  
  2. Instantiate runtime (future)  
  3. Bind commands/UI/event hooks  
  載入順序：解析清單→套用政策→註冊管理器；之後才實例化執行期並綁定指令/UI/事件。

## Decision log / 決策紀錄
- Prioritised manifest + capability plumbing before full runtime to unblock GUI integration.  
  先完成清單與能力管線，讓 GUI 能看到外掛狀態，再處理完整執行期。
- Chose JSON manifest for lightweight tooling; Rust-side validation keeps format tolerant.  
  採用 JSON 清單方便工具處理，並由 Rust 驗證確保格式寬鬆。
- Split Windows discovery crate so Linux/macOS builds stay green while ABI bridge evolves.  
  拆分 Windows 掃描 crate，確保在 ABI 橋接尚未完成時 Linux/macOS 仍可編譯。
