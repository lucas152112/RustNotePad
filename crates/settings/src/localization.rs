use icu_locid::{Locale, ParserError as LocaleParserError};
use icu_plurals::{PluralCategory as IcuPluralCategory, PluralOperands, PluralRules};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

const DEFAULT_LOCALE_CODE: &str = "en-US";
const DEFAULT_DISPLAY_NAME: &str = "English (en-US)";

const DEFAULT_STRINGS: &[(&str, &str)] = &[
    (
        "sample.editor_preview",
        "// RustNotePad UI Preview\nfn main() {\n    let mut search_engine = SearchEngine::new(\"alpha beta gamma\");\n    let options = SearchOptions::new(\"beta\");\n    if let Some(hit) = search_engine.find(0, &options).expect(\"search\") {\n        println!(\"Found match at byte {}\", hit.start);\n    }\n}\n",
    ),
    ("toolbar.workspace_prefix", "Workspace"),
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
    ("menu.view.bottom_panels", "Bottom Panels"),
    ("menu.encoding", "Encoding"),
    ("menu.encoding.encode_ansi", "Encode in ANSI"),
    ("menu.encoding.encode_utf8", "Encode in UTF-8"),
    ("menu.encoding.encode_utf8_bom", "Encode in UTF-8-BOM"),
    ("menu.encoding.encode_ucs2_le", "Encode in UCS-2 LE BOM"),
    ("menu.encoding.encode_ucs2_be", "Encode in UCS-2 BE BOM"),
    ("menu.encoding.convert_ansi", "Convert to ANSI"),
    ("menu.encoding.convert_utf8", "Convert to UTF-8"),
    ("menu.language", "Language"),
    ("menu.language.auto_detect", "Auto-Detect"),
    ("menu.language.english", "English"),
    ("menu.language.chinese_traditional", "Traditional Chinese"),
    ("menu.language.japanese", "Japanese"),
    ("menu.language.rust", "Rust"),
    ("menu.language.json", "JSON"),
    ("menu.settings", "Settings"),
    ("menu.settings.preferences", "Preferences..."),
    ("menu.settings.style_configurator", "Style Configurator..."),
    ("menu.settings.shortcut_mapper", "Shortcut Mapper..."),
    ("menu.settings.edit_popup_menu", "Edit Popup Context Menu..."),
    ("menu.tools", "Tools"),
    ("menu.tools.md5", "MD5 > Output Hash"),
    ("menu.tools.sha256", "SHA-256 > Output Hash"),
    ("menu.tools.open_cmd", "Open Command Prompt"),
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
    ("settings.window.title", "Settings"),
    ("settings.window.sections", "Sections"),
    ("settings.tab.preferences", "Preferences"),
    ("settings.tab.style", "Style Configurator"),
    ("settings.tab.shortcuts", "Shortcut Mapper"),
    ("settings.tab.context_menu", "Popup Context Menu"),
    ("settings.preferences.heading", "Editor Preferences"),
    ("settings.preferences.autosave", "Enable autosave"),
    ("settings.preferences.autosave_interval", "Autosave interval"),
    ("settings.preferences.autosave_suffix", " min"),
    ("settings.preferences.line_numbers", "Show line numbers"),
    ("settings.preferences.highlight_line", "Highlight active line"),
    ("settings.preferences.localization_heading", "Localization"),
    ("settings.preferences.localization_label", "Select interface language"),
    (
        "settings.preferences.note",
        "Changes are saved automatically for the preview state.",
    ),
    (
        "settings.preferences.transfer_heading",
        "Import / Export Preferences",
    ),
    (
        "settings.preferences.transfer_hint",
        "Relative paths are resolved against the current workspace.",
    ),
    ("settings.preferences.export_path", "Export to path"),
    ("settings.preferences.import_path", "Import from path"),
    ("settings.preferences.export_button", "Export"),
    ("settings.preferences.import_button", "Import"),
    ("settings.style.heading", "Style Configurator"),
    ("settings.style.theme_label", "Available themes"),
    (
        "settings.style.note",
        "Select a theme to apply it immediately to the preview.",
    ),
    ("settings.style.import_heading", "Theme Import"),
    ("settings.style.import_button", "Import Theme"),
    (
        "settings.style.import_hint",
        "Supported formats: .tmTheme, .xml, .sublime-color-scheme. Imported themes are saved under workspace/.rustnotepad/themes.",
    ),
    ("settings.placeholder.heading", "Coming Soon"),
    (
        "settings.shortcuts.placeholder",
        "Shortcut mapping will be available in a future build.",
    ),
    (
        "settings.context.placeholder",
        "Popup menu customization is not yet available in this preview.",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl PluralCategory {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "zero" => Some(Self::Zero),
            "one" => Some(Self::One),
            "two" => Some(Self::Two),
            "few" => Some(Self::Few),
            "many" => Some(Self::Many),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

impl From<IcuPluralCategory> for PluralCategory {
    fn from(value: IcuPluralCategory) -> Self {
        match value {
            IcuPluralCategory::Zero => PluralCategory::Zero,
            IcuPluralCategory::One => PluralCategory::One,
            IcuPluralCategory::Two => PluralCategory::Two,
            IcuPluralCategory::Few => PluralCategory::Few,
            IcuPluralCategory::Many => PluralCategory::Many,
            IcuPluralCategory::Other => PluralCategory::Other,
        }
    }
}

#[derive(Debug, Clone)]
struct PluralMessage {
    forms: BTreeMap<PluralCategory, String>,
}

impl PluralMessage {
    fn template_for<'a>(
        &'a self,
        plural_rules: Option<&PluralRules>,
        count: Option<u64>,
    ) -> &'a str {
        let category = count
            .map(|value| select_plural_category(plural_rules, value))
            .unwrap_or(PluralCategory::Other);
        self.forms
            .get(&category)
            .or_else(|| self.forms.get(&PluralCategory::Other))
            .map(|value| value.as_str())
            .unwrap_or("")
    }
}

#[derive(Debug, Clone)]
enum Message {
    Simple(String),
    Plural(PluralMessage),
}

impl Message {
    fn render<'a>(
        &'a self,
        plural_rules: Option<&PluralRules>,
        params: &LocalizationParams<'_>,
    ) -> Cow<'a, str> {
        match self {
            Message::Simple(text) => render_template(text, params),
            Message::Plural(plural) => {
                render_template(plural.template_for(plural_rules, params.count), params)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalizationParams<'a> {
    count: Option<u64>,
    positional: &'a [&'a str],
}

impl<'a> LocalizationParams<'a> {
    pub fn new(positional: &'a [&'a str]) -> Self {
        Self {
            count: None,
            positional,
        }
    }

    pub fn with_count(positional: &'a [&'a str], count: u64) -> Self {
        Self {
            count: Some(count),
            positional,
        }
    }

    pub fn count(&self) -> Option<u64> {
        self.count
    }

    pub fn positional(&self) -> &'a [&'a str] {
        self.positional
    }
}

impl LocalizationParams<'static> {
    pub fn empty() -> Self {
        LocalizationParams {
            count: None,
            positional: &[],
        }
    }

    pub fn count_only(count: u64) -> Self {
        LocalizationParams {
            count: Some(count),
            positional: &[],
        }
    }
}

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
    #[error("locale {locale} message '{key}' is missing plural 'other' form")]
    PluralMissingOther { locale: String, key: String },
    #[error("locale {locale} message '{key}' contains invalid plural category '{category}'")]
    InvalidPluralCategory {
        locale: String,
        key: String,
        category: String,
    },
    #[error("locale {locale} message '{key}' uses unsupported type '{kind}'")]
    UnsupportedMessageType {
        locale: String,
        key: String,
        kind: String,
    },
    #[error("locale identifier '{locale}' is invalid: {error}")]
    InvalidLocaleIdentifier {
        locale: String,
        error: LocaleParserError,
    },
}

#[derive(Debug, Clone)]
pub struct LocaleSummary {
    pub code: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct LocaleCatalogStats {
    pub code: String,
    pub display_name: String,
    pub total_entries: usize,
    pub plural_entries: usize,
}

#[derive(Debug, Clone)]
struct LocaleCatalog {
    summary: LocaleSummary,
    plural_rules: Option<Arc<PluralRules>>,
    messages: HashMap<String, Message>,
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
    strings: HashMap<String, LocaleEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LocaleEntry {
    Simple(String),
    Typed(LocaleEntryTyped),
}

#[derive(Debug, Deserialize)]
struct LocaleEntryTyped {
    #[serde(rename = "type")]
    kind: String,
    #[serde(flatten)]
    forms: HashMap<String, String>,
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
        Self::load_from_dirs(std::iter::once(path), default_locale)
    }

    /// Loads locale definitions from multiple directories in order.
    /// （依序從多個目錄載入語系定義。）
    pub fn load_from_dirs<I, P>(paths: I, default_locale: &str) -> Result<Self, LocalizationError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut manager = Self::fallback();
        for path in paths {
            manager.load_directory(path.as_ref())?;
        }
        manager.apply_default_locale(default_locale);
        Ok(manager)
    }

    /// Returns the index of the currently active locale.
    /// （回傳目前啟用語系的索引。）
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Returns the locale code of the active language.
    /// （回傳目前啟用語系的代碼字串。）
    pub fn active_code(&self) -> &str {
        self.catalogs[self.active].summary.code.as_str()
    }

    /// Exposes available locales (code + display name).
    /// （提供可用語系的代碼與顯示名稱。）
    pub fn locale_summaries(&self) -> Vec<LocaleSummary> {
        self.catalogs
            .iter()
            .map(|catalog| catalog.summary.clone())
            .collect()
    }

    /// Provides per-locale statistics useful for tooling.
    /// （回傳語系統計資訊，供工具使用。）
    pub fn catalog_stats(&self) -> Vec<LocaleCatalogStats> {
        self.catalogs
            .iter()
            .map(|catalog| LocaleCatalogStats {
                code: catalog.summary.code.clone(),
                display_name: catalog.summary.display_name.clone(),
                total_entries: catalog.messages.len(),
                plural_entries: catalog
                    .messages
                    .values()
                    .filter(|message| matches!(message, Message::Plural(_)))
                    .count(),
            })
            .collect()
    }

    /// Returns missing keys for the provided locale relative to the fallback locale.
    /// （比對預設語系，回傳指定語系缺少的鍵。）
    pub fn missing_keys(&self, code: &str) -> Option<Vec<String>> {
        let fallback_keys = {
            let mut keys: Vec<_> = self.catalogs[self.fallback]
                .messages
                .keys()
                .cloned()
                .collect();
            keys.sort();
            keys
        };
        let catalog = self
            .catalogs
            .iter()
            .find(|catalog| catalog.summary.code == code)?;
        let mut missing = Vec::new();
        for key in fallback_keys {
            if !catalog.messages.contains_key(&key) {
                missing.push(key);
            }
        }
        Some(missing)
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

    /// Switches the active locale by locale code.
    /// （依語系代碼切換目前啟用的語系。）
    pub fn set_active_by_code(&mut self, code: &str) -> bool {
        if let Some(index) = self
            .catalogs
            .iter()
            .position(|catalog| catalog.summary.code == code)
        {
            self.active = index;
            true
        } else {
            false
        }
    }

    /// Retrieves a localized string, falling back to English when missing.
    /// （取得指定鍵的在地化字串，若缺少則回退至英文。）
    pub fn text<'a>(&'a self, key: &'a str) -> Cow<'a, str> {
        self.text_with_params(key, &LocalizationParams::empty())
    }

    /// Retrieves a localized string, applying parameters when provided.
    /// （取得在地化字串，必要時套用參數。）
    pub fn text_with_params<'a>(
        &'a self,
        key: &'a str,
        params: &LocalizationParams<'_>,
    ) -> Cow<'a, str> {
        if let Some(message) = self.catalogs[self.active].messages.get(key) {
            let rules = self.catalogs[self.active].plural_rules.as_deref();
            message.render(rules, params)
        } else if let Some(message) = self.catalogs[self.fallback].messages.get(key) {
            let rules = self.catalogs[self.fallback].plural_rules.as_deref();
            message.render(rules, params)
        } else {
            Cow::Borrowed(key)
        }
    }

    /// Returns the locale code configured as fallback.
    /// （回傳用作回退的語系代碼。）
    pub fn fallback_code(&self) -> &str {
        self.catalogs[self.fallback].summary.code.as_str()
    }

    /// Returns true if the specified locale provides the given key.
    /// （檢查指定語系是否存在特定鍵。）
    pub fn locale_has_key(&self, code: &str, key: &str) -> bool {
        self.catalogs
            .iter()
            .find(|catalog| catalog.summary.code == code)
            .and_then(|catalog| catalog.messages.get(key))
            .is_some()
    }

    fn load_directory(&mut self, dir: &Path) -> Result<(), LocalizationError> {
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

                    let messages = build_messages(&file.locale, file.strings)?;
                    let plural_rules = plural_rules_for(&file.locale)?;

                    if file.locale == self.catalogs[self.fallback].summary.code {
                        let mut merged = default_strings_map();
                        merged.extend(messages.into_iter());
                        self.catalogs[self.fallback] = LocaleCatalog {
                            summary: LocaleSummary {
                                code: file.locale,
                                display_name,
                            },
                            plural_rules,
                            messages: merged,
                        };
                    } else {
                        if self
                            .catalogs
                            .iter()
                            .any(|catalog| catalog.summary.code == file.locale)
                        {
                            return Err(LocalizationError::DuplicateLocale(file.locale));
                        }
                        self.catalogs.push(LocaleCatalog {
                            summary: LocaleSummary {
                                code: file.locale,
                                display_name,
                            },
                            plural_rules,
                            messages,
                        });
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Directory missing is acceptable; fallback catalog already loaded.
            }
            Err(err) => return Err(LocalizationError::ReadDir(dir.to_path_buf(), err)),
        }
        Ok(())
    }

    fn apply_default_locale(&mut self, default_locale: &str) {
        if let Some(idx) = self
            .catalogs
            .iter()
            .position(|catalog| catalog.summary.code == default_locale)
        {
            self.fallback = idx;
        }
        self.active = self.fallback;
    }
}

fn default_catalog() -> LocaleCatalog {
    let plural_rules = plural_rules_for(DEFAULT_LOCALE_CODE).ok().flatten();
    LocaleCatalog {
        summary: LocaleSummary {
            code: DEFAULT_LOCALE_CODE.to_string(),
            display_name: DEFAULT_DISPLAY_NAME.to_string(),
        },
        plural_rules,
        messages: default_strings_map(),
    }
}

fn default_strings_map() -> HashMap<String, Message> {
    let mut map = HashMap::new();
    for (key, value) in DEFAULT_STRINGS {
        map.insert((*key).to_string(), Message::Simple((*value).to_string()));
    }
    map
}

fn plural_rules_for(locale: &str) -> Result<Option<Arc<PluralRules>>, LocalizationError> {
    let parsed =
        Locale::from_str(locale).map_err(|error| LocalizationError::InvalidLocaleIdentifier {
            locale: locale.to_string(),
            error,
        })?;
    match PluralRules::try_new_cardinal(&parsed.into()) {
        Ok(rules) => Ok(Some(Arc::new(rules))),
        Err(_) => Ok(None),
    }
}

fn build_messages(
    locale: &str,
    entries: HashMap<String, LocaleEntry>,
) -> Result<HashMap<String, Message>, LocalizationError> {
    let mut messages = HashMap::new();
    for (key, entry) in entries {
        let message = match entry {
            LocaleEntry::Simple(value) => Message::Simple(value),
            LocaleEntry::Typed(typed) => {
                if typed.kind != "plural" {
                    return Err(LocalizationError::UnsupportedMessageType {
                        locale: locale.to_string(),
                        key: key.clone(),
                        kind: typed.kind,
                    });
                }
                let mut forms = BTreeMap::new();
                for (category, template) in typed.forms {
                    let parsed = PluralCategory::parse(&category).ok_or(
                        LocalizationError::InvalidPluralCategory {
                            locale: locale.to_string(),
                            key: key.clone(),
                            category,
                        },
                    )?;
                    forms.insert(parsed, template);
                }
                if !forms.contains_key(&PluralCategory::Other) {
                    return Err(LocalizationError::PluralMissingOther {
                        locale: locale.to_string(),
                        key: key.clone(),
                    });
                }
                Message::Plural(PluralMessage { forms })
            }
        };
        messages.insert(key, message);
    }
    Ok(messages)
}

fn render_template<'a>(template: &'a str, params: &LocalizationParams<'_>) -> Cow<'a, str> {
    if params.count.is_none() && (params.positional.is_empty() || !template.contains('{')) {
        return Cow::Borrowed(template);
    }

    let mut current: Cow<'a, str> = Cow::Borrowed(template);
    if let Some(count) = params.count {
        let placeholder = "{count}";
        if current.contains(placeholder) {
            let replacement = count.to_string();
            current = Cow::Owned(current.replace(placeholder, &replacement));
        }
    }

    for (idx, value) in params.positional.iter().enumerate() {
        let placeholder = format!("{{{idx}}}");
        if current.contains(&placeholder) {
            current = Cow::Owned(current.replace(&placeholder, value));
        }
    }

    current
}

fn select_plural_category(rules: Option<&PluralRules>, count: u64) -> PluralCategory {
    if let Some(rules) = rules {
        if let Ok(operands) = PluralOperands::from_str(&count.to_string()) {
            return PluralCategory::from(rules.category_for(operands));
        }
    }
    if count == 1 {
        PluralCategory::One
    } else {
        PluralCategory::Other
    }
}
