# agent.md — RustNotePad（Notepad++ 全功能對齊版）

> 目標：以 Rust 重構「Notepad++」之**全部功能**（以 Notepad++ v8.8.6 為基準），並提供 Linux / Windows / macOS 三平臺版本。  
> 本文件供 **CLI agent** 直接執行與追蹤（任務拆解、檢查清單、測試指令、一鍵腳本）。

---

## 0. 法務與相容性界線
- **授權**：Notepad++ 採 GPL；若本專案參照其行為/文件但**不複用其程式碼**，可選 GPLv3 以確保相容；若後續複用其碼（或實作被視為衍生作品），需採 GPLv3。  
- **商標**：避免使用「Notepad++」名稱與 logo；專案代號：**RustNotePad**。  
- **相容性策略**：
  - **Windows**：提供 **N++ Plugins ABI 相容層（僅 Windows）** 以便載入既有 DLL 外掛（受限於 Win32 API）。
  - **跨平臺**：提供 **WASM 外掛系統**（Linux/macOS/Windows 通用）。官方/熱門外掛將提供**參考移植**。

---

## 1. 版本基準（Feature Baseline）
- 目標對齊 **Notepad++ v8.8.6**（Downloads 頁面為目前最新穩定版）。
- 功能表面積來自官方 User Manual 章節：Files、Editing、Searching、Views、Sessions/Workspaces/Projects、Function List、Auto-Completion、Syntax Highlighting（Built‑in / UDL）、Macros、Run、Shortcut Mapper、Localization、Themes、Printing、Command Line、Plugins Admin/Plugin System 等。

> **實作原則**：以**功能對等**為主（UI/實現可不同）；任何行為差異需在「相容性備註」列出。

---

## 2. 架構與模組
```
rustnotepad/
  crates/
    core/            # Rope 緩衝、多游標、undo/redo、剪貼板、EOL/編碼
    highlight/       # tree-sitter / 主題 / 折疊規則
    search/          # 單檔/跨檔搜尋、regex、索引
    project/         # 專案/工作區/會話(Session)管理
    function_list/   # 函式清單解析器（基於語法/正規規則）
    autocomplete/    # 文字/語言自動完成 (LSP 介面於 lsp_client)
    lsp_client/      # LSP 協定客戶端（跳轉/診斷/補全/重構）
    macros/          # 巨集錄製/回放、可程式化動作
    runexec/         # Run 功能（外部工具）、I/O sandbox
    settings/        # 設定檔、快捷鍵映射、主題管理
    printing/        # 列印/預覽、頁首頁尾
    plugin_winabi/   # Windows: Notepad++ Plugin ABI 相容層（選配）
    plugin_wasm/     # 跨平臺 WASM 外掛 Host（權限沙箱、API）
    cmdline/         # 指令列參數解析與啟動流程
    telemetry/       # 可選遙測（預設關閉）、崩潰報告
  apps/
    gui-tauri/       # GUI 外殼（Tauri）
    cli/             # CLI 工具（批次轉碼、搜尋、取代、compare 等）
  assets/
    themes/          # 配色與樣式
    langs/           # 語言包（本地化字串）
  scripts/
    dev/             # 開發腳本、lint、格式化
    ci/              # CI/CD、測試、打包
    release/         # 三平臺安裝包
```

---

## 3. 功能對齊清單（Feature Parity Checklist）
> 每項皆需：**設計文檔**、**單元/整合測試**、**E2E 測試腳本**、**相容性備註**。

### 3.1 檔案 / 編碼 / 行尾
- 開啟/新建/另存/還原未儲存
- 編碼偵測/轉換（UTF‑8/UTF‑16/各種 ANSI/東亞編碼）；BOM 處理
- 行尾 LF/CRLF/CR 自動偵測與切換
- 最近文件、檔案關聯、檔案監控（檔案變更提示/自動重新載入）
- **CLI 等價**：批次轉檔 `rustnotepad-cli convert --from gbk --to utf8`

### 3.2 編輯
- 多游標/矩形（列）選取/列編輯模式
- 替換/大小寫轉換/縮排/修剪空白/行排序/重複行移除
- 書籤/行折疊/行號/邊欄/文件地圖
- 拆分視窗（多檢視、跨分頁拖放）、多執行個體策略
- 文字編碼安全存檔（防空檔/權限/競態）

### 3.3 搜尋與取代
- 目前檔 / 選取範圍 / 多檔 / 專案樹搜尋
- Regex / 反向 / 大小寫 / 全字 / 逐檔與彙總結果
- 跳列跳欄、標記結果、在結果中再次搜尋

### 3.4 視圖 / 介面
- 分頁、鎖定/釘選、顏色標籤
- 文件地圖、狀態列（編碼/EOL/行列/語言/插入覆寫）
- 主題/字型設定、UI 語言、快捷鍵映射（Shortcut Mapper）

### 3.5 語法高亮與 UDL
- 內建語言高亮/折疊
- **UDL**（使用者自訂語言）：定義/匯入/匯出、關鍵字、註解、數字、字串、摺疊規則
- 主題與高亮樣式編輯器

### 3.6 自動完成 / 函式清單
- 字詞/語法詞彙補全（語言詞庫）
- 函式清單解析：根據語言規則與 UDL/正規表示式
- **LSP 整合**：跳轉定義/參考、診斷、格式化、重命名（可開關）

### 3.7 巨集 / Run
- **巨集**：錄製/命名/快捷鍵/儲存與載入/可重播
- **Run**：外部工具執行（工作目錄/環境變數/管線）與輸出面板

### 3.8 會話 / 專案 / 工作區
- Session：開啟分頁集、游標與捲動位置還原
- Project Panel：加入檔案/資料夾、過濾器、快速開啟
- Workspace：多專案切換、跨專案搜尋

### 3.9 列印 / 預覽
- 語法著色列印、頁首頁尾、頁碼與日期
- 預覽縮放、紙張方向/邊界

### 3.10 指令列
- 模擬 N++ 參數：`-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>`…
- 新增跨平臺擴充：`--session <file>`, `--project <file>`, `--theme <name>`

### 3.11 外掛系統
- **Windows（相容層）**：支援既有 Notepad++ DLL 外掛（以適配層轉譯 Scintilla/N++ 訊息）
- **跨平臺（主接口）**：WASM 外掛（權限最小化，Host API：緩衝讀寫、指令註冊、UI 面板、事件訂閱、檔案/網路受控存取）
- **外掛管理**：清單索引、簽章驗證、安裝/更新/停用/移除、相依性檢查

### 3.12 本地化 / 佈景 / 偏好
- 語系檔（XML/JSON/TOML 其一，含複數規則）
- 主題配色（導入 `.xml`/`.tmTheme`/`.sublime-syntax` 映射）
- 偏好設定 UI 與匯入匯出

---

## 4. 里程碑（參考 24 週）
1. **W1‑W4：核心與檔案系統**（core/fs/search/settings/cmdline）  
2. **W5‑W8：GUI/多檔/搜尋/主題/Shortcut Mapper**（gui-tauri, search, settings）  
3. **W9‑W12：高亮/UDL/函式清單/列印**（highlight/function_list/printing）  
4. **W13‑W16：LSP 與自動完成、巨集/Run、專案/會話**  
5. **W17‑W20：外掛（WASM）+ Windows 插件相容層 PoC**  
6. **W21‑W24：收斂、相容性驗證、效能/壓測、發佈 RC**

---

## 5. 測試與驗收
- **單元測試**：每 crate 覆蓋率 > 80%。
- **整合/E2E**：以 Tauri 測試工具或 Playwright 自動化 GUI 測試。
- **相容性對照**：針對 Notepad++ User Manual 每章節撰寫對照測試案例。  
- **跨平臺矩陣**：Win10/11、Ubuntu LTS、macOS 2 版（Intel/Apple Silicon）。
- **效能基準**：
  - 開啟 500MB 檔案載入 < 2s（SSD）
  - 搜尋 10k 檔案 < 4s（冷快取）
  - 編輯與捲動 60 FPS（一般專案）

---

## 6. CLI Agent 任務（可直接呼叫）

### 6.1 一鍵初始化
```bash
make init             # 安裝工具鏈、pre-commit、git hooks
make bootstrap        # 生成子 crates、workspace、模板設定
```

### 6.2 建置 & 執行
```bash
make build            # release 全專案
make dev              # debug 監看重編
make run              # 啟動 GUI
make cli              # 建置 CLI 工具
```

### 6.3 測試
```bash
make test             # 單元+整合測試
make e2e              # 端對端 GUI 測試
make bench            # 效能基準
```

### 6.4 封裝
```bash
make dist             # 產出 Windows/MSIX, macOS .app(dmg), Linux AppImage/DEB/RPM
```

---

## 7. 外掛 API（WASM）草案
```text
Host API:
  fs.read(path, range?) -> bytes/text
  buf.read(range?) -> text
  buf.apply(edits[]) -> ok
  ui.registerCommand(id, title, handler)
  ui.showPanel({html|text})
  events.subscribe({onOpen,onSave,onChange,onSelection,onKey})
  net.fetch(url, opts) [需權限]
  workspace.search(query, opts) -> results
安全：權限宣告（fs.readonly、net.fetch、workspace.search）、沙箱資源限額、簽章驗證。
```

---

## 8. 相容性備註（關鍵差異）
- **Scintilla**：原 N++ 直接用 Win32 + Scintilla；PadRS 以 Rust/跨平臺 GUI 封裝，行為需逐項核對。  
- **Plugins（Windows DLL）**：相容層僅於 Windows 可用；Linux/macOS 建議改用 WASM 外掛。  
- **UI 細節**：等價而非完全同形；所有快捷鍵/對話框提供相容映射與可自訂。

---

## 9. 風險與對策
- **外掛相容性複雜**：先鎖定前 10 大外掛做官方移植/範例，建立 WASM 生態。  
- **效能**：採 viewport-only render、增量語法高亮快取、索引分片與記憶體映射 I/O。  
- **跨平臺文件系統差異**：路徑/權限統一抽象、加強錯誤處理與回滾。

---

## 10. 起步指令（CLI Agent）
```bash
# 1) 初始化專案骨架
cargo install create-tauri-app --locked
create-tauri-app rustnotepad --template vanilla
# 2) 加入 workspace 與 crates（以模板腳本產生）
./scripts/dev/scaffold.sh
# 3) 啟動 GUI
npm install && npm run tauri dev
# 4) 跑測試
cargo test --workspace
```

---

## 11. 成功定義（DoD）
- 與 Notepad++ v8.8.6 **功能逐項對齊**清單全部通過。  
- 三平臺打包可安裝、啟動、編輯、列印、搜尋、巨集、UDL、專案/會話完全可用。  
- 外掛：Windows 能載入 3 個經典 DLL 外掛；跨平臺能安裝/啟動 3 個 WASM 範例外掛。
