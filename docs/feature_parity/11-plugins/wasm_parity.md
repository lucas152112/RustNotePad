# WASM Host Parity Report（WASM 宿主相容性報告）

## Summary / 摘要
- Runtime aligns with Notepad++ plugin lifecycle: discovery, enable/disable, command dispatch, logging.  
  執行期符合 Notepad++ 外掛生命週期：掃描、啟用/停用、命令派發與紀錄。
- Capability model extends Notepad++ API surface with explicit allow-lists for file, network, and editor scopes.  
  能力模型在 Notepad++ API 之上加入明確的檔案、網路與編輯器授權清單。
- Trust policy enforces Ed25519 signatures and default signer rotation; unsigned packages require manual opt-in.  
  信任策略強制 Ed25519 簽章與預設簽署者；未簽章套件需使用者另行允許。

## Capability Mapping / 能力對應
| Capability | Description | Notepad++ parity | 備註 |
| --- | --- | --- | --- |
| `buffer-read` | Read-only buffer access via host callbacks | ✅ | 對應 `SCI_GETTEXT` 與 `SCI_GETSELTEXT` |
| `buffer-write` | Mutating editor buffer | ✅ | 對應 `SCI_REPLACESEL`；保留 undo 集成 |
| `filesystem-read` | Enumerate & read workspace files | ⚠️ | 限制於工作區；禁止絕對路徑跳脫 |
| `filesystem-write` | Create/modify files | ⚠️ | 需權限提示；預設拒絕 |
| `process-spawn` | Launch external commands | ✅ | 對應 Notepad++ `Run` 指令功能 |
| `network-client` | Outbound HTTP/WebSocket | ⚠️ | 預設停用；需明確許可與代理設定 |
| `register-command` | Add command palette entries | ✅ | 對應 Notepad++ `FuncItem` |
| `settings-read/write` | Persist plugin settings | ✅ | 存放於 `<workspace>/plugins/wasm/<id>/settings.json` |

## Sandboxing / 沙箱策略
- WASI preview2 with host shims for editor + filesystem.  
  使用 WASI preview2 並提供編輯器/檔案系統橋接。
- Mandatory permission prompts for `filesystem-write` / `network-client`.  
  `filesystem-write` 與 `network-client` 需強制授權提示。
- Resource limits: 64 MiB linear memory soft cap, 5s execution timeout per command.  
  資源限制：線性記憶體 64 MiB 軟限制、每個命令 5 秒逾時。

## Compatibility Gaps / 差異
- Pending: host clipboard APIs (`SCI_COPY`, `SCI_PASTE`).  
  待辦：剪貼簿 API 封裝。
- Pending: docking dialogs and toolbar registration equivalents.  
  待辦：浮動面板與工具列註冊介面。
- Pending: streaming binary payloads larger than 4 MiB (current channel chunked).  
  待辦：超過 4 MiB 的二進位串流需優化。

## Verification / 驗證
- `cargo test -p rustnotepad_plugin_wasm` validates manifest schema, signature policy, and capability guards.  
  `cargo test -p rustnotepad_plugin_wasm` 驗證資訊檔、簽章與能力限制。
- `cargo test -p rustnotepad_gui plugin_admin_install_and_remove_flows_update_inventory` covers runtime enable/disable wiring.  
  `cargo test -p rustnotepad_gui plugin_admin_install_and_remove_flows_update_inventory` 測 試執行期啟停流程。
- Manual E2E: sample WASM plugins (`hello-buffer`, `command-orchestrator`, `fs-watcher`) executed on Windows 11 / Ubuntu 24.04; commands completed within policy limits and logs captured for release notes.  
  手動端到端：`hello-buffer`、`command-orchestrator`、`fs-watcher` 範例已於 Windows 11 與 Ubuntu 24.04 執行，命令在策略限制內完成並已記錄於版本紀錄。
