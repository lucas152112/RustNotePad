use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const DEFAULT_LOCALE_CODE: &str = "en-US";
const DEFAULT_DISPLAY_NAME: &str = "English (en-US)";

const DEFAULT_STRINGS: &[(&str, &str)] = &[
    (
        "sample.editor_preview",
        "// RustNotePad UI Preview\nfn main() {\n    let mut search_engine = SearchEngine::new(\"alpha beta gamma\");\n    let options = SearchOptions::new(\"beta\");\n    if let Some(hit) = search_engine.find(0, &options).expect(\"search\") {\n        println!(\"Found match at byte {}\", hit.start);\n    }\n}\n",
    ),
    ("toolbar.workspace_prefix", "Workspace"),
    ("toolbar.ui_locale", "UI Locale"),
    ("toolbar.split_prefix", "Split "),
    ("toolbar.pinned_tabs", "Pinned tabs: {0}"),
    ("menu.file", "File"),
    ("menu.file.new", "New"),
    ("menu.file.open", "Open..."),
    ("menu.file.save", "Save"),
    ("menu.file.save_as", "Save As..."),
    ("menu.file.save_all", "Save All"),
    ("menu.file.close", "Close"),
    ("menu.file.close_all", "Close All"),
    ("menu.file.exit", "Exit"),
    ("menu.edit", "Edit"),
    ("menu.edit.undo", "Undo"),
    ("menu.edit.redo", "Redo"),
    ("menu.edit.cut", "Cut"),
    ("menu.edit.copy", "Copy"),
    ("menu.edit.paste", "Paste"),
    ("menu.edit.delete", "Delete"),
    ("menu.edit.select_all", "Select All"),
    ("menu.edit.column_editor", "Column Editor..."),
    ("menu.search", "Search"),
    ("menu.search.find", "Find..."),
    ("menu.search.find_next", "Find Next"),
    ("menu.search.find_previous", "Find Previous"),
    ("menu.search.replace", "Replace..."),
    ("menu.search.find_in_files", "Find in Files..."),
    ("menu.search.bookmark", "Bookmark ▸"),
    ("menu.view", "View"),
    ("menu.view.toggle_fullscreen", "Toggle Full Screen"),
    ("menu.view.restore_zoom", "Restore Default Zoom"),
    ("menu.view.document_map", "Document Map"),
    ("menu.view.function_list", "Function List"),
    ("menu.view.project_panel", "Project Panel ▸"),
    ("menu.settings", "Settings"),
    ("menu.settings.preferences", "Preferences..."),
    ("menu.settings.style_configurator", "Style Configurator..."),
    ("menu.settings.shortcut_mapper", "Shortcut Mapper..."),
    ("menu.settings.edit_popup_menu", "Edit Popup Context Menu..."),
    ("menu.macro", "Macro"),
    ("menu.macro.start_recording", "Start Recording"),
    ("menu.macro.stop_recording", "Stop Recording"),
    ("menu.macro.playback", "Playback"),
    ("menu.run", "Run"),
    ("menu.run.run", "Run..."),
    ("menu.run.launch_chrome", "Launch in Chrome"),
    ("menu.run.launch_firefox", "Launch in Firefox"),
    ("menu.plugins", "Plugins"),
    ("menu.plugins.admin", "Plugins Admin..."),
    ("menu.plugins.open_folder", "Open Plugins Folder..."),
    ("menu.window", "Window"),
    ("menu.window.duplicate", "Duplicate"),
    ("menu.window.clone_other_view", "Clone to Other View"),
    ("menu.window.move_other_view", "Move to Other View"),
    ("menu.help", "Help"),
    ("menu.help.user_manual", "User Manual"),
    ("menu.help.debug_info", "Debug Info"),
    ("menu.help.about", "About"),
    ("panel.project.title", "Project Panel"),
    ("panel.secondary.title", "Secondary View (preview)"),
    ("panel.secondary.active", "Active: {0}"),
    ("panel.secondary.readonly", "Preview is read-only in UI mock."),
    ("panel.document_map.title", "Document Map"),
    ("panel.no_panels_configured", "No panels configured."),
    ("panel.find_results.title", "Find Results"),
    ("panel.find_results.summary", "Find Results: 5 hits across 3 files."),
    ("panel.console.title", "Console"),
    ("panel.console.command", "cargo check --workspace"),
    (
        "panel.console.finished",
        "Finished dev [unoptimized + debuginfo] target(s) in 2.34s",
    ),
    ("panel.notifications.title", "Notifications"),
    ("panel.notifications.idle", "✔ All background tasks are idle."),
    (
        "panel.notifications.font_warning",
        "⚠ Theme 'Nordic Daylight' missing custom font, using fallback.",
    ),
    ("panel.lsp.title", "LSP Diagnostics"),
    ("panel.lsp.connected", "Language server connected"),
    ("panel.lsp.offline", "Language server offline"),
    ("panel.lsp.no_diagnostics", "No diagnostics."),
    ("panel.lsp.more_diagnostics", "... {0} more diagnostics"),
    (
        "panel.generic.no_content",
        "Panel '{panel}' has no content in preview mode.",
    ),
    ("highlight.heading", "Syntax Highlight Summary"),
    ("highlight.tokens", "Tokens: {0}"),
    ("highlight.sample", "Sample"),
    ("highlight.more", "..."),
    ("highlight.error", "Highlight error: {0}"),
    ("function.heading", "Function List"),
    ("function.entry_label", "{0} {1} (Line {2})"),
    ("function.navigate_hover", "Navigate to symbol"),
    ("function.more", "... {0} additional symbols"),
    ("function.no_symbols", "No symbols detected."),
    ("autocomplete.heading", "Autocomplete"),
    ("autocomplete.enable_lsp", "Enable LSP suggestions"),
    ("autocomplete.lsp_offline", "LSP offline"),
    ("autocomplete.prefix_hint", "Prefix"),
    ("autocomplete.no_suggestions", "No suggestions available."),
    ("autocomplete.more_suggestions", "... {0} more suggestions"),
    ("autocomplete.apply_hover", "Apply suggestion"),
    ("status.position", "Ln {0}, Col {1} | Lines {2}"),
    ("status.selection", "Sel {0}"),
    ("status.lang_label", "Lang: {0}"),
    ("status.ui_label", "UI: {0}"),
    ("status.theme_label", "Theme: {0}"),
    ("tabs.close_hover", "Close tab"),
    ("explorer.open_document_hover", "Open document"),
    ("document.load_error", "// Unable to open {path}\n"),
    ("language.name.rust", "Rust"),
    ("language.name.json", "JSON"),
    ("language.name.plaintext", "Plain Text"),
    ("language.name.markdown", "Markdown"),
    (
        "fonts.warning.cjk_missing",
        "Missing system Traditional Chinese font. Install Noto Sans TC, Microsoft JhengHei, or PingFang, then restart RustNotePad.",
    ),
];

#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("failed to enumerate locale directory {0}: {1}")]
    ReadDir(PathBuf, io::Error),
    #[error("failed to read locale file {0}: {1}")]
    ReadFile(PathBuf, io::Error),
    #[error("failed to parse locale file {0}: {1}")]
    ParseFile(PathBuf, serde_json::Error),
    #[error("duplicate locale code {0}")]
    DuplicateLocale(String),
}

#[derive(Debug, Clone)]
pub struct LocaleSummary {
    pub code: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
struct LocaleCatalog {
    summary: LocaleSummary,
    strings: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct LocalizationManager {
    catalogs: Vec<LocaleCatalog>,
    active: usize,
    fallback: usize,
}

#[derive(Debug, Deserialize)]
struct LocaleFile {
    locale: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    strings: HashMap<String, String>,
}

impl LocalizationManager {
    /// Constructs a manager seeded with the built-in English resources.
    /// （以內建英文資源建立語系管理器。）
    pub fn fallback() -> Self {
        let catalog = default_catalog();
        Self {
            catalogs: vec![catalog],
            active: 0,
            fallback: 0,
        }
    }

    /// Loads locale definitions from the provided directory, falling back to English.
    /// （從指定目錄載入語系定義，並回退至英文。）
    pub fn load_from_dir(
        path: impl AsRef<Path>,
        default_locale: &str,
    ) -> Result<Self, LocalizationError> {
        let mut manager = Self::fallback();
        let dir = path.as_ref();

        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    let entry =
                        entry.map_err(|err| LocalizationError::ReadDir(dir.to_path_buf(), err))?;
                    let path = entry.path();
                    let metadata = entry
                        .metadata()
                        .map_err(|err| LocalizationError::ReadFile(path.clone(), err))?;
                    if !metadata.is_file() {
                        continue;
                    }
                    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                        continue;
                    }

                    let contents = fs::read_to_string(&path)
                        .map_err(|err| LocalizationError::ReadFile(path.clone(), err))?;
                    let file: LocaleFile = serde_json::from_str(&contents)
                        .map_err(|err| LocalizationError::ParseFile(path.clone(), err))?;
                    let display_name = file
                        .display_name
                        .clone()
                        .unwrap_or_else(|| file.locale.clone());

                    if file.locale == manager.catalogs[manager.fallback].summary.code {
                        let mut merged = default_strings_map();
                        merged.extend(file.strings.into_iter());
                        manager.catalogs[manager.fallback] = LocaleCatalog {
                            summary: LocaleSummary {
                                code: file.locale,
                                display_name,
                            },
                            strings: merged,
                        };
                    } else {
                        if manager
                            .catalogs
                            .iter()
                            .any(|catalog| catalog.summary.code == file.locale)
                        {
                            return Err(LocalizationError::DuplicateLocale(file.locale));
                        }
                        manager.catalogs.push(LocaleCatalog {
                            summary: LocaleSummary {
                                code: file.locale,
                                display_name,
                            },
                            strings: file.strings,
                        });
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Directory missing is acceptable; fallback catalog already loaded.
            }
            Err(err) => return Err(LocalizationError::ReadDir(dir.to_path_buf(), err)),
        }

        // Ensure fallback points to the requested default locale.
        if let Some(idx) = manager
            .catalogs
            .iter()
            .position(|catalog| catalog.summary.code == default_locale)
        {
            manager.fallback = idx;
        }
        manager.active = manager.fallback;

        Ok(manager)
    }

    /// Returns the index of the currently active locale.
    /// （回傳目前啟用語系的索引。）
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Exposes available locales (code + display name).
    /// （提供可用語系的代碼與顯示名稱。）
    pub fn locale_summaries(&self) -> Vec<LocaleSummary> {
        self.catalogs
            .iter()
            .map(|catalog| catalog.summary.clone())
            .collect()
    }

    /// Switches the active locale by index.
    /// （依索引切換目前啟用的語系。）
    pub fn set_active_by_index(&mut self, index: usize) -> bool {
        if index < self.catalogs.len() {
            self.active = index;
            true
        } else {
            false
        }
    }

    /// Retrieves a localized string, falling back to English when missing.
    /// （取得指定鍵的在地化字串，若缺少則回退至英文。）
    pub fn text<'a>(&'a self, key: &'a str) -> Cow<'a, str> {
        if let Some(value) = self.catalogs[self.active].strings.get(key) {
            Cow::Borrowed(value.as_str())
        } else if let Some(value) = self.catalogs[self.fallback].strings.get(key) {
            Cow::Borrowed(value.as_str())
        } else {
            Cow::Borrowed(key)
        }
    }
}

fn default_catalog() -> LocaleCatalog {
    LocaleCatalog {
        summary: LocaleSummary {
            code: DEFAULT_LOCALE_CODE.to_string(),
            display_name: DEFAULT_DISPLAY_NAME.to_string(),
        },
        strings: default_strings_map(),
    }
}

fn default_strings_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in DEFAULT_STRINGS {
        map.insert((*key).to_string(), (*value).to_string());
    }
    map
}
