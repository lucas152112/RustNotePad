# agent.md – RustNotePad (Full Notepad++ Parity / Notepad++ 全功能對齊)

> Goal: rebuild all Notepad++ capabilities (baseline: v8.8.6) in Rust with native builds for Linux / Windows / macOS.  
> 目標：以 Rust 重建 Notepad++ 全部功能（基準版本：v8.8.6），並提供 Linux / Windows / macOS 原生版本。
>
> This guide is intended for the CLI agent to execute directly (task breakdown, checklists, test commands, automation scripts).  
> 本指南供 CLI agent 直接執行（任務拆解、檢查清單、測試指令、自動化腳本）。

---

## 0. Legal & Compatibility Boundaries / 法務與相容性界線
- **License**: Notepad++ uses GPL. Behaviour cloning without source reuse may adopt GPLv3; any reuse or derivative work must adopt GPLv3.  
  **授權**：Notepad++ 為 GPL，若僅仿效行為且不複用原始碼可採 GPLv3；若有程式碼重用或衍生作品則必須使用 GPLv3。
- **Trademark**: avoid “Notepad++” name/logo; internal codename is **RustNotePad**.  
  **商標**：避免使用「Notepad++」名稱與標誌；內部代號為 **RustNotePad**。
- **Compatibility Strategy**: Windows supplies an ABI shim to load legacy DLL plugins; cross-platform builds provide a WASM plugin host with sandboxing and reference ports for popular plugins.  
  **相容策略**：Windows 透過 ABI 相容層支援既有 DLL 外掛；跨平臺版本提供具沙箱的 WASM 外掛宿主並附常見外掛示範移植。

---

## 1. Feature Baseline / 功能基準
- Align with Notepad++ v8.8.6; coverage mirrors official manual sections (Files, Editing, Searching, Views, Sessions, Function List, Auto-Completion, Syntax Highlighting, Macros, Run, Shortcut Mapper, Localization, Themes, Printing, Command Line, Plugins).  
  對齊 Notepad++ v8.8.6；功能面向依官方手冊章節（檔案、編輯、搜尋、檢視、會話、函式清單、自動完成、語法高亮、巨集、Run、快捷鍵映射、在地化、主題、列印、指令列、外掛）。
- Behavioural parity is mandatory; UI or internal differences must be documented under compatibility notes.  
  行為必須一致；如有 UI 或內部差異需於相容性備註記錄。

---

## 2. Architecture & Modules / 架構與模組
- `core`: rope buffer, multi-caret, undo/redo, encoding handling. / Rope 緩衝、多游標、undo/redo、編碼處理  
- `highlight`: tree-sitter integration, themes, folding rules. / tree-sitter、主題、摺疊規則  
- `search`: single/multi-file search, regex, indexing. / 單檔/跨檔搜尋、正則、索引  
- `project`: sessions/projects/workspaces. / 工作階段、專案、工作區  
- `function_list`: grammar/regex-driven parsing. / 語法/正則函式清單  
- `autocomplete`: dictionary/LSP completions (bridged via `lsp_client`). / 字典與 LSP 補全（透過 `lsp_client`）  
- `macros`, `runexec`, `settings`, `printing`, `plugin_winabi`, `plugin_wasm`, `cmdline`, `telemetry`.  
  其他模組依序涵蓋巨集、外部執行、設定、列印、外掛、指令列、遙測等。  
- Apps (`gui-tauri`, `cli`), assets (`themes`, `langs`), scripts (`dev`, `ci`, `release`) follow the same bilingual descriptions as `AGENT.md`.  
  應用 (`gui-tauri`, `cli`)、資產 (`themes`, `langs`)、腳本 (`dev`, `ci`, `release`) 描述同 `AGENT.md`。

---

## 3. Feature Checklist / 功能檢查清單
> Each feature ships with design docs, tests (unit/integration/E2E), and compatibility notes. Templates live under `docs/feature_parity/`.  
> 每項功能須包含設計文件、單元/整合/E2E 測試與相容性備註，模板位於 `docs/feature_parity/`。

(Refer to Sections 3.1–3.12 in `AGENT.md`; content is identical and bilingual.)  
（3.1–3.12 詳細項目與 `AGENT.md` 相同，已提供中英對照。）

---

## 4. Milestones / 里程碑
- W1–W4: Core, file IO, search, settings, cmdline.  
  W1–W4：核心、檔案 IO、搜尋、設定、指令列。
- W5–W8: GUI, multi-file, search, themes, shortcut mapper.  
  W5–W8：GUI、多檔、搜尋、主題、快捷鍵映射。
- W9–W12: Highlight/UDL, function list, printing.  
  W9–W12：語法高亮/UDL、函式清單、列印。
- W13–W16: LSP, autocomplete, macros/run, projects/sessions.  
  W13–W16：LSP、自動完成、巨集/Run、專案/工作階段。
- W17–W20: Plugins (WASM) + Windows ABI shim.  
  W17–W20：外掛（WASM）與 Windows ABI。
- W21–W24: Stabilisation, compatibility, performance, RC.  
  W21–W24：收斂、相容性、效能、RC。

---

## 5. Testing & Acceptance / 測試與驗收
- Unit coverage ≥80% per crate. / 每個 crate 覆蓋率 ≥80%。  
- GUI automation via Playwright/Tauri for E2E. / 端到端測試以 Playwright/Tauri 自動化。  
- Compatibility suites mirror manual chapters. / 相容性測試比照官方手冊。  
- Platform matrix: Windows 10/11, Ubuntu LTS, macOS (Intel/Apple Silicon).  
  平臺矩陣：Windows 10/11、Ubuntu LTS、macOS（Intel/Apple Silicon）。  
- Performance targets: 500 MB file <2 s, search 10k files <4 s, 60 FPS editing/scrolling.  
  效能目標：500 MB 檔案 <2 秒、搜尋 1 萬檔案 <4 秒、編輯/捲動 60 FPS。

---

## 6. CLI Agent Tasks / CLI 任務
- `make init`, `make bootstrap` – initial setup scripts.  
  `make init`, `make bootstrap` – 初始設定腳本。  
- `make build`, `make dev`, `make run`, `make cli` – build/dev commands.  
  `make build`, `make dev`, `make run`, `make cli` – 建置與開發指令。
