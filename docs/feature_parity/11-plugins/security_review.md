# Security Review – Plugin System 3.11（安全性檢視 – 外掛系統 3.11）

## Scope / 範圍
- Windows DLL bridge (`rustnotepad_plugin_winabi`)
- WASM host + permission sandbox (`rustnotepad_plugin_host`, `rustnotepad_plugin_wasm`)
- Plugin admin workflows (install/update/remove), GUI + CLI

## Findings / 檢視結果
1. **DLL loading restricted to workspace tree** – discovery traverses `plugins/win32` only; absolute-path escapes blocked.  
   DLL 載入僅限 `plugins/win32`，阻擋所有絕對路徑跳脫。
2. **Unicode enforcement** – non-Unicode DLLs rejected before command registration to avoid ANSI window hooks.  
   非 Unicode DLL 於命令註冊前遭拒，避免 ANSI hook。
3. **WASM sandbox defaults deny write/network** – policy locked down unless explicitly granted via UI prompt, stored per plugin.  
   WASM 沙箱預設拒絕寫入與網路，需 UI 授權並逐外掛保存。
4. **Signature validation** – Ed25519 signatures required; unsigned packages toggled off until user override.  
   Ed25519 簽章為強制，未簽章套件預設停用。
5. **CLI path resolution** – install/remove resolves relative paths under provided workspace, preventing traversal attacks.  
   CLI 以工作區為基準解析相對路徑，避免目錄穿越攻擊。

## Mitigations / 緩解措施
- Statically link WASM runtime to disallow dynamic module loading.  
  將 WASM 執行期靜態連結以禁止動態載入模組。
- Persist trust decisions in profile store with audit log (GUI status banner, CLI stdout).  
  於設定檔保存信任決策並透過 GUI/CLI 顯示記錄。
- Require overwrite flag for plugin updates – prevents accidental replacement.  
  更新外掛需指定覆寫旗標，避免誤覆寫。

## Residual Risks / 剩餘風險
- Windows message/Scintilla bridge now active, but telemetry on plugin callbacks remains minimal; extend logging to capture misbehaving DLLs.  
  Windows 訊息與 Scintilla 橋接已啟用，但目前回呼紀錄仍有限，需強化紀錄以追蹤異常 DLL。
- WASM capability prompts rely on user consent; need future rate-limiting to avoid social engineering spam.  
  WASM 權限提示仰賴使用者判斷，後續需增加節流與標記。
- No reputation scoring for plugin sources; relying solely on signatures + manual trust.  
  尚未導入外掛來源信譽機制，目前僅依賴簽章與人工判斷。

## Next Steps / 後續步驟
- Complete Windows message bridge and add logging for `messageProc` invocations.  
  完成 Windows 訊息橋接並紀錄 `messageProc` 呼叫。
- Implement per-plugin installation provenance (origin URL, checksum).  
  記錄外掛安裝來源與雜湊資訊。
- Integrate security prompts with centralized notification tray.  
  將安全性提示整合至通知中心，提升可見度。

Reviewed by / 檢視人：Security WG – 2025-11-05
