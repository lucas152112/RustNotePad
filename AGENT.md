# agent.md — RustNotePad（Full Notepad++ Feature Parity / Notepad++ 全功能對齊版）

> Goal: re-implement **every** Notepad++ capability (baseline: Notepad++ v8.8.6) in Rust and ship native builds for Linux, Windows, and macOS.  
> 目標：以 Rust 重製 Notepad++ **全部功能**（基準版本：Notepad++ v8.8.6），並提供 Linux、Windows、macOS 原生版本。
>
> This handbook is tailored for the **CLI agent** to execute directly (task breakdown, checklists, test commands, one-shot scripts).  
> 本手冊供 **CLI agent** 直接引用（任務拆解、檢查清單、測試指令、一鍵腳本）。
>
> English reference: `docs/AGENT_EN.md`（maintained in bilingual format as well）。  
> 英文參考版：`docs/AGENT_EN.md`（同樣維持中英雙語）。

---

## 0. Legal & Compatibility Boundaries / 法務與相容性界線
- **License**: Notepad++ is GPL. If we mirror behaviour/docs without reusing code, choose GPLv3 for compatibility; if we reuse code (derivative work), GPLv3 is mandatory.  
  **授權**：Notepad++ 為 GPL。若僅仿效行為/文件且不挪用原始碼，可採 GPLv3；若有複用程式碼或構成衍生作品，必須採 GPLv3。
- **Trademark**: avoid using the “Notepad++” name or logo; internal codename is **RustNotePad**.  
  **商標**：避免使用「Notepad++」名稱與標誌；專案代號為 **RustNotePad**。
- **Compatibility Strategy**:  
  **相容策略**：
  - **Windows**: provide an **N++ plugin ABI shim (Windows only)** so existing DLL plugins load via translated Win32 messages.  
    **Windows**：提供 **Notepad++ 插件 ABI 相容層（僅限 Windows）**，將 Win32 訊息轉譯以載入既有 DLL。
  - **Cross-platform**: ship a **WASM plugin host** for Linux/macOS/Windows with permission sandboxing; publish sample ports for official/popular plugins.  
    **跨平臺**：推出 **WASM 外掛宿主**（Linux/macOS/Windows 通用）並給予權限沙箱；為官方/熱門外掛提供示範移植。

---

## 1. Feature Baseline / 功能對齊基準
- Target parity with **Notepad++ v8.8.6** (current stable release).  
  對齊 **Notepad++ v8.8.6**（最新穩定版）。
- Scope follows the official User Manual sections: Files, Editing, Searching, Views, Sessions / Workspaces / Projects, Function List, Auto-Completion, Syntax Highlighting (Built-in & UDL), Macros, Run, Shortcut Mapper, Localization, Themes, Printing, Command Line, Plugin Admin / Plugin System.  
  功能範圍依官方手冊章節：檔案、編輯、搜尋、檢視、會話/工作區/專案、函式清單、自動完成、語法高亮（內建與 UDL）、巨集、Run、快捷鍵映射、在地化、主題、列印、指令列、外掛管理/系統等。
- **Implementation principle**: aim for behavioural parity; UI or internal design may differ, but every deviation must be recorded under Compatibility Notes.  
  **實作原則**：以行為一致為優先；UI 或內部實現可不同，但任何差異需記錄在相容性備註。

---

## 2. Architecture & Modules / 架構與模組
```
rustnotepad/
  crates/
    core/            # Rope buffer, multi-caret, undo/redo, clipboard, EOL/encoding
    highlight/       # Tree-sitter, themes, folding rules
    search/          # Single/multi-file search, regex, indexing
    project/         # Project/workspace/session management
    function_list/   # Function list parser (grammar/regex driven)
    autocomplete/    # Word/language completion (LSP bridge in lsp_client)
    lsp_client/      # LSP protocol client (goto, diagnostics, completion, refactor)
    macros/          # Macro recording/playback, programmable actions
    runexec/         # Run external tools, I/O sandbox
    settings/        # Config storage, shortcut mapper, theme manager
    printing/        # Printing/preview, headers/footers
    plugin_winabi/   # Windows: Notepad++ plugin ABI compatibility (optional)
    plugin_wasm/     # Cross-platform WASM plugin host (sandboxed APIs)
    cmdline/         # Command-line parsing & bootstrap
    telemetry/       # Optional metrics (off by default), crash reporting
  apps/
    gui-tauri/       # GUI shell built with Tauri
    cli/             # CLI utilities (convert/search/replace/compare)
  assets/
    themes/          # Colour schemes & UI styles
    langs/           # Localisation packs
  scripts/
    dev/             # Development helpers (lint, format, tooling)
    ci/              # CI/CD scripts, automation, packaging
    release/         # Cross-platform installer generation
```
```
rustnotepad/
  crates/
    core/            # Rope 緩衝、多游標、undo/redo、剪貼簿、EOL/編碼
    highlight/       # tree-sitter、主題、摺疊規則
    search/          # 單檔/跨檔搜尋、正則、索引
    project/         # 專案/工作區/工作階段管理
    function_list/   # 函式清單解析（語法/正則）
    autocomplete/    # 文字/語言自動完成（透過 lsp_client）
    lsp_client/      # LSP 客戶端（跳轉、診斷、補全、重構）
    macros/          # 巨集錄製/回放、程式化操作
    runexec/         # Run 外部工具、I/O 沙箱
    settings/        # 設定檔、快捷鍵映射、主題管理
    printing/        # 列印/預覽、頁首頁尾
    plugin_winabi/   # Windows：Notepad++ 插件 ABI 相容層（選配）
    plugin_wasm/     # 跨平臺 WASM 外掛宿主（沙箱 API）
    cmdline/         # 指令列解析與啟動流程
    telemetry/       # 選用遙測（預設關閉）、崩潰回報
  apps/
    gui-tauri/       # Tauri GUI 外殼
    cli/             # CLI 工具（轉檔/搜尋/取代/比對）
  assets/
    themes/          # 配色與 UI 樣式
    langs/           # 語言包
  scripts/
    dev/             # 開發腳本、lint、格式化
    ci/              # CI/CD、測試、自動化打包
    release/         # 三平臺安裝包產製
```

---

## 3. Feature Parity Checklist / 功能對齊清單
> Every feature must ship with a **design doc**, **unit/integration tests**, **E2E scripts**, and **compatibility notes**.  
> 每項功能需提供 **設計文件**、**單元/整合測試**、**端到端測試腳本** 與 **相容性備註**。  
> Templates live in `docs/feature_parity/`.  
> 模板位於 `docs/feature_parity/`。

### 3.1 Files / Encoding / Line Endings  
### 3.1 檔案 / 編碼 / 行尾
- Open / New / Save / Save As / restore unsaved sessions  
  開啟 / 新建 / 儲存 / 另存 / 還原未儲存進度
- Encoding detection & conversion (UTF‑8/UTF‑16/legacy ANSI/East Asian encodings) with BOM handling  
  編碼偵測與轉換（UTF‑8/UTF‑16/各類 ANSI/東亞編碼），並處理 BOM
- Auto-detect & switch LF / CRLF / CR endings  
  自動偵測並切換 LF / CRLF / CR
- Recent files, file associations, external change monitoring & reload prompts  
  最近檔案、檔案關聯、外部變更監控與重新載入提示
- **CLI parity**: batch convert `rustnotepad-cli convert --from gbk --to utf8`  
  **CLI 等價**：批次轉檔 `rustnotepad-cli convert --from gbk --to utf8`

### 3.2 Editing  
### 3.2 編輯
- Multi-caret, rectangular selection, column mode  
  多游標、矩形選取、欄模式
- Replace, case conversion, indentation, trim whitespace, sort, dedupe lines  
  取代、大小寫轉換、縮排、修剪空白、排序、刪除重複行
- Bookmarks, folding, line numbers, gutter, document map  
  書籤、程式碼摺疊、行號、邊欄、文件地圖
- Split views, cross-tab drag/drop, multi-instance strategy  
  分割視窗、跨分頁拖放、多執行個體策略
- Encoding-safe saves guarding zero-byte or permission issues  
  編碼安全儲存，防止零位元組或權限錯誤

### 3.3 Search & Replace  
### 3.3 搜尋與取代
- Current file / selection / multi-file / project tree search  
  目前檔案、選取範圍、多檔、專案樹搜尋
- Regex, reverse, case-sensitive, whole-word, aggregated results  
  正則、反向、區分大小寫、全字匹配、結果彙總
- Jump to line/column, mark results, search within results  
  跳行跳列、標記結果、在結果中再次搜尋

### 3.4 View / Interface  
### 3.4 檢視 / 介面
- Tabs, pin/lock, colour tags  
  分頁、釘選/鎖定、顏色標籤
- Document map, status bar (encoding/EOL/line/column/language/insert-overwrite)  
  文件地圖、狀態列（編碼/EOL/行列/語言/插入覆寫）
- Themes, fonts, UI language, shortcut mapper  
  主題、字型、介面語言、快捷鍵映射

### 3.5 Syntax Highlighting & UDL  
### 3.5 語法高亮與 UDL
- Built-in language highlighting/folding  
  內建語言高亮/摺疊
- **UDL**: define/import/export keywords, comments, numbers, strings, folding rules  
  **UDL**：定義/匯入/匯出關鍵字、註解、數字、字串、摺疊規則
- Theme & highlight style editor  
  主題與高亮樣式編輯器

### 3.6 Auto-Completion / Function List  
### 3.6 自動完成 / 函式清單
- Word / syntax completion from language dictionaries  
  文字/語法補全（語言字典）
- Function list parsing via grammar or regex/UDL fallbacks  
  函式清單解析（語法或正則/UDL 備援）
- **LSP integration**: goto definition/references, diagnostics, formatting, rename (toggleable)  
  **LSP 整合**：跳轉定義/參考、診斷、格式化、重新命名（可切換）

### 3.7 Macros / Run  
### 3.7 巨集 / Run
- Macro record/name/shortcut/save/load/replay  
  巨集錄製、命名、快捷鍵、儲存、載入、重播
- Run external tools with working directory/env vars/pipelines + output panel  
  執行外部工具（工作目錄、環境變數、管線）與輸出面板

### 3.8 Sessions / Projects / Workspaces  
### 3.8 工作階段 / 專案 / 工作區
- Session restore of tabs, cursors, scroll positions  
  工作階段復原分頁、游標、捲動位置
- Project panel (add files/folders, filters, quick open)  
  專案面板（加入檔案/資料夾、過濾、快速開啟）
- Workspace switching & cross-project search  
  工作區切換、跨專案搜尋

### 3.9 Printing / Preview  
### 3.9 列印 / 預覽
- Syntax-coloured printing with headers/footers/page metadata  
  語法著色列印（含頁首/頁尾/頁碼）
- Preview zoom, paper orientation, margins  
  預覽縮放、紙張方向、邊界設定

### 3.10 Command Line  
### 3.10 指令列
- Match N++ flags (`-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>`, …)  
  對齊 N++ 參數（`-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>` 等）
- Add cross-platform extensions (`--session`, `--project`, `--theme`, …)  
  補充跨平臺參數（`--session`, `--project`, `--theme` 等）

### 3.11 Plugins  
### 3.11 外掛系統
- **Windows**: load Notepad++ DLL plugins via compatibility layer translating Scintilla/N++ messages  
  **Windows**：透過相容層轉譯 Scintilla/N++ 訊息以載入 DLL 外掛
- **Cross-platform**: WASM plugins with minimum privileges (buffer read/write, register commands, UI panels, event hooks, constrained IO)  
  **跨平臺**：WASM 外掛（最小權限，支援緩衝區讀寫、指令註冊、UI 面板、事件掛勾、受限 I/O）
- Plugin management: index, signature checks, install/update/disable/remove, dependency resolution  
  外掛管理：清單索引、簽章驗證、安裝/更新/停用/移除、相依性處理

### 3.12 Localization / Themes / Preferences  
### 3.12 在地化 / 主題 / 偏好
- Language packs (XML/JSON/TOML with pluralisation rules)  
  語言包（XML/JSON/TOML，支援複數規則）
- Theme import (`.xml`, `.tmTheme`, `.sublime-syntax`)  
  主題匯入（`.xml`, `.tmTheme`, `.sublime-syntax`）
- Preference UI with import/export  
  偏好設定 UI，支援匯入/匯出

---

## 4. Milestones (24-week reference) / 里程碑（24 週規劃）
1. **W1–W4: Core & File System** – core/fs/search/settings/cmdline  
   **W1–W4：核心與檔案系統** – core/fs/search/settings/cmdline
2. **W5–W8: GUI, Multi-file, Search, Themes, Shortcut Mapper** – gui-tauri, search, settings  
   **W5–W8：GUI、多檔、搜尋、主題、快捷鍵映射** – gui-tauri、search、settings
3. **W9–W12: Highlighting, UDL, Function List, Printing** – highlight/function_list/printing  
   **W9–W12：語法高亮、UDL、函式清單、列印** – highlight/function_list/printing
4. **W13–W16: LSP & Autocomplete, Macros/Run, Projects/Sessions**  
   **W13–W16：LSP 與自動完成、巨集/Run、專案/工作階段**
5. **W17–W20: Plugins (WASM) + Windows ABI PoC**  
   **W17–W20：外掛（WASM）與 Windows ABI 概念驗證**
6. **W21–W24: Stabilisation, compatibility validation, performance/stress, RC**  
   **W21–W24：收斂、相容性驗證、效能/壓測、發佈 RC**

---

## 5. Testing & Acceptance / 測試與驗收
- **Unit tests**: maintain >80% coverage per crate.  
  **單元測試**：每個 crate 覆蓋率 > 80%。
- **Integration/E2E**: use Tauri automation or Playwright for GUI coverage.  
  **整合/E2E**：透過 Tauri 自動化或 Playwright 覆蓋 GUI。
- **Compatibility validation**: craft scenarios mirroring each Notepad++ manual chapter.  
  **相容性驗證**：為 Notepad++ 使用手冊每章建立對照案例。
- **Cross-platform matrix**: Windows 10/11, Ubuntu LTS, macOS (Intel & Apple Silicon).  
  **跨平臺矩陣**：Windows 10/11、Ubuntu LTS、macOS（Intel 與 Apple Silicon）。
- **Performance targets**:  
  **效能目標**：
  - Open 500 MB file < 2 s (SSD) / 開啟 500 MB 檔案 < 2 秒（SSD）
  - Search 10k files < 4 s (cold cache) / 搜尋 1 萬檔案 < 4 秒（冷快取）
  - Editing/scrolling 60 FPS (typical project) / 編輯與捲動維持 60 FPS（一般專案）

---

## 6. CLI Agent Tasks / CLI Agent 任務

### 6.1 Bootstrap (One-shot) / 一鍵初始化
```bash
make init             # Install toolchains, pre-commit, git hooks
make bootstrap        # Generate sub-crates, workspace, template config
```
```bash
make init             # 安裝工具鏈、pre-commit、git hooks
make bootstrap        # 建立子 crate、工作區與模板配置
```

### 6.2 Build & Run / 建置與執行
```bash
make build            # Full release build
make dev              # Debug build with watcher
make run              # Launch GUI shell
make cli              # Build CLI utilities
```
```bash
make build            # 建置 release 版本
make dev              # 建置/監看 debug 版本
make run              # 啟動 GUI
make cli              # 建置 CLI 工具
```
