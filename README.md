# RustNotePad / RustNotePad（正體中文）

Because Notepad++ is Windows-only, RustNotePad reimplements it in Rust so Linux and other platforms can work seamlessly with Traditional Chinese content.  
由於 Notepad++ 僅提供 Windows 版本，RustNotePad 以 Rust 重新實作，讓 Linux 等平台也能順暢處理正體中文內容。

RustNotePad is a long-term effort to recreate the full Notepad++ experience using Rust.  
RustNotePad 旨在以 Rust 重現 Notepad++ 的完整功能，提供跨平台且現代化的編輯體驗。

This repository currently provides a **UI preview** so we can iterate on layout and visual hierarchy before wiring up the real editor core.  
目前的程式碼庫提供 **UI 預覽版**，用來先行驗證版面與資訊架構，後續將逐步串接實際的編輯核心。

## UI Preview / 介面預覽

- Top-level menus for File, Edit, Search, View, Encoding, Language, Settings, Macro, Run, Plugins, Window, and Help.  
  具備檔案、編輯、搜尋、檢視、編碼、語言、設定、巨集、執行、外掛、視窗、說明等完整選單。
- Primary and secondary toolbars that showcase the main command groups.  
  主、副工具列展示常用指令群組。
- Tab strip with example documents, including a dirty indicator.  
  標籤列示範多文件開啟情境，並顯示尚未儲存的狀態。
- Left sidebar with Project Panel, Function List, and Document Switcher mocks.  
  左側欄包含專案面板、函式清單與文件切換器的預覽。
- Right sidebar for Document Map and Outline previews.  
  右側欄展示文件導覽圖與大綱。
- Bottom dock area with Find Results, Console Output, Notifications, and LSP Diagnostics tabs.  
  底部面板提供搜尋結果、主控台輸出、通知、LSP 診斷等分頁。
- Status bar mirroring Notepad++ metadata (cursor location, encoding, mode, zoom, platform).  
  狀態列同步顯示游標位置、編碼、模式、縮放與平台資訊。
- Central editor placeholder rendered with `egui`.  
  中央編輯區以 `egui` 呈現，後續會接上實際的文件緩衝。

## Running Locally / 本機執行

1. Install [Rust](https://www.rust-lang.org/tools/install) (the `cargo` toolchain is required).  
   安裝 [Rust](https://www.rust-lang.org/tools/install) 與 `cargo` 工具鏈。
2. From the repository root run / 在專案根目錄執行：

   ```bash
   cargo run -p rustnotepad_gui
   ```

   This launches the eframe window showing the full RustNotePad shell without backend functionality.  
   指令會開啟 eframe 視窗，載入 RustNotePad 的完整介面殼層（尚未串接後端功能）。

> **Note / 注意：** All controls are currently mock-ups. They exist to verify layout, naming, and docking structure before real behaviour is implemented.  
> 現階段所有控制項皆為靜態預覽，用於驗證排版、命名與停駐結構，尚未具備實際功能。

## Crates / 子套件

- `rustnotepad_core`: foundational document model with encoding/BOM round-trips, disk-change detection, recovery snapshots, file monitoring, and multi-caret primitives.  
  `rustnotepad_core`：文件模型、編碼/BOM 循環、磁碟變更偵測、還原快照、檔案監控、以及多游標基礎。
- `rustnotepad_settings`: settings and state utilities (recent files, associations, layout/theme definitions, storage helpers).  
  `rustnotepad_settings`：設定與狀態工具（最近文件、檔案關聯、版面與主題定義、儲存工具）。

## Build Script / 建置腳本

To produce a release binary named `rustnotepad` under `bin/`, execute:  
若要產出 `bin/rustnotepad` 釋出版可執行檔，請執行：

```bash
./scripts/dev/build_rustnotepad.sh
```

The script ensures `cargo` is available, builds `rustnotepad_gui` in release mode, and copies the resulting executable to `bin/rustnotepad`.  
腳本會檢查 `cargo` 是否可用、以 release 模式建置 `rustnotepad_gui`，並將可執行檔複製到 `bin/rustnotepad`。

To build multiple platform targets in one pass (requires corresponding toolchains), run:  
若要一次建置多個平台目標（需先安裝相應工具鏈），請執行：

```bash
./scripts/build-all-targets.sh
```

Set `RNP_TARGETS` (space-separated) to override the default triples.  
可透過設定以空白分隔的 `RNP_TARGETS` 覆寫預設的三元組清單。
