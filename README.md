# RustNotePad
NotePad++ Restructure by Rust

一個使用 Rust 語言實現的簡單文字編輯器，參考 Windows 版 NotePad++。
A simple text editor implemented in Rust, inspired by NotePad++ for Windows.

## Features 功能

- 基本文字編輯 / Basic text editing
- 開啟檔案 / Open files
- 儲存檔案 / Save files
- 新建檔案 / Create new files
- 圖形使用者介面 / Graphical user interface

## Build 建置

確保已安裝 Rust 工具鏈：
Make sure you have Rust toolchain installed:

```bash
cargo build --release
```

## Run 執行

```bash
cargo run
```

## Dependencies 依賴

- `eframe` - GUI framework
- `egui` - Immediate mode GUI library
- `rfd` - File dialog support

## License 授權

MIT License

