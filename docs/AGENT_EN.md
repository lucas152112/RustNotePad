# agent.md -- RustNotePad (Full Notepad++ Feature Parity)

> Goal: re-implement **all** functionality of Notepad++ (baseline: Notepad++ v8.8.6) in Rust, and ship native builds for Linux / Windows / macOS.  
> This handbook is meant for a **CLI agent** to execute directly (task breakdown, checklists, test commands, one-click scripts).

---

## 0. Legal & Compatibility Boundaries
- **License**: Notepad++ uses GPL. If we only mirror behaviour/documentation without reusing code, choose GPLv3 for compatibility; if we reuse any code (or the work counts as a derivative), GPLv3 is mandatory.  
- **Trademark**: avoid using the "Notepad++" name or logo; project codename: **RustNotePad**.  
- **Compatibility Strategy**:
  - **Windows**: provide an **N++ Plugins ABI compatibility layer (Windows only)** to load existing DLL plugins (limited to Win32 API constraints).
  - **Cross-platform**: ship a **WASM plugin system** (Linux/macOS/Windows). Provide **reference ports** for official/popular plugins.

---

## 1. Feature Baseline
- Target parity with **Notepad++ v8.8.6** (latest stable on the Downloads page).
- Coverage derived from the official User Manual sections: Files, Editing, Searching, Views, Sessions/Workspaces/Projects, Function List, Auto-Completion, Syntax Highlighting (Built-in / UDL), Macros, Run, Shortcut Mapper, Localization, Themes, Printing, Command Line, Plugins Admin / Plugin System, etc.

> **Implementation principle**: aim for **behavioural parity** (UI/implementation may differ). Any divergence must be recorded under "Compatibility Notes".

---

## 2. Architecture & Modules
```
rustnotepad/
  crates/
    core/            # Rope buffer, multi-caret, undo/redo, clipboard, EOL/encoding
    highlight/       # tree-sitter, themes, folding rules
    search/          # Single/multi-file search, regex, indexing
    project/         # Project/workspace/session management
    function_list/   # Function list parser (grammar/regex driven)
    autocomplete/    # Word/language completion (LSP interface founded in lsp_client)
    lsp_client/      # LSP protocol client (goto/diagnostics/completion/refactor)
    macros/          # Macro recording/playback, programmable actions
    runexec/         # Run feature (external tools), I/O sandbox
    settings/        # Configuration files, shortcut mapper, theme manager
    printing/        # Print/preview, headers/footers
    plugin_winabi/   # Windows: Notepad++ Plugin ABI compatibility layer (optional)
    plugin_wasm/     # Cross-platform WASM plugin host (permission sandbox, API)
    cmdline/         # Command-line parsing & bootstrap flow
    telemetry/       # Optional telemetry (off by default), crash reporting
  apps/
    gui-tauri/       # GUI shell (Tauri)
    cli/             # CLI utilities (batch convert, search, replace, compare, etc.)
  assets/
    themes/          # Color schemes & styles
    langs/           # Language packs (localized strings)
  scripts/
    dev/             # Dev scripts, lint, formatting
    ci/              # CI/CD, testing, packaging
    release/         # Installers for all three platforms
```

---

## 3. Feature Parity Checklist
> Each item requires: **design doc**, **unit/integration tests**, **E2E scripts**, **compatibility notes**.  
> See `docs/feature_parity/` for detailed tracking templates.

### 3.1 Files / Encoding / Line Endings
- Open / New / Save / Save As / restore unsaved changes
- Encoding detection/conversion (UTF-8/UTF-16/legacy ANSI/East Asian encodings); BOM handling
- Auto-detect and switch LF/CRLF/CR endings
- Recent files, file associations, file monitoring (change prompts / auto-reload)
- **CLI parity**: batch convert `rustnotepad-cli convert --from gbk --to utf8`

### 3.2 Editing
- Multi-caret / rectangular (column) selection / column mode
- Replace / case conversion / indentation / trim whitespace / sort / deduplicate lines
- Bookmarks / folding / line numbers / gutter / document map
- Split view (multiple panes, cross-tab drag) and multi-instance policies
- Encoding-safe saves (avoid zero-byte files, permissions issues, races)

### 3.3 Search & Replace
- Current file / selection-only / multi-file / project tree searches
- Regex / reverse / case-sensitive / whole-word / per-file & aggregated results
- Jump to line/column, mark results, search within results

### 3.4 View / Interface
- Tabs, pinning/locking, color tags
- Document map, status bar (encoding/EOL/line/column/language/insert-overwrite)
- Themes / fonts, UI language, shortcut mapper

### 3.5 Syntax Highlighting & UDL
- Built-in language highlighting/folding
- **UDL** (User Defined Language): define/import/export keywords, comments, numbers, strings, folding rules
- Theme & highlight style editor

### 3.6 Auto-Completion / Function List
- Word / syntax vocabulary completion (language dictionaries)
- Function list parsing via language rules and UDL/regex
- **LSP integration**: go to definition/reference, diagnostics, formatting, rename (toggleable)

### 3.7 Macro / Run
- **Macro**: record/name/shortcut/save/load/replay
- **Run**: external tool execution (working directory/env vars/pipelines) with output panel

### 3.8 Sessions / Projects / Workspaces
- Sessions: reopen tab sets with cursor/scroll restoration
- Project Panel: add files/folders, filters, quick open
- Workspaces: multi-project switching, cross-project search

### 3.9 Printing / Preview
- Syntax-colored printing, headers/footers, page numbers/date
- Preview zoom, paper orientation/margins

### 3.10 Command Line
- Match N++ flags: `-multiInst`, `-nosession`, `-ro`, `-n<line>`, `-c<col>`, `-l<Lang>`, ...
- Add cross-platform extensions: `--session <file>`, `--project <file>`, `--theme <name>`

### 3.11 Plugins
- **Windows (compat layer)**: support existing Notepad++ DLL plugins (translate Scintilla/N++ messages)
- **Cross-platform (primary interface)**: WASM plugins (least privilege; host API: buffer read/write, command registration, UI panels, event subscriptions, constrained file/network access)
- **Plugin management**: catalog index, signature validation, install/update/disable/remove, dependency checks

### 3.12 Localization / Themes / Preferences
- Language packs (XML/JSON/TOML with pluralization rules)
- Theme color mapping (import `.xml` / `.tmTheme` / `.sublime-syntax`)
- Preferences UI plus import/export

---

## 4. Milestones (24-week reference)
1. **W1-W4: Core & File System** (core/fs/search/settings/cmdline)  
2. **W5-W8: GUI/Multi-file/Search/Theme/Shortcut Mapper** (gui-tauri, search, settings)  
3. **W9-W12: Highlighting/UDL/Function List/Printing** (highlight/function_list/printing)  
4. **W13-W16: LSP & Auto-complete, Macro/Run, Project/Session**  
5. **W17-W20: Plugins (WASM) + Windows ABI compatibility PoC**  
6. **W21-W24: Stabilization, compatibility validation, performance/stress, release candidate**

---

## 5. Testing & Acceptance
- **Unit tests**: each crate > 80% coverage.
- **Integration/E2E**: automate GUI through Tauri tooling or Playwright.
- **Compatibility validation**: craft test cases mirroring every chapter of the Notepad++ User Manual.  
- **Cross-platform matrix**: Windows 10/11, Ubuntu LTS, macOS (Intel & Apple Silicon).
- **Performance baselines**:
  - Load 500 MB file in < 2 s (SSD)
  - Search 10k files in < 4 s (cold cache)
  - Editing & scrolling at 60 FPS (typical project)

---

## 6. CLI Agent Tasks (Direct Invocations)

### 6.1 One-shot Bootstrapping
```bash
make init             # Install toolchains, pre-commit, git hooks
make bootstrap        # Generate sub-crates, workspace, template config
```

### 6.2 Build & Run
```bash
make build            # Full release build
make dev              # Debug build with watch/recompile
make run              # Launch GUI
make cli              # Build CLI tools
```

### 6.3 Testing
```bash
make test             # Unit + integration tests
make e2e              # End-to-end GUI suite
make bench            # Performance benchmarks
```

### 6.4 Packaging
```bash
make dist             # Produce Windows MSIX, macOS .app/.dmg, Linux AppImage/DEB/RPM
```

---

## 7. Plugin API (WASM) Draft
```text
Host API:
  fs.read(path, range?) -> bytes/text
  buf.read(range?) -> text
  buf.apply(edits[]) -> ok
  ui.registerCommand(id, title, handler)
  ui.showPanel({html|text})
  events.subscribe({onOpen,onSave,onChange,onSelection,onKey})
  net.fetch(url, opts) [requires permission]
  workspace.search(query, opts) -> results
Security: permission declaration (fs.readonly, net.fetch, workspace.search), sandbox quotas, signature validation.
```

---

## 8. Compatibility Notes (Key Differences)
- **Scintilla**: Notepad++ uses Win32 + Scintilla directly; RustNotePad wraps behaviour via Rust/cross-platform GUI--verify feature-by-feature.  
- **Plugins (Windows DLL)**: compatibility layer is Windows-only; Linux/macOS should favour WASM plugins.  
- **UI details**: behaviourally equivalent but not identical visuals; map and allow customizing all shortcuts/dialogues.

---

## 9. Risks & Mitigations
- **Plugin compatibility complexity**: target the top 10 plugins first for official ports/examples to seed the WASM ecosystem.  
- **Performance**: adopt viewport-only rendering, incremental highlight caching, sharded indexing, memory-mapped I/O.  
- **Cross-platform filesystem quirks**: unify path/permission abstractions, invest in robust error handling and rollback strategies.

---

## 10. Kickoff Commands (CLI Agent)
```bash
# 1) Scaffold project skeleton
cargo install create-tauri-app --locked
create-tauri-app rustnotepad --template vanilla
# 2) Add workspace & crates (generated by template script)
./scripts/dev/scaffold.sh
# 3) Launch GUI
npm install && npm run tauri dev
# 4) Run tests
cargo test --workspace
```

---

## 11. Definition of Done (DoD)
- All items in the Notepad++ v8.8.6 **feature parity checklist** pass.  
- Installable on all three platforms; launching, editing, printing, searching, macros, UDL, projects/sessions all fully operational.  
- Plugins: Windows loads three legacy DLL plugins; cross-platform installs/runs three WASM sample plugins.
