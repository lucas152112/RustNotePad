# Plugin Authoring Guide（外掛開發指南）

## Directory Layout / 資料夾結構
- Place manifests and binaries under a self-contained folder.  
   建議將資訊檔與可執行檔放在同一資料夾中。
- WASM plugins require `plugin.json`, the compiled `.wasm` payload, and optional `signature.json`.  
   WASM 外掛需包含 `plugin.json`、已編譯的 `.wasm` 檔案以及選用的 `signature.json`。
- Windows DLL plugins ship a single `*.dll` (or a folder containing one DLL plus assets).  
   Windows DLL 外掛需要單一 `*.dll` 檔案（或包含 DLL 與額外資源的資料夾）。

## Manifest Schema / 資訊檔格式
 ```json
 {
   "id": "dev.rustnotepad.example",
   "name": "Example Plugin",
   "version": "1.2.3",
   "entry": "bin/module.wasm",
   "capabilities": ["buffer-read", "register-command"]
 }
 ```
- `id`: reverse-DNS identifier that must be unique per workspace.  
   `id`：需具唯一性，建議採反向網域名稱格式。
- `entry`: path to the WASM module relative to the plugin root.  
   `entry`：相對於外掛根目錄的 WASM 模組路徑。
- `capabilities`: list of requested host permissions, matching the capability registry.  
   `capabilities`：宣告所需的宿主權限，必須對應既有能力清單。

## Signing / 簽章
- The default trust policy accepts Ed25519 signatures stored in `signature.json`.  
   預設信任政策接受保存在 `signature.json` 的 Ed25519 簽章。
- Use the signing helper under `scripts/dev/sign-plugin.rs` or your own Ed25519 tooling.  
   可使用 `scripts/dev/sign-plugin.rs` 或自備 Ed25519 工具簽署外掛。
- Unsigned plugins remain disabled unless users override the policy.  
   未簽章外掛預設為停用狀態，除非使用者手動允許。

## Installation Paths / 安裝目錄
- WASM plugins install to `plugins/wasm/<plugin-id>` beneath the workspace.  
   WASM 外掛會安裝到工作區下的 `plugins/wasm/<plugin-id>`。
- Windows DLL plugins install to `plugins/win32/<dll-name>`.  
   Windows DLL 外掛會安裝到 `plugins/win32/<dll-name>`。

## Installing & Updating / 安裝與更新
- **GUI**: open *Settings → Plugins*, enter the source path, select overwrite if needed, then choose *Install WASM plugin* or *Install Windows plugin*.  
  **GUI**：開啟「設定 → 外掛」，輸入來源路徑，必要時勾選覆寫，接著點選「安裝 WASM 外掛」或「安裝 Windows 外掛」。
- **CLI**:  
  **CLI**：
  - `rustnotepad-cli --workspace <path> plugin install <source>` auto-detects the kind.  
    `rustnotepad-cli --workspace <路徑> plugin install <來源>` 會自動偵測外掛類型。
  - Force a specific type with `--kind wasm` or `--kind windows`.  
    可透過 `--kind wasm` 或 `--kind windows` 指定外掛類型。
  - Use `--overwrite` when replacing an existing plugin.  
    覆寫既有外掛時，可搭配 `--overwrite`。

## Removal / 移除
- **GUI**: use *Remove plugin* in the plugin list and confirm the action.  
  **GUI**：在外掛列表中選擇「移除外掛」並確認操作。
- **CLI**: `rustnotepad-cli --workspace <path> plugin remove --wasm <plugin-id>` or `--dll <name>`.  
  **CLI**：可使用 `rustnotepad-cli --workspace <路徑> plugin remove --wasm <外掛 ID>` 或 `--dll <檔名>`。

## Verification / 相容性驗證
- On Windows, run `rustnotepad-cli plugin verify <dll-or-folder> --show-commands` to load the DLL and inspect exported commands & shortcuts.  
  Windows 平台可使用 `rustnotepad-cli plugin verify <DLL 或資料夾> --show-commands` 載入 DLL 並檢視匯出的命令與快捷鍵。

## Testing / 測試
- Run `cargo test -p rustnotepad_plugin_wasm` to validate manifests, capabilities, and signature policies.  
   透過 `cargo test -p rustnotepad_plugin_wasm` 驗證資訊檔、能力與簽章政策。
- Use `cargo test -p rustnotepad_gui plugin_admin_install_and_remove_flows_update_inventory` to ensure GUI automation still succeeds.  
   執行 `cargo test -p rustnotepad_gui plugin_admin_install_and_remove_flows_update_inventory` 以確保 GUI 自動化流程正常。
- CLI workflows are covered by `cargo test -p rustnotepad_cli --test plugin`.  
   CLI 流程則由 `cargo test -p rustnotepad_cli --test plugin` 覆蓋。
