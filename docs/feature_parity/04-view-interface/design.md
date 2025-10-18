# Design Draft – Feature 3.4

## Goals
- Provide a reusable layout description that captures pane configuration, pin/lock state, and dock visibility across sessions.
- Deliver a theme system with palette + font metadata, loadable from disk and reusable by both GUI and future headless tooling.
- Elevate the UI preview so that it reflects tab pinning, color tags, split panes, document map, bottom dock, and status bar metrics.
- Expose the theme and locale selectors through the toolbar to mirror Notepad++ customisability.

## Architecture Overview

### Layout management
- Introduced `rustnotepad_settings::layout` with `LayoutConfig`, `PaneLayout`, `TabView`, and `DockLayout`.
  - `LayoutConfig` serialises to/from JSON (validated against duplicate IDs and split ratios).
  - `PaneLayout` tracks pinned vs regular tabs, active tab ID, and lightweight colour tags (`TabColorTag`).
  - `DockLayout` records visible bottom panels and the currently focused panel.
- `LayoutConfig::default()` seeds the GUI preview with representative files (mirroring the search feature work).
- Helper APIs:
  - `LayoutConfig::set_active_tab` mutates the in-memory layout when the user switches tabs.
  - `LayoutConfig::pinned_tabs` is consumed by UI/tooling for quick counts.
  - `LayoutConfig::validate_split_ratio` guards view split adjustments.

### Theme system
- Added `rustnotepad_settings::theme` with `ThemeDefinition`, `ThemeManager`, `ResolvedPalette`, `FontSettings`, and low-level `Color`.
  - Themes are JSON documents stored under `assets/themes/` (see `midnight_indigo.json`, `nordic_daylight.json`).
  - `ThemeDefinition::resolve_palette` parses hex colours into RGBA and validates fonts.
  - `ThemeManager::load_from_dir` discovers theme JSON files; falls back to built-in dark/light definitions when no files are found.
  - `ThemeManager` caches resolved palettes for cheap lookups (`active_palette`, `theme_names`, etc.).
- Colour parsing supports `#RRGGBB` and `#RRGGBBAA` forms; tests validate success/failure cases.

### GUI integration (eframe preview)
- `RustNotePadApp` now owns:
  - `LayoutConfig` (drives tab strips, split panes, bottom dock).
  - `ThemeManager` & `ResolvedPalette` (applied to `egui::Context` + `Style` when changed).
  - Locale list + status bar state.
- Toolbar enhancements:
  - Theme selector (backed by `ThemeManager::set_active_index`).
  - Locale selector (updates status bar).
  - Split ratio slider invoking `LayoutConfig::validate_split_ratio`.
- Tab strip rendering:
  - Pinned tabs rendered ahead of regular tabs.
  - Lock state annotated with `[RO]`.
  - Colour tags painted via `TabColorTag::hex`.
- Split panes:
  - Primary pane hosts the editable buffer.
  - Secondary pane displays read-only preview metadata.
- Bottom dock renders panel tabs + sample content for Find Results, Console, Notifications, and LSP diagnostics; controlled via `DockLayout`.
- Status bar aggregates cursor metrics, encoding/EOL, active document language, theme, and UI language.

### Document map & project panel
- Document map now streams lines from the live editor buffer.
- Project tree reuses static `Lazy<Vec<ProjectNode>>` but reflects layout modules (core/search/docs/tests).

## Open items
- Wire the layout/theme modules into a persistence layer once Tauri shell is introduced.
- Add per-tab close buttons and drag/drop simulation in preview.
- Integrate actual language detection and document statistics when core editor is ready.

## Decision log
- Adopted JSON for both layout and theme assets (matches ecosystem expectations and permits CLI tooling).
- Kept layout/theme logic inside `rustnotepad_settings` so CLI, GUI, and future daemons share a single source of truth.
- eframe preview applies themes lazily (only when selection changes) to avoid redundant style churn.
- Theme palette intentionally minimal (background/panel/accent/editor/status) until further UX studies.
