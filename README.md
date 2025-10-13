# RustNotePad

RustNotePad is a long-term effort to recreate the full Notepad++ experience using Rust.  
This repository currently provides a **UI preview** that mirrors the major elements of the desktop application so we can iterate on layout and visual hierarchy before wiring up the real editor core.

## UI Preview

- Top-level menus for File, Edit, Search, View, Encoding, Language, Settings, Macro, Run, Plugins, Window, and Help.
- Primary and secondary toolbars that showcase the main command groups.
- Tab strip with example documents, including a dirty indicator.
- Left sidebar with Project Panel, Function List, and Document Switcher mocks.
- Right sidebar for Document Map and Outline previews.
- Bottom dock area with Find Results, Console Output, Notifications, and LSP Diagnostics tabs.
- Status bar mirroring Notepad++ metadata (cursor location, encoding, mode, zoom, platform).
- Central editor placeholder rendered with `egui` that will later connect to the actual buffer engine.

## Running locally

1. Install [Rust](https://www.rust-lang.org/tools/install) (the `cargo` toolchain is required).
2. From the repository root run:

   ```bash
   cargo run -p rustnotepad_gui
   ```

   This launches the eframe window showing the full RustNotePad shell without backend functionality.

> **Note:** All controls are non-interactive placeholders for now. They exist to verify the layout, naming, and docking structure before implementing real behaviour.

## Build script

To produce a release binary named `rustnotepad` under `bin/`, execute:

```bash
./scripts/build_rustnotepad.sh
```

The script ensures `cargo` is available, builds the `rustnotepad_gui` crate in release mode, and copies the resulting executable to `bin/rustnotepad`.
