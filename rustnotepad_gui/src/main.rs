use ab_glyph::FontArc;
use chrono::Local;
use eframe::{egui, App, Frame, NativeOptions};
use egui::{
    vec2, Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, RichText,
    TextStyle,
};
use once_cell::sync::Lazy;
use rustnotepad_autocomplete::{
    CompletionEngine, CompletionItem, CompletionRequest, DocumentIndex, DocumentWordsProvider,
    LanguageDictionaryProvider, LspProvider, Snippet, SnippetProvider,
};
use rustnotepad_function_list::{FunctionKind, ParserRegistry, RegexParser, RegexRule, TextRange};
use rustnotepad_highlight::LanguageRegistry;
use rustnotepad_lsp_client::{DiagnosticSeverity, LspClient};
use rustnotepad_macros::{MacroError, MacroExecutor, MacroPlayer, MacroRecorder, MacroStore};
use rustnotepad_project::{
    AutosaveManifest, ProjectNode, ProjectNodeDraft, ProjectNodeId, ProjectNodeKind, ProjectTree,
    ProjectTreeStore, SessionCaret, SessionScroll, SessionSnapshot, SessionStore, SessionTab,
    SessionWindow, UnsavedHash,
};
use rustnotepad_runexec::{RunExecutor, RunResult, RunSpec, StdinPayload};
use rustnotepad_settings::{
    Color, LayoutConfig, LocaleSummary, LocalizationManager, PaneLayout, PaneRole, ResolvedPalette,
    SnippetStore, TabColorTag, TabView, ThemeDefinition, ThemeKind, ThemeManager,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const APP_TITLE: &str = "RustNotePad – UI Preview";
const PREVIEW_DOCUMENT_ID: &str = "preview.rs";
const PREVIEW_LANGUAGE_ID: &str = "rust";

const MAX_MACRO_MESSAGES: usize = 10;
const MAX_RUN_HISTORY: usize = 6;

static MACRO_COMMAND_OPTIONS: &[(&str, &str, &str)] = &[
    (
        "macro.command.uppercase_preview",
        "Uppercase preview text",
        "將預覽文字轉成大寫",
    ),
    (
        "macro.command.append_signature",
        "Append signature comment",
        "新增簽名註解內容",
    ),
];

struct RunLogEntry {
    title: String,
    command: String,
    working_dir: Option<PathBuf>,
    env: Vec<(String, String)>,
    cleared_env: bool,
    result: RunResult,
    stdout_text: String,
    stderr_text: String,
    timeout_ms: Option<u64>,
    kill_on_timeout: bool,
}

fn log_with_level(message: impl Into<String>, level: &str) {
    let message = message.into();
    let formatted_time = Local::now().format("%Y-%m-%d %H:%M:%S");
    let file = APP_TITLE;
    let line = format!("[{level}]{formatted_time} {file} {message}");
    eprintln!("{line}");
    if let Err(err) = append_log_line(&line) {
        let warn_line = format!("[WARN]{formatted_time} {file} Failed to write log file: {err}");
        eprintln!("{warn_line}");
    }
}

fn log_info(message: impl Into<String>) {
    log_with_level(message, "INFO");
}

fn log_warn(message: impl Into<String>) {
    log_with_level(message, "WARN");
}

fn log_error(message: impl Into<String>) {
    log_with_level(message, "ERROR");
}

fn append_log_line(line: &str) -> std::io::Result<()> {
    let mut base = std::env::current_exe()?;
    base.pop();
    base.push("logs");
    fs::create_dir_all(&base)?;
    base.push("rustnotepad_gui.log");
    let mut file = OpenOptions::new().create(true).append(true).open(base)?;
    writeln!(file, "{line}")?;
    Ok(())
}

#[derive(Clone, Copy)]
struct MenuSection {
    title_key: &'static str,
    item_keys: &'static [&'static str],
}

impl MenuSection {
    const fn new(title_key: &'static str, item_keys: &'static [&'static str]) -> Self {
        Self {
            title_key,
            item_keys,
        }
    }
}

static MENU_STRUCTURE: Lazy<Vec<MenuSection>> = Lazy::new(|| {
    vec![
        MenuSection::new(
            "menu.file",
            &[
                "menu.file.new",
                "menu.file.open",
                "menu.file.save",
                "menu.file.save_as",
                "menu.file.save_all",
                "menu.file.close",
                "menu.file.close_all",
                "menu.file.exit",
            ],
        ),
        MenuSection::new(
            "menu.edit",
            &[
                "menu.edit.undo",
                "menu.edit.redo",
                "menu.edit.cut",
                "menu.edit.copy",
                "menu.edit.paste",
                "menu.edit.delete",
                "menu.edit.select_all",
                "menu.edit.column_editor",
            ],
        ),
        MenuSection::new(
            "menu.search",
            &[
                "menu.search.find",
                "menu.search.find_next",
                "menu.search.find_previous",
                "menu.search.replace",
                "menu.search.find_in_files",
                "menu.search.bookmark",
            ],
        ),
        MenuSection::new(
            "menu.view",
            &[
                "menu.view.toggle_fullscreen",
                "menu.view.restore_zoom",
                "menu.view.document_map",
                "menu.view.function_list",
                "menu.view.project_panel",
            ],
        ),
        MenuSection::new(
            "menu.encoding",
            &[
                "menu.encoding.encode_ansi",
                "menu.encoding.encode_utf8",
                "menu.encoding.encode_utf8_bom",
                "menu.encoding.encode_ucs2_le",
                "menu.encoding.encode_ucs2_be",
                "menu.encoding.convert_ansi",
                "menu.encoding.convert_utf8",
            ],
        ),
        MenuSection::new(
            "menu.language",
            &[
                "menu.language.auto_detect",
                "menu.language.english",
                "menu.language.chinese_traditional",
                "menu.language.japanese",
                "menu.language.rust",
                "menu.language.json",
            ],
        ),
        MenuSection::new(
            "menu.settings",
            &[
                "menu.settings.preferences",
                "menu.settings.style_configurator",
                "menu.settings.shortcut_mapper",
                "menu.settings.edit_popup_menu",
            ],
        ),
        MenuSection::new(
            "menu.tools",
            &["menu.tools.md5", "menu.tools.sha256", "menu.tools.open_cmd"],
        ),
        MenuSection::new(
            "menu.macro",
            &[
                "menu.macro.start_recording",
                "menu.macro.stop_recording",
                "menu.macro.playback",
            ],
        ),
        MenuSection::new(
            "menu.run",
            &[
                "menu.run.run",
                "menu.run.launch_chrome",
                "menu.run.launch_firefox",
            ],
        ),
        MenuSection::new(
            "menu.plugins",
            &["menu.plugins.admin", "menu.plugins.open_folder"],
        ),
        MenuSection::new(
            "menu.window",
            &[
                "menu.window.duplicate",
                "menu.window.clone_other_view",
                "menu.window.move_other_view",
            ],
        ),
        MenuSection::new(
            "menu.help",
            &[
                "menu.help.user_manual",
                "menu.help.debug_info",
                "menu.help.about",
            ],
        ),
    ]
});

fn build_sample_project_tree() -> ProjectTree {
    fn add_folder(
        tree: ProjectTree,
        parent: ProjectNodeId,
        name: &str,
        path: Option<&str>,
    ) -> (ProjectTree, ProjectNodeId) {
        let kind = ProjectNodeKind::Folder {
            path: path.map(PathBuf::from),
            filters: Vec::new(),
        };
        let draft = ProjectNodeDraft::new(name, kind);
        let (tree, diff) = tree
            .add_child(parent, draft)
            .expect("sample tree folder insertion failed");
        let new_id = *diff
            .added
            .first()
            .expect("sample tree folder id unavailable");
        (tree, new_id)
    }

    fn add_file(tree: ProjectTree, parent: ProjectNodeId, name: &str, path: &str) -> ProjectTree {
        let draft = ProjectNodeDraft::new(
            name,
            ProjectNodeKind::File {
                path: PathBuf::from(path),
            },
        );
        let (tree, _) = tree
            .add_child(parent, draft)
            .expect("sample tree file insertion failed");
        tree
    }

    let tree = ProjectTree::empty("workspace", None);
    let root_id = tree.root_id();

    let (tree, crates_id) = add_folder(tree, root_id, "crates", Some("crates"));
    let (tree, core_id) = add_folder(tree, crates_id, "core", Some("crates/core"));
    let tree = add_file(tree, core_id, "document.rs", "crates/core/src/document.rs");
    let tree = add_file(tree, core_id, "editor.rs", "crates/core/src/editor.rs");
    let tree = add_file(
        tree,
        core_id,
        "search_session.rs",
        "crates/core/src/search_session.rs",
    );

    let (tree, search_id) = add_folder(tree, crates_id, "search", Some("crates/search"));
    let tree = add_file(tree, search_id, "Cargo.toml", "crates/search/Cargo.toml");
    let tree = add_file(tree, search_id, "src/lib.rs", "crates/search/src/lib.rs");

    let (tree, docs_id) = add_folder(tree, root_id, "docs", Some("docs"));
    let (tree, feature_id) =
        add_folder(tree, docs_id, "feature_parity", Some("docs/feature_parity"));
    let tree = add_file(
        tree,
        feature_id,
        "03-search-replace/design.md",
        "docs/feature_parity/03-search-replace/design.md",
    );
    let tree = add_file(
        tree,
        feature_id,
        "04-view-interface/design.md",
        "docs/feature_parity/04-view-interface/design.md",
    );

    let (tree, tests_id) = add_folder(tree, root_id, "tests", Some("tests"));
    let tree = add_file(
        tree,
        tests_id,
        "search_workflow.rs",
        "tests/search_workflow.rs",
    );
    let tree = add_file(
        tree,
        tests_id,
        "search_large_workspace.rs",
        "tests/search_large_workspace.rs",
    );

    tree
}

fn default_workspace_root() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(appdata) = env::var_os("APPDATA") {
            return PathBuf::from(appdata)
                .join("RustNotePad")
                .join("Workspaces");
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg_config) = env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg_config)
                .join("rustnotepad")
                .join("workspaces");
        }
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("rustnotepad")
                .join("workspaces");
        }
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("rustnotepad_workspaces")
}

struct StatusBarState {
    line: usize,
    column: usize,
    lines: usize,
    selection: usize,
    encoding: &'static str,
    eol: &'static str,
    mode: &'static str,
    document_language: String,
    ui_language: String,
    theme: String,
}

impl StatusBarState {
    fn new(layout: &LayoutConfig, theme: &str, locale: &str) -> Self {
        let mut state = Self {
            line: 1,
            column: 1,
            lines: 1,
            selection: 0,
            encoding: "UTF-8",
            eol: "LF",
            mode: "INS",
            document_language: "Plain Text".into(),
            ui_language: locale.to_string(),
            theme: theme.to_string(),
        };
        state.refresh_from_layout(layout);
        state
    }

    fn refresh_from_layout(&mut self, layout: &LayoutConfig) {
        let language = layout
            .active_tab(PaneRole::Primary)
            .or_else(|| layout.active_tab(PaneRole::Secondary))
            .and_then(|tab| tab.language.clone())
            .unwrap_or_else(|| "Plain Text".into());
        self.document_language = language;
    }

    fn refresh_cursor(&mut self, contents: &str) {
        let lines_iter = contents.lines();
        let total_lines = lines_iter.clone().count().max(1);
        let last_line = lines_iter.last().unwrap_or_default();
        self.line = total_lines;
        self.lines = total_lines;
        self.column = last_line.chars().count().saturating_add(1);
    }

    fn set_theme(&mut self, theme_name: &str) {
        self.theme = theme_name.to_string();
    }

    fn set_locale(&mut self, locale: &str) {
        self.ui_language = locale.to_string();
    }

    fn set_document_language(&mut self, language: String) {
        self.document_language = language;
    }
}

struct RustNotePadApp {
    layout: LayoutConfig,
    editor_preview: String,
    bottom_tab_index: usize,
    selected_locale: usize,
    localization: LocalizationManager,
    sample_editor_content: String,
    show_settings_window: bool,
    active_settings_page: SettingsPage,
    theme_manager: ThemeManager,
    palette: ResolvedPalette,
    status: StatusBarState,
    pending_theme_refresh: bool,
    fonts_installed: bool,
    cjk_font_available: bool,
    font_warning: Option<String>,
    highlight_registry: LanguageRegistry,
    function_registry: ParserRegistry,
    project_tree: ProjectTree,
    project_tree_store: ProjectTreeStore,
    session_store: SessionStore,
    workspace_root: PathBuf,
    document_index: Arc<DocumentIndex>,
    lsp_client: Arc<LspClient>,
    autocomplete_engine: CompletionEngine,
    completion_prefix: String,
    completion_results: Vec<CompletionItem>,
    current_document_id: String,
    current_language_id: String,
    preferences: PreferencesState,
    macro_recorder: MacroRecorder,
    macro_store: MacroStore,
    macro_messages: VecDeque<String>,
    macro_pending_name: String,
    macro_input_buffer: String,
    macro_selected_command: usize,
    macro_repeat_count: usize,
    macro_live_events: Vec<String>,
    selected_macro: Option<String>,
    run_history: VecDeque<RunLogEntry>,
    run_last_error: Option<String>,
    run_timeout_enabled: bool,
    run_timeout_secs: u64,
    run_kill_on_timeout: bool,
    show_open_dialog: bool,
    open_dialog_path: String,
    open_dialog_error: Option<String>,
    show_save_as_dialog: bool,
    save_dialog_path: String,
    save_dialog_error: Option<String>,
    current_document_path: Option<PathBuf>,
    document_dirty: bool,
    untitled_counter: usize,
    pending_exit: bool,
}

impl Default for RustNotePadApp {
    fn default() -> Self {
        let localization = LocalizationManager::load_from_dir("assets/langs", "en-US")
            .unwrap_or_else(|_| LocalizationManager::fallback());
        let locale_summaries = localization.locale_summaries();
        let selected_locale = localization.active_index();
        let locale_display = locale_summaries
            .get(selected_locale)
            .map(|summary| summary.display_name.clone())
            .unwrap_or_else(|| "English (en-US)".to_string());
        let sample_editor_content = localization.text("sample.editor_preview").into_owned();

        let theme_manager = ThemeManager::load_from_dir("assets/themes").unwrap_or_else(|_| {
            ThemeManager::new(vec![
                ThemeDefinition::builtin_dark(),
                ThemeDefinition::builtin_light(),
            ])
            .expect("built-in themes")
        });
        let palette = theme_manager.active_palette().clone();
        let layout = LayoutConfig::default();
        let active_panel_index = layout
            .bottom_dock
            .active_panel
            .as_ref()
            .and_then(|active| {
                layout
                    .bottom_dock
                    .visible_panels
                    .iter()
                    .position(|panel| panel == active)
            })
            .unwrap_or(0);

        let mut status =
            StatusBarState::new(&layout, &theme_manager.active_theme().name, &locale_display);

        let highlight_registry = LanguageRegistry::with_defaults();
        let function_registry = build_function_registry();
        let mut initial_editor_content = sample_editor_content.clone();
        let mut initial_document_id = PREVIEW_DOCUMENT_ID.to_string();
        let mut initial_language_id = PREVIEW_LANGUAGE_ID.to_string();

        let workspace_root = default_workspace_root();
        if let Err(err) = fs::create_dir_all(&workspace_root) {
            log_warn(format!(
                "Workspace directory creation failed at {}: {err}",
                workspace_root.display()
            ));
        }
        let project_tree_store = ProjectTreeStore::new(workspace_root.join("project_tree.json"));
        let project_tree = match project_tree_store.load() {
            Ok(Some(tree)) => {
                log_info(format!(
                    "Loaded project tree from {}",
                    project_tree_store.path().display()
                ));
                tree
            }
            Ok(None) => {
                let tree = build_sample_project_tree();
                match project_tree_store.save(&tree) {
                    Ok(()) => log_info(format!(
                        "Seeded sample project tree at {}",
                        project_tree_store.path().display()
                    )),
                    Err(err) => log_warn(format!("Failed to seed project tree: {err}")),
                }
                tree
            }
            Err(err) => {
                log_error(format!("Failed to load project tree: {err}"));
                build_sample_project_tree()
            }
        };

        let session_dir = workspace_root.join("sessions");
        if let Err(err) = fs::create_dir_all(&session_dir) {
            log_warn(format!(
                "Session directory creation failed at {}: {err}",
                session_dir.display()
            ));
        }
        let session_store = SessionStore::new(
            session_dir.join("session.json"),
            session_dir.join("autosave"),
        );
        if let Ok(Some(snapshot)) = session_store.load() {
            if let Some(window) = snapshot.windows.first() {
                if let Some(tab) = window.tabs.first() {
                    if let Some(path) = tab.path.as_ref().and_then(|p| p.to_str()) {
                        initial_document_id = path.to_string();
                        initial_language_id = language_id_from_path(path).to_string();
                    } else if let Some(name) = &tab.display_name {
                        initial_document_id = name.clone();
                    }
                    if let Some(hash) = &tab.unsaved_hash {
                        match session_store.autosave().read_contents(hash) {
                            Ok(bytes) if !bytes.is_empty() => {
                                if let Ok(text) = String::from_utf8(bytes) {
                                    initial_editor_content = text;
                                }
                            }
                            Ok(_) => {}
                            Err(err) => log_warn(format!("Failed to read autosave buffer: {err}")),
                        }
                    }
                }
            }
            log_info("Session snapshot restored");
        }

        let document_index = Arc::new(DocumentIndex::new());
        document_index.update_document(&initial_document_id, &initial_editor_content);
        let lsp_client = Arc::new(LspClient::new());
        let mut autocomplete_engine = CompletionEngine::new();
        autocomplete_engine.register_provider(
            "lsp",
            0,
            LspProvider::new(lsp_client.clone()).with_max_items(24),
        );

        let snippet_store = SnippetStore::builtin();
        let snippet_items: Vec<Snippet> = snippet_store
            .entries()
            .iter()
            .map(|definition| {
                let mut snippet = Snippet::new(definition.trigger.clone(), definition.body.clone());
                if let Some(description) = &definition.description {
                    snippet = snippet.with_description(description.clone());
                }
                if let Some(language) = &definition.language {
                    snippet = snippet.with_language(language.clone());
                }
                snippet
            })
            .collect();
        autocomplete_engine.register_provider(
            "snippets",
            2,
            SnippetProvider::new(snippet_items).with_max_items(32),
        );

        let mut dictionary_provider = LanguageDictionaryProvider::new().with_max_items(48);
        let mut fallback_keywords = Vec::new();
        for language_id in ["rust", "json"] {
            if let Some(language) = highlight_registry.get(language_id) {
                dictionary_provider.register_language(
                    language_id,
                    language.keywords().to_vec(),
                    language.case_sensitive,
                );
                fallback_keywords.extend(language.keywords().iter().cloned());
            }
        }
        if !fallback_keywords.is_empty() {
            dictionary_provider.register_fallback(fallback_keywords);
        }
        autocomplete_engine.register_provider("dictionary", 4, dictionary_provider);
        autocomplete_engine.register_provider(
            "document_words",
            6,
            DocumentWordsProvider::new(document_index.clone())
                .with_prefix_minimum(1)
                .with_max_items(40),
        );
        let completion_prefix = "ma".to_string();
        status.set_document_language(
            localization
                .text(language_display_key(&initial_language_id))
                .into_owned(),
        );
        status.set_locale(&locale_display);

        let default_macro_name = "Macro 1".to_string();
        let mut app = Self {
            layout,
            editor_preview: initial_editor_content.clone(),
            bottom_tab_index: active_panel_index,
            selected_locale,
            localization,
            sample_editor_content,
            show_settings_window: false,
            active_settings_page: SettingsPage::Preferences,
            theme_manager,
            palette,
            status,
            pending_theme_refresh: true,
            fonts_installed: false,
            cjk_font_available: false,
            font_warning: None,
            highlight_registry,
            function_registry,
            project_tree,
            project_tree_store,
            session_store,
            workspace_root,
            document_index,
            lsp_client,
            autocomplete_engine,
            completion_prefix,
            completion_results: Vec::new(),
            current_document_id: initial_document_id.clone(),
            current_language_id: initial_language_id.clone(),
            preferences: PreferencesState::default(),
            macro_recorder: MacroRecorder::new(),
            macro_store: MacroStore::new(),
            macro_messages: VecDeque::new(),
            macro_pending_name: default_macro_name,
            macro_input_buffer: String::new(),
            macro_selected_command: 0,
            macro_repeat_count: 1,
            macro_live_events: Vec::new(),
            selected_macro: None,
            run_history: VecDeque::new(),
            run_last_error: None,
            run_timeout_enabled: true,
            run_timeout_secs: 2,
            run_kill_on_timeout: true,
            show_open_dialog: false,
            open_dialog_path: String::new(),
            open_dialog_error: None,
            show_save_as_dialog: false,
            save_dialog_path: String::new(),
            save_dialog_error: None,
            current_document_path: if Path::new(&initial_document_id).exists() {
                Some(PathBuf::from(&initial_document_id))
            } else {
                None
            },
            document_dirty: false,
            untitled_counter: 1,
            pending_exit: false,
        };
        app.status.refresh_cursor(&app.editor_preview);
        app.refresh_completions();

        if let Ok(locale_override) = env::var("RUSTNOTEPAD_LOCALE") {
            let summaries = app.localization.locale_summaries();
            if let Some(idx) = summaries
                .iter()
                .position(|summary| summary.code == locale_override)
            {
                app.apply_locale_change(idx, &summaries);
            }
        }

        app
    }
}

fn build_function_registry() -> ParserRegistry {
    let mut registry = ParserRegistry::new();
    let rust_rules = vec![
        RegexRule::new(
            r"(?m)^\s*(?:pub\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
            FunctionKind::Function,
        )
        .expect("rust function rule"),
        RegexRule::new(
            r"(?m)^\s*(?:pub\s+)?struct\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
            FunctionKind::Struct,
        )
        .expect("rust struct rule"),
        RegexRule::new(
            r"(?m)^\s*(?:pub\s+)?enum\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
            FunctionKind::Enum,
        )
        .expect("rust enum rule"),
        RegexRule::new(
            r"(?m)^\s*(?:pub\s+)?impl(?:<[^>]+>)?\s+(?P<name>[A-Za-z_][A-Za-z0-9_:<>]*)",
            FunctionKind::Region,
        )
        .expect("rust impl rule"),
    ];
    registry.register_parser(PREVIEW_LANGUAGE_ID, Box::new(RegexParser::new(rust_rules)));
    registry
}

impl RustNotePadApp {
    fn apply_theme_if_needed(&mut self, ctx: &egui::Context) {
        if self.pending_theme_refresh {
            self.apply_active_theme(ctx);
            self.pending_theme_refresh = false;
        }
    }

    fn refresh_completions(&mut self) {
        let mut request = CompletionRequest::new(
            Some(self.current_document_id.clone()),
            self.completion_prefix.clone(),
        )
        .with_max_items(16)
        .with_language(Some(self.current_language_id.clone()));
        if let Some(language) = self
            .highlight_registry
            .get(self.current_language_id.as_str())
        {
            request = request.with_case_sensitive(language.case_sensitive);
        }
        let result = self.autocomplete_engine.request(request);
        self.completion_results = result.items;
    }

    fn push_macro_log(&mut self, message: impl Into<String>) {
        if self.macro_messages.len() >= MAX_MACRO_MESSAGES {
            self.macro_messages.pop_front();
        }
        self.macro_messages.push_back(message.into());
    }

    fn push_macro_log_localized(&mut self, en: &str, zh: &str) {
        let message = self.localized(en, zh);
        self.push_macro_log(message);
    }

    fn push_macro_log_localized_owned(&mut self, en: String, zh: String) {
        let message = self.localized_owned(en, zh);
        self.push_macro_log(message);
    }

    fn next_macro_name(&self) -> String {
        let mut index = self.macro_store.iter().count() + 1;
        loop {
            let candidate = format!("Macro {}", index);
            if self.macro_store.get(&candidate).is_none() {
                return candidate;
            }
            index += 1;
        }
    }

    fn after_macro_edit(&mut self) {
        self.document_index
            .update_document(&self.current_document_id, &self.editor_preview);
        self.status.refresh_cursor(&self.editor_preview);
        self.refresh_completions();
        self.mark_document_dirty();
    }

    fn start_macro_recording(&mut self) {
        match self.macro_recorder.start() {
            Ok(_) => {
                self.macro_pending_name = self.next_macro_name();
                self.macro_live_events.clear();
                self.macro_input_buffer.clear();
                self.push_macro_log_localized("Recording started", "已開始錄製");
            }
            Err(err) => {
                self.push_macro_log_localized_owned(
                    format!("Cannot start recording: {err}"),
                    format!("無法開始錄製：{err}"),
                );
            }
        }
    }

    fn cancel_macro_recording(&mut self) {
        if self.macro_recorder.is_recording() {
            self.macro_recorder.cancel();
            self.macro_live_events.clear();
            self.macro_input_buffer.clear();
            self.push_macro_log_localized("Recording cancelled", "已取消錄製");
        } else {
            self.push_macro_log_localized("Recorder idle", "錄製器目前未啟動");
        }
    }

    fn stop_macro_recording(&mut self) {
        if !self.macro_recorder.is_recording() {
            self.push_macro_log_localized("Recorder idle", "錄製器目前未啟動");
            return;
        }

        let name = if self.macro_pending_name.trim().is_empty() {
            self.next_macro_name()
        } else {
            self.macro_pending_name.trim().to_string()
        };
        match self.macro_recorder.finish(name.clone(), None) {
            Ok(macro_def) => {
                if macro_def.is_empty() {
                    self.push_macro_log_localized("Cannot save empty macro", "無法儲存空巨集");
                } else if let Err(err) = self.macro_store.insert(macro_def) {
                    match err {
                        MacroError::DuplicateMacroName(existing) => {
                            self.push_macro_log_localized_owned(
                                format!("Macro '{existing}' already exists"),
                                format!("巨集「{existing}」已存在"),
                            );
                        }
                        other => {
                            self.push_macro_log_localized_owned(
                                format!("Failed to save macro: {other}"),
                                format!("無法儲存巨集：{other}"),
                            );
                        }
                    }
                } else {
                    self.selected_macro = Some(name.clone());
                    self.push_macro_log_localized_owned(
                        format!("Saved macro '{name}'"),
                        format!("已儲存巨集「{name}」"),
                    );
                    self.macro_pending_name = self.next_macro_name();
                }
            }
            Err(err) => {
                self.push_macro_log_localized_owned(
                    format!("Failed to finish recording: {err}"),
                    format!("無法完成錄製：{err}"),
                );
            }
        }
        self.macro_live_events.clear();
        self.macro_input_buffer.clear();
    }

    fn add_macro_text_event(&mut self) {
        if !self.macro_recorder.is_recording() {
            self.push_macro_log_localized("Recorder idle", "錄製器目前未啟動");
            return;
        }
        let text = self.macro_input_buffer.trim();
        if text.is_empty() {
            self.push_macro_log_localized("Enter text before adding", "請先輸入文字再加入");
            return;
        }
        if let Err(err) = self.macro_recorder.record_text(text.to_string()) {
            self.push_macro_log_localized_owned(
                format!("Failed to record text: {err}"),
                format!("無法記錄文字：{err}"),
            );
            return;
        }
        let preview = if text.len() > 18 {
            format!("{}…", &text[..18])
        } else {
            text.to_string()
        };
        self.macro_live_events.push(format!("text:\"{preview}\""));
        self.push_macro_log_localized_owned(
            format!("Recorded text \"{preview}\""),
            format!("已記錄文字「{preview}」"),
        );
        self.macro_input_buffer.clear();
    }

    fn add_macro_command_event(&mut self) {
        if !self.macro_recorder.is_recording() {
            self.push_macro_log_localized("Recorder idle", "錄製器目前未啟動");
            return;
        }
        if MACRO_COMMAND_OPTIONS.is_empty() {
            self.push_macro_log_localized("No commands registered", "尚未註冊指令");
            return;
        }
        let max_index = MACRO_COMMAND_OPTIONS.len().saturating_sub(1);
        if self.macro_selected_command > max_index {
            self.macro_selected_command = max_index;
        }
        let (command_id, label_en, label_zh) = MACRO_COMMAND_OPTIONS[self.macro_selected_command];
        let label = self.localized(label_en, label_zh);
        if let Err(err) = self.macro_recorder.record_command(command_id.to_string()) {
            self.push_macro_log_localized_owned(
                format!("Failed to record command: {err}"),
                format!("無法記錄指令：{err}"),
            );
            return;
        }
        self.macro_live_events.push(format!("command:{command_id}"));
        self.push_macro_log_localized_owned(
            format!("Recorded command '{label}'"),
            format!("已記錄指令「{label}」"),
        );
    }

    fn play_selected_macro(&mut self) {
        let Some(selected) = self.selected_macro.clone() else {
            self.push_macro_log_localized("Select a macro first", "請先選擇巨集");
            return;
        };
        let Some(macro_def) = self.macro_store.get(&selected).cloned() else {
            self.push_macro_log_localized_owned(
                format!("Macro '{selected}' not found"),
                format!("找不到巨集「{selected}」"),
            );
            return;
        };
        let repeat = NonZeroUsize::new(self.macro_repeat_count.max(1))
            .unwrap_or(NonZeroUsize::new(1).unwrap());
        let mut executor = AppMacroExecutor { app: self };
        match MacroPlayer::play(&macro_def, repeat, &mut executor) {
            Ok(_) => {
                let count = repeat.get();
                self.push_macro_log_localized_owned(
                    format!("Played macro '{selected}' ×{count}"),
                    format!("已播放巨集「{selected}」×{count}"),
                );
            }
            Err(err) => {
                self.push_macro_log_localized_owned(
                    format!("Playback failed: {err}"),
                    format!("播放失敗：{err}"),
                );
            }
        }
    }

    fn delete_selected_macro(&mut self) {
        let Some(selected) = self.selected_macro.clone() else {
            self.push_macro_log_localized("Select a macro first", "請先選擇巨集");
            return;
        };
        match self.macro_store.remove(&selected) {
            Ok(_) => {
                self.push_macro_log_localized_owned(
                    format!("Deleted macro '{selected}'"),
                    format!("已刪除巨集「{selected}」"),
                );
                self.selected_macro = None;
            }
            Err(err) => {
                self.push_macro_log_localized_owned(
                    format!("Failed to delete macro: {err}"),
                    format!("無法刪除巨集：{err}"),
                );
            }
        }
    }

    fn handle_file_command(&mut self, item_key: &str) {
        match item_key {
            "menu.file.new" => self.new_document(),
            "menu.file.open" => self.prompt_open_document(),
            "menu.file.save" => self.save_current_document(),
            "menu.file.save_as" => self.prompt_save_as(),
            "menu.file.save_all" => self.save_all_documents(),
            "menu.file.close" => self.close_active_document(),
            "menu.file.close_all" => self.close_all_documents(),
            "menu.file.exit" => {
                self.pending_exit = true;
            }
            _ => {
                log_warn(self.localized_owned(
                    format!("Unsupported file command {item_key}"),
                    format!("未支援的檔案指令 {item_key}"),
                ));
            }
        }
    }

    fn handle_macro_command(&mut self, item_key: &str) {
        match item_key {
            "menu.macro.start_recording" => self.start_macro_recording(),
            "menu.macro.stop_recording" => self.stop_macro_recording(),
            "menu.macro.playback" => self.play_selected_macro(),
            _ => {
                self.push_macro_log_localized_owned(
                    format!("Unsupported macro command {item_key}"),
                    format!("未支援的巨集指令 {item_key}"),
                );
            }
        }
    }

    fn handle_run_command(&mut self, item_key: &str) {
        if let Some((title, spec)) = self.build_run_spec(item_key) {
            self.execute_run_spec(title, spec);
        } else {
            self.run_last_error = Some(self.localized_owned(
                format!("Unsupported run command {item_key}"),
                format!("未支援的執行指令 {item_key}"),
            ));
        }
    }

    fn new_document(&mut self) {
        let index = self.untitled_counter;
        self.untitled_counter += 1;
        let tab_id = format!("untitled-{index}");
        let title = self.localized_owned(format!("Untitled {index}"), format!("未命名 {index}"));

        let language_name = self.language_display_name("plaintext");
        if let Some(pane) = self
            .layout
            .panes
            .iter_mut()
            .find(|pane| pane.role == PaneRole::Primary)
        {
            let mut tab = TabView::new(tab_id.clone(), title.clone());
            tab.language = Some(language_name.clone());
            pane.tabs.push(tab);
            pane.active = Some(tab_id.clone());
        }

        self.current_document_id = tab_id.clone();
        self.current_document_path = None;
        self.current_language_id = "plaintext".into();
        self.editor_preview.clear();
        self.document_dirty = false;
        self.set_tab_dirty_state(&tab_id, false);
        self.document_index
            .update_document(&self.current_document_id, &self.editor_preview);
        self.status.refresh_cursor(&self.editor_preview);
        self.status.set_document_language(language_name);
        self.refresh_completions();
        self.status.refresh_from_layout(&self.layout);
        log_info(self.localized_owned(
            format!("Created new document {tab_id}"),
            format!("已建立新文件 {title}"),
        ));
    }

    fn prompt_open_document(&mut self) {
        self.open_dialog_error = None;
        if self.open_dialog_path.is_empty() {
            if let Some(path) = self.current_document_path.as_ref() {
                self.open_dialog_path = path.to_string_lossy().into_owned();
            }
        }
        self.show_open_dialog = true;
    }

    fn prompt_save_as(&mut self) {
        self.save_dialog_error = None;
        if let Some(path) = self.current_document_path.as_ref() {
            self.save_dialog_path = path.to_string_lossy().into_owned();
        } else if self.save_dialog_path.is_empty() {
            self.save_dialog_path = format!("{}.txt", self.current_document_id);
        }
        self.show_save_as_dialog = true;
    }

    fn save_current_document(&mut self) {
        if let Some(path) = self.current_document_path.clone() {
            match self.write_document_to(&path) {
                Ok(()) => {
                    log_info(self.localized_owned(
                        format!("Saved {}", path.display()),
                        format!("已儲存 {}", path.display()),
                    ));
                }
                Err(err) => {
                    log_error(self.localized_owned(
                        format!("Save failed: {err}"),
                        format!("儲存失敗：{err}"),
                    ));
                }
            }
        } else {
            self.prompt_save_as();
        }
    }

    fn save_all_documents(&mut self) {
        self.save_current_document();
        log_info(self.localized(
            "Save All currently writes only the active document",
            "「全部儲存」目前僅處理作用中文件",
        ));
    }

    fn close_active_document(&mut self) {
        let current_id = self.current_document_id.clone();
        let should_close = self.layout.panes.iter().any(|pane| {
            pane.role == PaneRole::Primary && pane.active.as_deref() == Some(current_id.as_str())
        });
        if should_close {
            self.close_tab(PaneRole::Primary, &current_id);
        }
    }

    fn close_all_documents(&mut self) {
        let tab_ids: Vec<String> = self
            .layout
            .panes
            .iter()
            .filter(|pane| pane.role == PaneRole::Primary)
            .flat_map(|pane| {
                pane.tabs
                    .iter()
                    .filter(|tab| !tab.is_locked && !tab.is_pinned)
                    .map(|tab| tab.id.clone())
            })
            .collect();
        for tab_id in tab_ids {
            self.close_tab(PaneRole::Primary, &tab_id);
        }
    }

    fn build_run_spec(&self, item_key: &str) -> Option<(String, RunSpec)> {
        match item_key {
            "menu.run.run" => {
                let spec = RunSpec::new("bash")
                    .with_args([
                        "-lc",
                        "read line; echo \"Macro:\" \"$line\"; echo \"Workspace:\" \"$RUN_WORKSPACE\"",
                    ])
                    .with_env("RUN_WORKSPACE", "RustNotePad")
                    .with_stdin(StdinPayload::Text("preview-input\n".into()))
                    .with_working_dir("crates/macros");
                Some((
                    self.localized("Run Macro Preview", "範例執行"),
                    self.apply_run_defaults(spec),
                ))
            }
            "menu.run.launch_chrome" => {
                let spec = RunSpec::new("bash")
                    .with_args(["-lc", "echo \"Launching Chrome to $RUN_URL\""])
                    .with_env("RUN_URL", "https://www.rust-lang.org/");
                Some((
                    self.localized("Launch Chrome preset", "模擬啟動 Chrome"),
                    self.apply_run_defaults(spec),
                ))
            }
            "menu.run.launch_firefox" => {
                let spec = RunSpec::new("bash")
                    .with_args(["-lc", "echo \"Launching Firefox to $RUN_URL\""])
                    .clear_env()
                    .with_env("RUN_URL", "https://notepad-plus-plus.org/");
                Some((
                    self.localized("Launch Firefox preset", "模擬啟動 Firefox"),
                    self.apply_run_defaults(spec),
                ))
            }
            _ => None,
        }
    }

    fn execute_run_spec(&mut self, title: String, spec: RunSpec) {
        let command = Self::format_run_command(&spec);
        let working_dir = spec.working_dir.clone();
        let env = spec
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        let cleared = spec.clear_env;
        let timeout_desc = spec
            .timeout_ms
            .map(|ms| format!("{:.2}s", (ms as f64) / 1000.0))
            .unwrap_or_else(|| "disabled".to_string());
        let kill_desc_en = if spec.kill_on_timeout {
            "kill on timeout"
        } else {
            "wait on timeout"
        };
        let kill_desc_zh = if spec.kill_on_timeout {
            "逾時終止"
        } else {
            "逾時不中止"
        };
        log_info(self.localized_owned(
            format!(
                "Executing run preset '{title}' -> {command} (timeout {timeout_desc}, {kill_desc_en})"
            ),
            format!(
                "執行預設「{title}」，指令：{command}（逾時設定：{timeout_desc}，{kill_desc_zh}）"
            ),
        ));
        match RunExecutor::execute(&spec) {
            Ok(result) => {
                let stdout_text = String::from_utf8_lossy(&result.stdout).into_owned();
                let stderr_text = String::from_utf8_lossy(&result.stderr).into_owned();
                let exit_desc = result
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "signal".to_string());
                let timed_out = result.timed_out;
                let duration_ms = result.duration_ms;
                let timed_label_en = if timed_out { "timed out" } else { "completed" };
                let timed_label_zh = if timed_out { "逾時" } else { "完成" };
                let log_title = title.clone();
                let entry = RunLogEntry {
                    title,
                    command,
                    working_dir,
                    env,
                    cleared_env: cleared,
                    result,
                    stdout_text,
                    stderr_text,
                    timeout_ms: spec.timeout_ms,
                    kill_on_timeout: spec.kill_on_timeout,
                };
                self.push_run_history(entry);
                self.run_last_error = None;
                log_info(self.localized_owned(
                    format!(
                        "Run preset '{log_title}' {timed_label_en} with exit {exit_desc} (duration {duration_ms} ms)"
                    ),
                    format!(
                        "執行預設「{log_title}」{timed_label_zh}，結束碼 {exit_desc}，耗時 {duration_ms} 毫秒"
                    ),
                ));
            }
            Err(err) => {
                self.run_last_error = Some(self.localized_owned(
                    format!("Execution failed: {err}"),
                    format!("執行失敗：{err}"),
                ));
                log_error(self.localized_owned(
                    format!("Run preset '{title}' failed: {err}"),
                    format!("執行預設「{title}」失敗：{err}"),
                ));
            }
        }
    }

    fn format_run_command(spec: &RunSpec) -> String {
        let mut parts = Vec::new();
        parts.push(spec.program.clone());
        parts.extend(spec.args.iter().cloned());
        parts.join(" ")
    }

    fn push_run_history(&mut self, entry: RunLogEntry) {
        if self.run_history.len() >= MAX_RUN_HISTORY {
            self.run_history.pop_back();
        }
        self.run_history.push_front(entry);
    }

    fn text<'a>(&'a self, key: &'a str) -> Cow<'a, str> {
        self.localization.text(key)
    }

    fn attempt_open_dialog(&mut self) -> bool {
        let trimmed = self.open_dialog_path.trim().to_string();
        if trimmed.is_empty() {
            self.open_dialog_error =
                Some(self.localized("Please enter a file path", "請輸入檔案路徑"));
            return false;
        }

        let path = PathBuf::from(trimmed.as_str());
        if !path.is_file() {
            self.open_dialog_error = Some(self.localized_owned(
                format!("File not found: {}", path.display()),
                format!("找不到檔案：{}", path.display()),
            ));
            return false;
        }

        let display = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(trimmed.as_str())
            .to_string();
        self.open_document(trimmed.as_str(), &display);
        self.open_dialog_error = None;
        self.open_dialog_path.clear();
        log_info(self.localized_owned(
            format!("Opened {}", path.display()),
            format!("已開啟 {}", path.display()),
        ));
        true
    }

    fn attempt_save_dialog(&mut self) -> bool {
        let trimmed = self.save_dialog_path.trim().to_string();
        if trimmed.is_empty() {
            self.save_dialog_error =
                Some(self.localized("Please enter a destination path", "請輸入儲存路徑"));
            return false;
        }

        let path = PathBuf::from(trimmed.as_str());
        match self.write_document_to(&path) {
            Ok(()) => {
                self.save_dialog_error = None;
                log_info(self.localized_owned(
                    format!("Saved {}", path.display()),
                    format!("已儲存 {}", path.display()),
                ));
                true
            }
            Err(err) => {
                self.save_dialog_error = Some(
                    self.localized_owned(format!("Save failed: {err}"), format!("儲存失敗：{err}")),
                );
                false
            }
        }
    }

    fn write_document_to(&mut self, target: &Path) -> Result<(), String> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::write(target, &self.editor_preview).map_err(|err| err.to_string())?;

        let old_id = self.current_document_id.clone();
        let new_id = target.to_string_lossy().into_owned();
        let language_id = language_id_from_path(&new_id);

        if old_id != new_id {
            self.document_index.remove_document(&old_id);
        }

        self.current_document_id = new_id.clone();
        self.current_document_path = Some(target.to_path_buf());
        self.current_language_id = language_id.to_string();
        self.document_dirty = false;
        self.document_index
            .update_document(&self.current_document_id, &self.editor_preview);
        self.update_tab_identity(&old_id, &new_id, target, language_id);
        self.set_tab_dirty_state(&new_id, false);
        self.status
            .set_document_language(self.language_display_name(language_id));
        self.status.refresh_cursor(&self.editor_preview);
        self.refresh_completions();
        Ok(())
    }

    fn update_tab_identity(
        &mut self,
        old_id: &str,
        new_id: &str,
        target: &Path,
        language_id: &str,
    ) {
        let language_name = self.language_display_name(language_id);
        let display_name = target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(new_id)
            .to_string();

        for pane in &mut self.layout.panes {
            for tab in &mut pane.tabs {
                if tab.id == old_id {
                    tab.id = new_id.to_string();
                    tab.title = display_name.clone();
                    tab.language = Some(language_name.clone());
                } else if tab.id == new_id {
                    tab.language = Some(language_name.clone());
                }
            }
            if pane.active.as_deref() == Some(old_id) {
                pane.active = Some(new_id.to_string());
            }
        }
    }

    fn set_tab_dirty_state(&mut self, tab_id: &str, dirty: bool) {
        for pane in &mut self.layout.panes {
            for tab in &mut pane.tabs {
                if tab.id == tab_id {
                    let has_star = tab.title.ends_with('*');
                    if dirty && !has_star {
                        tab.title.push('*');
                    } else if !dirty && has_star {
                        tab.title.pop();
                    }
                }
            }
        }
    }

    fn mark_document_dirty(&mut self) {
        if !self.document_dirty {
            self.document_dirty = true;
            let tab_id = self.current_document_id.clone();
            self.set_tab_dirty_state(&tab_id, true);
        }
    }

    fn format_indexed(&self, key: &str, values: &[String]) -> String {
        let mut text = self.text(key).into_owned();
        for (idx, value) in values.iter().enumerate() {
            let placeholder = format!("{{{}}}", idx);
            text = text.replace(&placeholder, value);
        }
        text
    }

    fn workspace_display_name(&self) -> String {
        self.workspace_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| self.workspace_root.to_string_lossy().to_string())
    }

    fn locale_code(&self) -> &str {
        self.localization.active_code()
    }

    fn localized(&self, en: &str, zh: &str) -> String {
        if self.locale_code().starts_with("zh") {
            zh.to_string()
        } else {
            en.to_string()
        }
    }

    fn localized_owned(&self, en: String, zh: String) -> String {
        if self.locale_code().starts_with("zh") {
            zh
        } else {
            en
        }
    }

    fn persist_session(&mut self) {
        let hash = UnsavedHash::from_bytes(self.editor_preview.as_bytes());
        if let Err(err) = self
            .session_store
            .autosave()
            .write_contents(&hash, self.editor_preview.as_bytes())
        {
            log_error(self.localized_owned(
                format!("Failed to write autosave buffer: {err}"),
                format!("無法寫入自動儲存內容：{err}"),
            ));
            return;
        }

        let mut manifest = match self.session_store.autosave().load_manifest() {
            Ok(manifest) => manifest,
            Err(err) => {
                log_warn(self.localized_owned(
                    format!("Failed to load autosave manifest: {err}"),
                    format!("無法載入自動儲存清單：{err}"),
                ));
                AutosaveManifest::default()
            }
        };
        manifest.touch(hash.clone());
        if let Err(err) = self.session_store.autosave().save_manifest(&manifest) {
            log_warn(self.localized_owned(
                format!("Failed to persist autosave manifest: {err}"),
                format!("無法儲存自動儲存清單：{err}"),
            ));
        }

        let mut tab = SessionTab::default();
        if Path::new(&self.current_document_id).exists() {
            tab.path = Some(Path::new(&self.current_document_id).to_path_buf());
        }
        tab.display_name = Some(self.current_document_id.clone());
        tab.caret = SessionCaret {
            line: clamp_to_u32(self.status.line.saturating_sub(1)),
            column: clamp_to_u32(self.status.column.saturating_sub(1)),
        };
        tab.scroll = SessionScroll {
            top_line: 0,
            horizontal_offset: 0,
        };
        tab.unsaved_hash = Some(hash);
        tab.dirty_external = false;

        let mut window = SessionWindow::new();
        window.tabs.push(tab);
        window.active_tab = Some(0);

        let snapshot = SessionSnapshot::new(vec![window]);
        match self.session_store.save(&snapshot) {
            Ok(()) => log_info(self.localized_owned(
                "Session snapshot saved".to_string(),
                "工作階段已儲存".to_string(),
            )),
            Err(err) => log_error(self.localized_owned(
                format!("Failed to save session snapshot: {err}"),
                format!("無法儲存工作階段快照：{err}"),
            )),
        }
    }

    fn apply_run_defaults(&self, mut spec: RunSpec) -> RunSpec {
        if self.run_timeout_enabled {
            let secs = self.run_timeout_secs.max(1);
            spec = spec.with_timeout(Duration::from_secs(secs));
        }
        spec = spec.with_kill_on_timeout(self.run_kill_on_timeout);
        spec
    }

    fn language_display_name(&self, language_id: &str) -> String {
        self.localization
            .text(language_display_key(language_id))
            .into_owned()
    }

    fn apply_locale_change(&mut self, index: usize, summaries: &[LocaleSummary]) {
        if let Some(target) = summaries.get(index) {
            if locale_requires_cjk(&target.code) && !self.cjk_font_available {
                log_warn(self.localized_owned(
                    format!(
                        "Blocked locale switch to {} (missing CJK font)",
                        target.code
                    ),
                    format!("無法切換語系至 {}（缺少 CJK 字型）", target.display_name),
                ));
                self.font_warning = Some(
                    self.localization
                        .text("fonts.warning.cjk_missing")
                        .into_owned(),
                );
                return;
            }
        }

        if !self.localization.set_active_by_index(index) {
            return;
        }
        self.selected_locale = index;
        if let Some(summary) = summaries.get(index) {
            self.status.set_locale(&summary.display_name);
        }
        if let Some(summary) = summaries.get(index) {
            log_info(self.localized_owned(
                format!("Locale switched to {}", summary.code),
                format!("介面語系已切換為 {}", summary.display_name),
            ));
        }
        self.sample_editor_content = self.localization.text("sample.editor_preview").into_owned();
        if self.current_document_id == PREVIEW_DOCUMENT_ID {
            self.editor_preview = self.sample_editor_content.clone();
            self.document_index
                .update_document(&self.current_document_id, &self.editor_preview);
            self.status.refresh_cursor(&self.editor_preview);
        }
        if self.font_warning.is_some() {
            self.font_warning = Some(self.text("fonts.warning.cjk_missing").into_owned());
        }
        self.status
            .set_document_language(self.language_display_name(&self.current_language_id));
        self.refresh_completions();
    }

    fn handle_settings_command(&mut self, item_key: &str) {
        let target_page = match item_key {
            "menu.settings.preferences" => Some(SettingsPage::Preferences),
            "menu.settings.style_configurator" => Some(SettingsPage::StyleConfigurator),
            "menu.settings.shortcut_mapper" => Some(SettingsPage::ShortcutMapper),
            "menu.settings.edit_popup_menu" => Some(SettingsPage::ContextMenu),
            _ => None,
        };
        if let Some(page) = target_page {
            self.active_settings_page = page;
            self.show_settings_window = true;
        }
    }

    fn open_document(&mut self, path: &str, display: &str) {
        if self.activate_tab(path) {
            return;
        }
        let file_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(display);
        let mut tab = TabView::new(path, file_name);
        let language_id = language_id_from_path(path);
        tab.language = Some(self.language_display_name(language_id));
        if let Some(pane) = self
            .layout
            .panes
            .iter_mut()
            .find(|pane| pane.role == PaneRole::Primary)
        {
            pane.tabs.push(tab);
            pane.active = Some(path.to_string());
        }
        self.activate_tab(path);
    }

    fn activate_tab(&mut self, tab_id: &str) -> bool {
        let mut activated = false;
        for pane in &mut self.layout.panes {
            if pane.tabs.iter().any(|tab| tab.id == tab_id) {
                pane.active = Some(tab_id.to_string());
                activated = true;
            }
        }
        if activated {
            if let Some(tab) = self
                .layout
                .panes
                .iter()
                .flat_map(|pane| pane.tabs.iter())
                .find(|tab| tab.id == tab_id)
                .cloned()
            {
                self.load_document(&tab.id, tab.language.as_deref());
            }
            self.status.refresh_from_layout(&self.layout);
        }
        activated
    }

    fn load_document(&mut self, path: &str, language_hint: Option<&str>) {
        let fallback_template = self.text("document.load_error").into_owned();
        let contents =
            fs::read_to_string(path).unwrap_or_else(|_| fallback_template.replace("{path}", path));
        let old_id = self.current_document_id.clone();
        if old_id != path {
            self.document_index.remove_document(&old_id);
        }
        self.editor_preview = contents;
        self.current_document_id = path.to_string();
        self.current_document_path = if Path::new(path).is_file() {
            Some(PathBuf::from(path))
        } else {
            None
        };
        let language_id = language_hint
            .and_then(language_id_from_hint)
            .map(|id| id.to_string())
            .unwrap_or_else(|| language_id_from_path(path).to_string());
        self.current_language_id = language_id.clone();
        self.lsp_client
            .set_enabled(self.current_language_id.clone(), true);
        self.document_index
            .update_document(&self.current_document_id, &self.editor_preview);
        self.document_dirty = false;
        let current_id = self.current_document_id.clone();
        self.set_tab_dirty_state(&current_id, false);
        self.status.refresh_cursor(&self.editor_preview);
        self.status
            .set_document_language(self.language_display_name(&language_id));
        self.status.selection = 0;
        self.refresh_completions();
    }

    fn close_tab(&mut self, role: PaneRole, tab_id: &str) {
        if let Some(pane) = self.layout.panes.iter_mut().find(|pane| pane.role == role) {
            if let Some(pos) = pane
                .tabs
                .iter()
                .position(|tab| tab.id == tab_id && !tab.is_locked && !tab.is_pinned)
            {
                pane.tabs.remove(pos);
                if pane.active.as_deref() == Some(tab_id) {
                    let new_active = pane
                        .tabs
                        .get(pos.min(pane.tabs.len().saturating_sub(1)))
                        .map(|tab| tab.id.clone());
                    pane.active = new_active;
                }
            }
        }

        if self.current_document_id == tab_id {
            if let Some(active_tab) = self
                .layout
                .panes
                .iter()
                .find_map(|pane| pane.active_tab())
                .cloned()
            {
                self.load_document(&active_tab.id, active_tab.language.as_deref());
            } else {
                self.editor_preview = self.sample_editor_content.clone();
                self.current_document_id = PREVIEW_DOCUMENT_ID.to_string();
                self.current_language_id = PREVIEW_LANGUAGE_ID.to_string();
                self.current_document_path = None;
                self.document_dirty = false;
                self.document_index
                    .update_document(&self.current_document_id, &self.editor_preview);
                self.status.refresh_cursor(&self.editor_preview);
                self.status
                    .set_document_language(self.language_display_name(PREVIEW_LANGUAGE_ID));
                self.refresh_completions();
            }
        }

        self.status.refresh_from_layout(&self.layout);
    }

    fn apply_active_theme(&mut self, ctx: &egui::Context) {
        self.ensure_fonts(ctx);
        self.palette = self.theme_manager.active_palette().clone();
        let definition = self.theme_manager.active_theme();

        let mut visuals = match definition.kind {
            ThemeKind::Dark => egui::Visuals::dark(),
            ThemeKind::Light => egui::Visuals::light(),
        };
        visuals.override_text_color = Some(color32_from_color(self.palette.editor_text));
        visuals.panel_fill = color32_from_color(self.palette.panel);
        visuals.window_fill = color32_from_color(self.palette.panel);
        visuals.extreme_bg_color = color32_from_color(self.palette.background);
        visuals.widgets.inactive.bg_fill = color32_from_color(self.palette.panel);
        visuals.widgets.inactive.fg_stroke.color = color32_from_color(self.palette.editor_text);
        visuals.widgets.hovered.bg_fill = color32_from_color(self.palette.accent);
        visuals.widgets.active.bg_fill = color32_from_color(self.palette.accent);
        visuals.widgets.active.fg_stroke.color = color32_from_color(self.palette.accent_text);
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.visuals.override_text_color = Some(color32_from_color(self.palette.editor_text));
        style.visuals.faint_bg_color = color32_from_color(self.palette.background);
        style.visuals.hyperlink_color = color32_from_color(self.palette.accent);
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(definition.fonts.ui_size as f32, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(definition.fonts.ui_size as f32, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(
                (definition.fonts.ui_size as f32) + 2.0,
                FontFamily::Proportional,
            ),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(definition.fonts.editor_size as f32, FontFamily::Monospace),
        );
        ctx.set_style(style);

        self.status.set_theme(&definition.name);
    }

    fn ensure_fonts(&mut self, ctx: &egui::Context) {
        if self.fonts_installed {
            return;
        }

        let mut definitions = FontDefinitions::default();
        if let Some((name, data)) = load_cjk_font() {
            definitions
                .font_data
                .insert(name.clone(), FontData::from_owned(data));
            if let Some(family) = definitions.families.get_mut(&FontFamily::Proportional) {
                family.insert(0, name.clone());
            }
            if let Some(family) = definitions.families.get_mut(&FontFamily::Monospace) {
                family.push(name);
            }
            self.cjk_font_available = true;
            self.font_warning = None;
            log_info(self.localized("CJK fallback font installed", "已載入正體中文字型備援"));
        } else if self.font_warning.is_none() {
            self.font_warning = Some(self.text("fonts.warning.cjk_missing").into_owned());
            self.cjk_font_available = false;
            log_warn(self.localized(
                "CJK fallback font missing; UI will stay English",
                "未載入正體中文字型，介面將維持英文",
            ));
        } else {
            self.cjk_font_available = false;
        }

        ctx.set_fonts(definitions);
        self.fonts_installed = true;
    }

    fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for section in MENU_STRUCTURE.iter() {
                        let title = self.text(section.title_key).into_owned();
                        ui.menu_button(title, |ui| {
                            let is_settings = section.title_key == "menu.settings";
                            let is_macro = section.title_key == "menu.macro";
                            let is_run = section.title_key == "menu.run";
                            let is_file = section.title_key == "menu.file";
                            for item_key in section.item_keys {
                                let label = self.text(item_key).into_owned();
                                if is_settings {
                                    if ui.button(label.clone()).clicked() {
                                        self.handle_settings_command(item_key);
                                        ui.close_menu();
                                    }
                                } else if is_macro {
                                    if ui.button(label.clone()).clicked() {
                                        self.handle_macro_command(item_key);
                                        ui.close_menu();
                                    }
                                } else if is_run {
                                    if ui.button(label.clone()).clicked() {
                                        self.handle_run_command(item_key);
                                        ui.close_menu();
                                    }
                                } else if is_file {
                                    if ui.button(label.clone()).clicked() {
                                        self.handle_file_command(item_key);
                                        ui.close_menu();
                                    }
                                } else {
                                    ui.add_enabled(false, egui::Button::new(label));
                                }
                            }
                        });
                    }
                });
            });
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .resizable(false)
            .exact_height(38.0)
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let workspace_label = format!(
                        "{}: {}",
                        self.text("toolbar.workspace_prefix"),
                        self.workspace_display_name()
                    );
                    ui.label(RichText::new(workspace_label).strong());
                    ui.separator();
                    let mut ratio = self.layout.split_ratio;
                    if ui
                        .add(
                            egui::Slider::new(&mut ratio, 0.3..=0.7)
                                .prefix(self.text("toolbar.split_prefix").to_string())
                                .show_value(false),
                        )
                        .changed()
                    {
                        if let Ok(valid) = LayoutConfig::validate_split_ratio(ratio) {
                            self.layout.split_ratio = valid;
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pinned_count = self.layout.pinned_tabs().len();
                        ui.label(
                            self.format_indexed("toolbar.pinned_tabs", &[pinned_count.to_string()]),
                        );
                    });
                });

                if let Some(warning) = &self.font_warning {
                    ui.separator();
                    ui.label(
                        RichText::new(warning)
                            .color(Color32::from_rgb(239, 68, 68))
                            .italics(),
                    );
                }
            });
    }

    fn show_left_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("project_panel")
            .default_width(220.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(self.text("panel.project.title"));
                    ui.separator();
                    if ui
                        .button(self.localized("Reload Tree", "重新載入專案樹"))
                        .on_hover_text(
                            self.localized("Reload project tree from disk", "從磁碟重新載入專案樹"),
                        )
                        .clicked()
                    {
                        match self.project_tree_store.load() {
                            Ok(Some(tree)) => {
                                self.project_tree = tree;
                                log_info(self.localized_owned(
                                    "Project tree reloaded".to_string(),
                                    "專案樹已重新載入".to_string(),
                                ));
                            }
                            Ok(None) => log_warn(self.localized_owned(
                                "Project tree file missing".to_string(),
                                "專案樹檔案不存在".to_string(),
                            )),
                            Err(err) => log_error(format!(
                                "{}",
                                self.localized_owned(
                                    format!("Failed to reload project tree: {err}"),
                                    format!("無法重新載入專案樹：{err}"),
                                )
                            )),
                        }
                    }
                    if ui
                        .button(self.localized("Save Session", "儲存工作階段"))
                        .on_hover_text(
                            self.localized("Flush session snapshot to disk", "將工作階段寫入磁碟"),
                        )
                        .clicked()
                    {
                        self.persist_session();
                    }
                    ui.add_space(6.0);
                    let root_children = self.project_tree.root.children.clone();
                    for child in root_children.iter() {
                        self.render_project_node(ui, child, 0);
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_highlight_summary(ui);
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_function_list_panel(ui);
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_completion_panel(ui);
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_macro_panel(ui);
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_run_panel(ui);
                });
            });
    }

    fn show_right_sidebar(&self, ctx: &egui::Context) {
        egui::SidePanel::right("document_map")
            .default_width(180.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading(self.text("panel.document_map.title"));
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        for (idx, line) in self.editor_preview.lines().enumerate() {
                            ui.label(format!("{:>4} {}", idx + 1, line));
                        }
                    });
            });
    }

    fn show_bottom_dock(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panels")
            .resizable(true)
            .min_height(140.0)
            .show(ctx, |ui| {
                let panel_ids = self.layout.bottom_dock.visible_panels.clone();
                if panel_ids.is_empty() {
                    ui.label(self.text("panel.no_panels_configured"));
                    return;
                }

                ui.horizontal(|ui| {
                    for (idx, panel) in panel_ids.iter().enumerate() {
                        let title_key = match panel.as_str() {
                            "find_results" => Some("panel.find_results.title"),
                            "console" => Some("panel.console.title"),
                            "notifications" => Some("panel.notifications.title"),
                            "lsp" => Some("panel.lsp.title"),
                            _ => None,
                        };
                        let title = title_key
                            .map(|key| self.text(key).into_owned())
                            .unwrap_or_else(|| panel.to_string());
                        let selected = idx == self.bottom_tab_index;
                        if ui
                            .selectable_label(selected, RichText::new(title).strong())
                            .clicked()
                        {
                            self.bottom_tab_index = idx;
                            self.layout.bottom_dock.active_panel = Some(panel.to_string());
                        }
                    }
                });
                ui.separator();

                let active_panel = panel_ids
                    .get(self.bottom_tab_index)
                    .cloned()
                    .or_else(|| self.layout.bottom_dock.active_panel.clone())
                    .unwrap_or_else(|| "find_results".into());

                match active_panel.as_str() {
                    "find_results" => {
                        ui.label(self.text("panel.find_results.summary"));
                        ui.label("search/src/lib.rs:42  let matches = engine.find_all(&options)?;");
                        ui.label(
                            "core/src/search_session.rs:155  document.set_contents(replaced_text);",
                        );
                    }
                    "console" => {
                        if let Some(error) = &self.run_last_error {
                            ui.colored_label(Color32::from_rgb(239, 68, 68), format!("{error}"));
                            ui.separator();
                        }
                        if self.run_history.is_empty() {
                            ui.label(self.localized("No run history yet", "尚無執行紀錄"));
                        } else {
                            for entry in self.run_history.iter() {
                                ui.group(|ui| {
                                    let exit_code = entry
                                        .result
                                        .exit_code
                                        .map(|code| code.to_string())
                                        .unwrap_or_else(|| "signal".to_string());
                                    ui.label(self.localized_owned(
                                        format!(
                                            "{}\nExit: {} | Duration: {} ms",
                                            entry.title, exit_code, entry.result.duration_ms
                                        ),
                                        format!(
                                            "{}\n結束碼：{}｜耗時 {} 毫秒",
                                            entry.title, exit_code, entry.result.duration_ms
                                        ),
                                    ));
                                    ui.label(self.localized_owned(
                                        format!("Command: {}", entry.command),
                                        format!("指令：{}", entry.command),
                                    ));
                                    let workdir = entry
                                        .working_dir
                                        .as_ref()
                                        .map(|path| path.display().to_string())
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(self.localized_owned(
                                        format!("Workdir: {}", workdir),
                                        format!("工作目錄：{}", workdir),
                                    ));
                                    let env_text = if entry.env.is_empty() {
                                        "-".to_string()
                                    } else {
                                        entry
                                            .env
                                            .iter()
                                            .map(|(k, v)| format!("{k}={v}"))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    };
                                    ui.label(self.localized_owned(
                                        format!("Env overrides: {}", env_text),
                                        format!("環境覆寫：{}", env_text),
                                    ));
                                    let cleared_en = if entry.cleared_env { "yes" } else { "no" };
                                    let cleared_zh = if entry.cleared_env { "是" } else { "否" };
                                    ui.label(self.localized_owned(
                                        format!("Cleared env: {cleared_en}"),
                                        format!("已清除環境變數：{cleared_zh}"),
                                    ));
                                    let timeout_desc_en = entry
                                        .timeout_ms
                                        .map(|ms| format!("{:.2} s", (ms as f64) / 1000.0))
                                        .unwrap_or_else(|| "disabled".to_string());
                                    let timeout_desc_zh = entry
                                        .timeout_ms
                                        .map(|ms| format!("{:.2} 秒", (ms as f64) / 1000.0))
                                        .unwrap_or_else(|| "已停用".to_string());
                                    ui.label(self.localized_owned(
                                        format!("Timeout: {timeout_desc_en}"),
                                        format!("逾時：{timeout_desc_zh}"),
                                    ));
                                    let kill_desc_en =
                                        if entry.kill_on_timeout { "yes" } else { "no" };
                                    let kill_desc_zh =
                                        if entry.kill_on_timeout { "是" } else { "否" };
                                    ui.label(self.localized_owned(
                                        format!("Kill on timeout: {kill_desc_en}"),
                                        format!("逾時後終止：{kill_desc_zh}"),
                                    ));
                                    if entry.result.timed_out {
                                        ui.colored_label(
                                            Color32::from_rgb(239, 68, 68),
                                            self.localized("Timed out", "已逾時"),
                                        );
                                    }

                                    egui::CollapsingHeader::new(
                                        self.localized("stdout", "標準輸出"),
                                    )
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        if entry.stdout_text.trim().is_empty() {
                                            ui.label(
                                                self.localized("No stdout output", "無標準輸出"),
                                            );
                                        } else {
                                            ui.code(entry.stdout_text.clone());
                                        }
                                    });
                                    egui::CollapsingHeader::new(
                                        self.localized("stderr", "標準錯誤"),
                                    )
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        if entry.stderr_text.trim().is_empty() {
                                            ui.label(
                                                self.localized(
                                                    "No stderr output",
                                                    "無標準錯誤輸出",
                                                ),
                                            );
                                        } else {
                                            ui.code(entry.stderr_text.clone());
                                        }
                                    });
                                });
                            }
                        }
                    }
                    "notifications" => {
                        ui.label(self.text("panel.notifications.idle"));
                        ui.label(self.text("panel.notifications.font_warning"));
                    }
                    "lsp" => {
                        if self.lsp_client.is_online() {
                            ui.label(self.text("panel.lsp.connected"));
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(239, 68, 68),
                                self.text("panel.lsp.offline").to_string(),
                            );
                        }
                        let diagnostics = self
                            .lsp_client
                            .diagnostics(self.current_language_id.as_str());
                        if diagnostics.is_empty() {
                            ui.label(self.text("panel.lsp.no_diagnostics"));
                        } else {
                            for diagnostic in diagnostics.iter().take(5) {
                                let color = diagnostic_color(diagnostic.severity);
                                ui.colored_label(
                                    color,
                                    format!("[{:?}] {}", diagnostic.severity, diagnostic.message),
                                );
                            }
                            if diagnostics.len() > 5 {
                                let remaining = diagnostics.len() - 5;
                                ui.label(self.format_indexed(
                                    "panel.lsp.more_diagnostics",
                                    &[remaining.to_string()],
                                ));
                            }
                        }
                    }
                    other => {
                        let template = self.text("panel.generic.no_content").into_owned();
                        ui.label(template.replace("{panel}", other));
                    }
                }
            });
    }

    fn show_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .exact_height(24.0)
            .show(ctx, |ui| {
                let palette = &self.palette;
                ui.painter().rect_filled(
                    ui.max_rect(),
                    0.0,
                    color32_from_color(palette.status_bar),
                );

                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.label(self.format_indexed(
                        "status.position",
                        &[
                            self.status.line.to_string(),
                            self.status.column.to_string(),
                            self.status.lines.to_string(),
                        ],
                    ));
                    ui.separator();
                    ui.label(
                        self.format_indexed(
                            "status.selection",
                            &[self.status.selection.to_string()],
                        ),
                    );
                    ui.separator();
                    ui.label(self.status.mode);
                    ui.separator();
                    ui.label(self.status.encoding);
                    ui.separator();
                    ui.label(self.status.eol);
                    ui.separator();
                    ui.label(self.format_indexed(
                        "status.lang_label",
                        &[self.status.document_language.clone()],
                    ));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.label(
                        self.format_indexed("status.ui_label", &[self.status.ui_language.clone()]),
                    );
                    ui.separator();
                    ui.label(
                        self.format_indexed("status.theme_label", &[self.status.theme.clone()]),
                    );
                });
            });
    }

    fn show_editor_area(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let primary_snapshot = self
                .layout
                .panes
                .iter()
                .find(|pane| pane.role == PaneRole::Primary)
                .cloned();
            let secondary_snapshot = self
                .layout
                .panes
                .iter()
                .find(|pane| pane.role == PaneRole::Secondary)
                .cloned();

            let available_height = ui.available_height();
            let available_width = ui.available_width();
            let primary_width = available_width * self.layout.split_ratio;
            let secondary_width = available_width - primary_width;

            ui.allocate_ui_with_layout(
                vec2(available_width, available_height),
                Layout::left_to_right(Align::Min),
                |ui| {
                    if let Some(pane) = primary_snapshot {
                        ui.allocate_ui_with_layout(
                            vec2(primary_width, available_height),
                            Layout::top_down(Align::Min),
                            |ui| {
                                self.render_tab_strip(ui, pane.clone());
                                ui.separator();
                                egui::Frame::group(ui.style())
                                    .fill(color32_from_color(self.palette.editor_background))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        color32_from_color(self.palette.panel),
                                    ))
                                    .show(ui, |ui| {
                                        let mut buffer = self.editor_preview.clone();
                                        let text_edit = egui::TextEdit::multiline(&mut buffer)
                                            .font(TextStyle::Monospace)
                                            .desired_rows(24)
                                            .desired_width(f32::INFINITY);
                                        let response = ui.add_sized(ui.available_size(), text_edit);
                                        if response.changed() {
                                            self.editor_preview = buffer;
                                            self.status.refresh_cursor(&self.editor_preview);
                                            self.document_index.update_document(
                                                &self.current_document_id,
                                                &self.editor_preview,
                                            );
                                            self.refresh_completions();
                                            self.mark_document_dirty();
                                        }
                                    });
                            },
                        );
                    }

                    if let Some(pane) = secondary_snapshot {
                        ui.separator();
                        ui.allocate_ui_with_layout(
                            vec2(secondary_width, available_height),
                            Layout::top_down(Align::Min),
                            |ui| {
                                self.render_tab_strip(ui, pane.clone());
                                ui.separator();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.label(self.text("panel.secondary.title"));
                                    ui.add_space(6.0);
                                    if let Some(active) = pane.active_tab() {
                                        ui.label(self.format_indexed(
                                            "panel.secondary.active",
                                            &[active.title.clone()],
                                        ));
                                        ui.label(self.text("panel.secondary.readonly"));
                                    }
                                });
                            },
                        );
                    }
                },
            );
        });
    }

    fn render_highlight_summary(&self, ui: &mut egui::Ui) {
        ui.heading(self.text("highlight.heading").to_string());
        match self
            .highlight_registry
            .highlight(self.current_language_id.as_str(), &self.editor_preview)
        {
            Ok(tokens) => {
                ui.label(self.format_indexed("highlight.tokens", &[tokens.len().to_string()]));
                let mut counts: BTreeMap<String, usize> = BTreeMap::new();
                for token in &tokens {
                    *counts.entry(format!("{:?}", token.kind)).or_insert(0) += 1;
                }
                for (kind, count) in counts.iter() {
                    ui.label(format!("{kind}: {count}"));
                }
                ui.add_space(6.0);
                ui.label(RichText::new(self.text("highlight.sample").to_string()).italics());
                for token in tokens.iter().take(6) {
                    if let Some(snippet) = self.editor_preview.get(token.range.clone()) {
                        let snippet = snippet.replace('\n', " ");
                        if snippet.trim().is_empty() {
                            continue;
                        }
                        let preview = truncate_snippet(&snippet, 32);
                        ui.label(format!("{:?}: {}", token.kind, preview));
                    }
                }
                if tokens.len() > 6 {
                    ui.label(self.text("highlight.more").to_string());
                }
            }
            Err(err) => {
                ui.colored_label(
                    Color32::from_rgb(239, 68, 68),
                    self.format_indexed("highlight.error", &[err.to_string()]),
                );
            }
        }
    }

    fn render_function_list_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("function.heading").to_string());
        match self
            .function_registry
            .parse(self.current_language_id.as_str(), &self.editor_preview)
        {
            Some(entries) if !entries.is_empty() => {
                for entry in entries.iter().take(10) {
                    let line = line_number_for_range(&self.editor_preview, &entry.range);
                    let label = self.format_indexed(
                        "function.entry_label",
                        &[
                            format!("{:?}", entry.kind),
                            entry.name.clone(),
                            line.to_string(),
                        ],
                    );
                    if ui
                        .selectable_label(false, label)
                        .on_hover_text(self.text("function.navigate_hover").to_string())
                        .clicked()
                    {
                        self.status.line = line;
                        self.status.column = 1;
                        self.status.selection = 0;
                    }
                }
                if entries.len() > 10 {
                    let remaining = entries.len() - 10;
                    ui.label(self.format_indexed("function.more", &[remaining.to_string()]));
                }
            }
            _ => {
                ui.label(self.text("function.no_symbols").to_string());
            }
        }
    }

    fn render_completion_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("autocomplete.heading").to_string());
        ui.horizontal(|ui| {
            let lsp_online = self.lsp_client.is_online();
            let mut lsp_enabled = self
                .lsp_client
                .is_enabled(self.current_language_id.as_str());
            if ui
                .checkbox(
                    &mut lsp_enabled,
                    self.text("autocomplete.enable_lsp").to_string(),
                )
                .changed()
            {
                self.lsp_client
                    .set_enabled(self.current_language_id.clone(), lsp_enabled);
                self.refresh_completions();
            }
            if !lsp_online {
                ui.colored_label(
                    Color32::from_rgb(239, 68, 68),
                    self.text("autocomplete.lsp_offline").to_string(),
                );
            }
        });
        let mut prefix = self.completion_prefix.clone();
        let response = ui.add(
            egui::TextEdit::singleline(&mut prefix)
                .hint_text(self.text("autocomplete.prefix_hint").to_string())
                .desired_width(f32::INFINITY),
        );
        if response.changed() {
            self.completion_prefix = prefix;
            self.refresh_completions();
        }

        if self.completion_results.is_empty() {
            ui.label(self.text("autocomplete.no_suggestions").to_string());
        } else {
            let visible = self.completion_results.len().min(10);
            for idx in 0..visible {
                if let Some(item) = self.completion_results.get(idx) {
                    let kind = format!("{:?}", item.kind);
                    let detail = item.detail.as_deref().unwrap_or("");
                    let label = if detail.is_empty() {
                        format!("{kind} - {}", item.label)
                    } else {
                        format!("{kind} - {} ({detail})", item.label)
                    };
                    if ui
                        .selectable_label(false, label)
                        .on_hover_text(self.text("autocomplete.apply_hover").to_string())
                        .clicked()
                    {
                        self.completion_prefix = item.label.clone();
                        self.refresh_completions();
                    }
                }
            }
            if self.completion_results.len() > 10 {
                let remaining = self.completion_results.len() - 10;
                ui.label(
                    self.format_indexed("autocomplete.more_suggestions", &[remaining.to_string()]),
                );
            }
        }
    }

    fn render_macro_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localized("Macro Recorder", "巨集錄製器"));
        let status_text = if self.macro_recorder.is_recording() {
            self.localized("Status: Recording", "狀態：錄製中")
        } else {
            self.localized("Status: Idle", "狀態：待命")
        };
        ui.label(status_text);

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.macro_recorder.is_recording(),
                    egui::Button::new(self.localized("Start", "開始")),
                )
                .clicked()
            {
                self.start_macro_recording();
            }
            if ui
                .add_enabled(
                    self.macro_recorder.is_recording(),
                    egui::Button::new(self.localized("Stop", "停止")),
                )
                .clicked()
            {
                self.stop_macro_recording();
            }
            if ui
                .add_enabled(
                    self.macro_recorder.is_recording(),
                    egui::Button::new(self.localized("Cancel", "取消")),
                )
                .clicked()
            {
                self.cancel_macro_recording();
            }
        });

        ui.add_space(6.0);
        ui.label(self.localized("Macro name", "巨集名稱"));
        ui.text_edit_singleline(&mut self.macro_pending_name);

        ui.add_space(6.0);
        ui.label(self.localized("Insert text", "插入文字"));
        let text_hint = self.localized("Enter text to record", "輸入要錄製的文字");
        ui.add(egui::TextEdit::singleline(&mut self.macro_input_buffer).hint_text(text_hint));
        if ui
            .add_enabled(
                self.macro_recorder.is_recording() && !self.macro_input_buffer.trim().is_empty(),
                egui::Button::new(self.localized("Add Text Event", "加入文字事件")),
            )
            .clicked()
        {
            self.add_macro_text_event();
        }

        ui.add_space(6.0);
        ui.label(self.localized("Command", "指令"));
        if !MACRO_COMMAND_OPTIONS.is_empty() {
            let max_index = MACRO_COMMAND_OPTIONS.len().saturating_sub(1);
            if self.macro_selected_command > max_index {
                self.macro_selected_command = max_index;
            }
            let (_, label_en, label_zh) = MACRO_COMMAND_OPTIONS[self.macro_selected_command];
            let current_label = self.localized(label_en, label_zh);
            egui::ComboBox::from_id_source("macro_command_combo")
                .selected_text(current_label)
                .show_ui(ui, |ui| {
                    for (idx, (_, label_en, label_zh)) in MACRO_COMMAND_OPTIONS.iter().enumerate() {
                        let label = self.localized(label_en, label_zh);
                        ui.selectable_value(&mut self.macro_selected_command, idx, label);
                    }
                });
            if ui
                .add_enabled(
                    self.macro_recorder.is_recording(),
                    egui::Button::new(self.localized("Add Command Event", "加入指令事件")),
                )
                .clicked()
            {
                self.add_macro_command_event();
            }
        }

        ui.add_space(6.0);
        ui.label(self.localized("Events", "事件"));
        if self.macro_live_events.is_empty() {
            ui.label(self.localized("No events recorded yet", "尚未錄製事件"));
        } else {
            for entry in &self.macro_live_events {
                ui.label(format!("• {entry}"));
            }
        }

        ui.separator();
        ui.label(self.localized("Saved macros", "已儲存巨集"));
        if self.macro_store.iter().next().is_none() {
            ui.label(self.localized("No macros saved", "尚未保存巨集"));
        } else {
            for (name, macro_def) in self.macro_store.iter() {
                let label = format!("{name} ({})", macro_def.events.len());
                let selected = self
                    .selected_macro
                    .as_ref()
                    .map(|current| current == name)
                    .unwrap_or(false);
                if ui.selectable_label(selected, label).clicked() {
                    self.selected_macro = Some(name.clone());
                }
            }
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(self.localized("Repeat", "重複次數"));
            self.macro_repeat_count = self.macro_repeat_count.max(1);
            ui.add(
                egui::DragValue::new(&mut self.macro_repeat_count)
                    .clamp_range(1..=20)
                    .speed(1.0),
            );
        });

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.selected_macro.is_some(),
                    egui::Button::new(self.localized("Play", "播放")),
                )
                .clicked()
            {
                self.play_selected_macro();
            }
            if ui
                .add_enabled(
                    self.selected_macro.is_some(),
                    egui::Button::new(self.localized("Delete", "刪除")),
                )
                .clicked()
            {
                self.delete_selected_macro();
            }
        });

        ui.separator();
        ui.label(self.localized("Recorder log", "錄製紀錄"));
        if self.macro_messages.is_empty() {
            ui.label(self.localized("No messages yet", "尚無紀錄"));
        } else {
            for entry in self.macro_messages.iter().rev() {
                ui.label(entry);
            }
        }
    }

    fn render_run_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localized("Run Presets", "執行預設"));
        ui.label(self.localized("Timeout", "逾時設定"));
        let enable_timeout_label = self.localized("Enable timeout", "啟用逾時");
        let kill_on_timeout_label = self.localized("Kill on timeout", "逾時後強制終止");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.run_timeout_enabled, enable_timeout_label.clone());
            ui.checkbox(&mut self.run_kill_on_timeout, kill_on_timeout_label.clone());
        });
        if self.run_timeout_enabled {
            self.run_timeout_secs = self.run_timeout_secs.clamp(1, 600);
            let suffix = if self.locale_code().starts_with("zh") {
                " 秒"
            } else {
                " s"
            };
            ui.add(
                egui::DragValue::new(&mut self.run_timeout_secs)
                    .clamp_range(1..=600)
                    .speed(1.0)
                    .suffix(suffix),
            )
            .on_hover_text(self.localized("Max execution time", "最大執行秒數"));
        } else {
            ui.label(self.localized("Timeout disabled", "已停用逾時限制"));
        }

        ui.add_space(6.0);
        ui.label(self.localized("Notes", "備註"));
        ui.small(self.localized(
            "Settings apply to all presets in this preview. Production builds will allow per-command overrides.",
            "此預覽版設定會套用到所有預設指令，正式版會提供每個指令的獨立設定。",
        ));
    }

    fn render_tab_strip(&mut self, ui: &mut egui::Ui, pane: PaneLayout) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                let active_id = pane.active.as_deref();
                for tab in pane.tabs.iter().filter(|tab| tab.is_pinned) {
                    self.render_tab_button(ui, pane.role, active_id, tab);
                }
                for tab in pane.tabs.iter().filter(|tab| !tab.is_pinned) {
                    self.render_tab_button(ui, pane.role, active_id, tab);
                }
            });
        });
    }

    fn render_tab_button(
        &mut self,
        ui: &mut egui::Ui,
        role: PaneRole,
        active_id: Option<&str>,
        tab: &TabView,
    ) {
        let is_active = active_id
            .map(|active| active == tab.id.as_str())
            .unwrap_or(false);

        ui.horizontal(|ui| {
            if let Some(tag) = tab.color {
                let color = color32_from_color(parse_tag_color(tag));
                draw_color_badge(ui, color);
                ui.add_space(4.0);
            }

            let mut label = String::new();
            if tab.is_pinned {
                label.push_str("[P] ");
            }
            label.push_str(&tab.title);
            if tab.is_locked {
                label.push_str(" [RO]");
            }
            let text = if is_active {
                RichText::new(label).color(color32_from_color(self.palette.accent_text))
            } else {
                RichText::new(label).color(color32_from_color(self.palette.editor_text))
            };

            let mut button = egui::Button::new(text).frame(false);
            if is_active {
                button = button.fill(color32_from_color(self.palette.accent));
            }
            if ui.add(button).clicked() {
                if self.activate_tab(&tab.id) {
                    self.status.refresh_from_layout(&self.layout);
                }
            }

            if !tab.is_pinned && !tab.is_locked {
                let close = egui::Button::new(RichText::new("✕").small()).frame(false);
                if ui
                    .add(close)
                    .on_hover_text(self.text("tabs.close_hover").to_string())
                    .clicked()
                {
                    self.close_tab(role, &tab.id);
                }
            }
        });
        ui.add_space(6.0);
    }

    fn render_project_node(&mut self, ui: &mut egui::Ui, node: &ProjectNode, depth: usize) {
        let indent = "    ".repeat(depth);
        match &node.kind {
            ProjectNodeKind::File { path } => {
                let label = format!("{indent}{}", node.name);
                let path_display = path.to_string_lossy();
                if ui
                    .selectable_label(false, label)
                    .on_hover_text(self.text("explorer.open_document_hover").to_string())
                    .clicked()
                {
                    self.open_document(path_display.as_ref(), &node.name);
                }
            }
            ProjectNodeKind::Folder { .. } => {
                let label = format!("{indent}{}", node.name);
                egui::CollapsingHeader::new(label)
                    .default_open(depth < 2)
                    .show(ui, |ui| {
                        for child in node.children.iter() {
                            self.render_project_node(ui, child, depth + 1);
                        }
                    });
            }
            ProjectNodeKind::Virtual { subtype, .. } => {
                let label = format!("{indent}{} ({subtype})", node.name);
                ui.label(RichText::new(label).color(color32_from_color(self.palette.editor_text)));
            }
        }
    }
}

struct AppMacroExecutor<'a> {
    app: &'a mut RustNotePadApp,
}

impl<'a> MacroExecutor for AppMacroExecutor<'a> {
    fn execute_command(&mut self, command_id: &str) -> Result<(), String> {
        match command_id {
            "macro.command.uppercase_preview" => {
                self.app.editor_preview = self.app.editor_preview.to_uppercase();
                self.app.after_macro_edit();
                self.app
                    .push_macro_log_localized("Preview uppercased", "預覽內容已轉成大寫");
                Ok(())
            }
            "macro.command.append_signature" => {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);
                let signature = self.app.localized_owned(
                    format!("\n// Macro signature {timestamp}\n"),
                    format!("\n// 巨集簽章 {timestamp}\n"),
                );
                self.app.editor_preview.push_str(&signature);
                self.app.after_macro_edit();
                self.app
                    .push_macro_log_localized("Appended signature", "已附加簽名註解");
                Ok(())
            }
            other => Err(self.app.localized_owned(
                format!("Unhandled command {other}"),
                format!("未支援指令 {other}"),
            )),
        }
    }

    fn insert_text(&mut self, text: &str) -> Result<(), String> {
        self.app.editor_preview.push_str(text);
        self.app.after_macro_edit();
        let snippet = if text.len() > 18 {
            format!("{}…", &text[..18])
        } else {
            text.to_string()
        };
        self.app.push_macro_log_localized_owned(
            format!("Inserted \"{snippet}\""),
            format!("已插入「{snippet}」"),
        );
        Ok(())
    }
}

impl App for RustNotePadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.apply_theme_if_needed(ctx);
        self.status.refresh_from_layout(&self.layout);

        self.show_menu_bar(ctx);
        self.show_toolbar(ctx);
        self.show_left_sidebar(ctx);
        self.show_right_sidebar(ctx);
        self.show_bottom_dock(ctx);
        self.show_status_bar(ctx);
        self.show_editor_area(ctx);
        self.render_settings_window(ctx);
        self.render_file_dialogs(ctx);

        if self.pending_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.pending_exit = false;
        }
    }
}

impl Drop for RustNotePadApp {
    fn drop(&mut self) {
        self.persist_session();
    }
}

fn color32_from_color(color: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

impl RustNotePadApp {
    fn render_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings_window {
            return;
        }
        let mut open = self.show_settings_window;
        egui::Window::new(self.text("settings.window.title").to_string())
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.set_min_height(320.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(180.0);
                        ui.heading(self.text("settings.window.sections").to_string());
                        ui.separator();
                        let tabs = [
                            (SettingsPage::Preferences, "settings.tab.preferences"),
                            (SettingsPage::StyleConfigurator, "settings.tab.style"),
                            (SettingsPage::ShortcutMapper, "settings.tab.shortcuts"),
                            (SettingsPage::ContextMenu, "settings.tab.context_menu"),
                        ];
                        for (page, key) in tabs.iter() {
                            let label = self.text(key).to_string();
                            ui.selectable_value(&mut self.active_settings_page, *page, label);
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        match self.active_settings_page {
                            SettingsPage::Preferences => self.render_preferences_page(ui),
                            SettingsPage::StyleConfigurator => self.render_style_configurator(ui),
                            SettingsPage::ShortcutMapper => {
                                self.render_placeholder_page(ui, "settings.shortcuts.placeholder")
                            }
                            SettingsPage::ContextMenu => {
                                self.render_placeholder_page(ui, "settings.context.placeholder")
                            }
                        }
                    });
                });
            });
        self.show_settings_window = open;
    }

    fn render_preferences_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("settings.preferences.heading").to_string());
        ui.separator();
        let autosave_label = self.text("settings.preferences.autosave").to_string();
        let line_numbers_label = self.text("settings.preferences.line_numbers").to_string();
        let highlight_label = self.text("settings.preferences.highlight_line").to_string();
        let suffix = self
            .text("settings.preferences.autosave_suffix")
            .to_string();
        ui.checkbox(&mut self.preferences.autosave_enabled, autosave_label);
        ui.horizontal(|ui| {
            ui.label(
                self.text("settings.preferences.autosave_interval")
                    .to_string(),
            );
            ui.add(
                egui::DragValue::new(&mut self.preferences.autosave_interval_minutes)
                    .clamp_range(1..=60)
                    .speed(1.0)
                    .suffix(suffix),
            );
        });
        ui.checkbox(&mut self.preferences.show_line_numbers, line_numbers_label);
        ui.checkbox(&mut self.preferences.highlight_active_line, highlight_label);
        ui.add_space(12.0);

        // Render locale selector similar to Notepad++ preferences panel.
        // （仿照 Notepad++ 偏好設定面板呈現語系選擇器。）
        ui.separator();
        ui.heading(
            self.text("settings.preferences.localization_heading")
                .to_string(),
        );
        let locale_summaries = self.localization.locale_summaries();
        let mut locale_index = self.selected_locale;
        let current_locale = locale_summaries
            .get(locale_index)
            .map(|summary| summary.display_name.clone())
            .unwrap_or_else(|| "English (en-US)".to_string());
        ui.label(
            self.text("settings.preferences.localization_label")
                .to_string(),
        );
        egui::ComboBox::from_id_source("preferences_locale_selector")
            .width(220.0)
            .selected_text(current_locale)
            .show_ui(ui, |ui| {
                for (idx, summary) in locale_summaries.iter().enumerate() {
                    let selected = idx == locale_index;
                    if ui
                        .selectable_label(selected, &summary.display_name)
                        .clicked()
                    {
                        locale_index = idx;
                    }
                }
            });
        if locale_index != self.selected_locale {
            self.apply_locale_change(locale_index, &locale_summaries);
        }
        ui.add_space(12.0);
        ui.label(RichText::new(self.text("settings.preferences.note").to_string()).italics());
    }

    fn render_style_configurator(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("settings.style.heading").to_string());
        ui.separator();
        let theme_names: Vec<&str> = self.theme_manager.theme_names().collect();
        let mut active_index = self.theme_manager.active_index();
        ui.label(self.text("settings.style.theme_label").to_string());
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for (idx, name) in theme_names.iter().enumerate() {
                        let selected = idx == active_index;
                        if ui.selectable_label(selected, *name).clicked() {
                            active_index = idx;
                        }
                    }
                });
        });
        if active_index != self.theme_manager.active_index() {
            if self.theme_manager.set_active_index(active_index).is_some() {
                self.pending_theme_refresh = true;
                self.status
                    .set_theme(&self.theme_manager.active_theme().name);
            }
        }
        ui.add_space(12.0);
        ui.label(RichText::new(self.text("settings.style.note").to_string()).italics());
    }

    fn render_placeholder_page(&self, ui: &mut egui::Ui, key: &str) {
        ui.heading(self.text("settings.placeholder.heading").to_string());
        ui.separator();
        ui.label(self.text(key).to_string());
    }

    fn render_file_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_open_dialog {
            let mut open = self.show_open_dialog;
            let mut should_close = false;
            egui::Window::new(self.localized("Open File", "開啟檔案"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_min_width(360.0);
                    ui.label(self.localized("Enter a path to open", "輸入要開啟的檔案路徑"));
                    ui.add(egui::TextEdit::singleline(&mut self.open_dialog_path));
                    if let Some(err) = &self.open_dialog_error {
                        ui.colored_label(Color32::from_rgb(239, 68, 68), err);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button(self.localized("Open", "開啟")).clicked() {
                            if self.attempt_open_dialog() {
                                should_close = true;
                            }
                        }
                        if ui.button(self.localized("Cancel", "取消")).clicked() {
                            self.open_dialog_error = None;
                            self.open_dialog_path.clear();
                            should_close = true;
                        }
                    });
                });
            self.show_open_dialog = if should_close { false } else { open };
        }

        if self.show_save_as_dialog {
            let mut open = self.show_save_as_dialog;
            let mut should_close = false;
            egui::Window::new(self.localized("Save As", "另存新檔"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_min_width(360.0);
                    ui.label(self.localized("Choose a destination path", "選擇要儲存的路徑"));
                    ui.add(egui::TextEdit::singleline(&mut self.save_dialog_path));
                    if let Some(err) = &self.save_dialog_error {
                        ui.colored_label(Color32::from_rgb(239, 68, 68), err);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button(self.localized("Save", "儲存")).clicked() {
                            if self.attempt_save_dialog() {
                                should_close = true;
                            }
                        }
                        if ui.button(self.localized("Cancel", "取消")).clicked() {
                            self.save_dialog_error = None;
                            should_close = true;
                        }
                    });
                });
            self.show_save_as_dialog = if should_close { false } else { open };
        }
    }
}

fn diagnostic_color(severity: DiagnosticSeverity) -> Color32 {
    match severity {
        DiagnosticSeverity::Error => Color32::from_rgb(239, 68, 68),
        DiagnosticSeverity::Warning => Color32::from_rgb(249, 115, 22),
        DiagnosticSeverity::Information => Color32::from_rgb(59, 130, 246),
        DiagnosticSeverity::Hint => Color32::from_rgb(148, 163, 184),
    }
}

fn parse_tag_color(tag: TabColorTag) -> Color {
    // Tag hex strings are trusted constants; unwrap is safe.
    // 標籤色碼為可信常數，unwrap 可安全使用。
    Color::from_hex(tag.hex()).expect("valid color tag")
}

fn draw_color_badge(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

fn language_id_from_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "json" => "json",
        "toml" => "plaintext",
        "md" => "plaintext",
        _ => "plaintext",
    }
}

fn language_id_from_hint(hint: &str) -> Option<&'static str> {
    match hint.to_lowercase().replace(' ', "").as_str() {
        "rust" => Some("rust"),
        "json" => Some("json"),
        "plaintext" => Some("plaintext"),
        "markdown" => Some("plaintext"),
        _ => None,
    }
}

fn language_display_key(language_id: &str) -> &'static str {
    match language_id {
        "rust" => "language.name.rust",
        "json" => "language.name.json",
        "plaintext" => "language.name.plaintext",
        "markdown" => "language.name.markdown",
        _ => "language.name.plaintext",
    }
}

fn locale_requires_cjk(code: &str) -> bool {
    let normalised = code.to_ascii_lowercase();
    normalised.starts_with("zh")
        || normalised.starts_with("ja")
        || normalised.starts_with("ko")
        || normalised == "zh-tw"
        || normalised == "zh-cn"
}

fn clamp_to_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

fn truncate_snippet(snippet: &str, max_chars: usize) -> String {
    let trimmed = snippet.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let prefix: String = trimmed.chars().take(max_chars).collect();
        format!("{prefix}…")
    }
}

fn line_number_for_range(text: &str, range: &TextRange) -> usize {
    let start = range.start.min(text.len());
    text[..start].lines().count().saturating_add(1)
}

fn install_panic_logger() {
    static PANIC_HOOK: Once = Once::new();
    PANIC_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let location = info
                .location()
                .map(|loc| format!("{}:{}", loc.file(), loc.line()))
                .unwrap_or_else(|| "unknown".to_string());
            let payload = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic payload unavailable".to_string());
            log_error(format!("panic captured at {location}: {payload}"));
        }));
    });
}

fn load_cjk_font() -> Option<(String, Vec<u8>)> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();

    candidates.push(manifest_dir.join("assets/fonts/NotoSansTC-Regular.otf"));
    candidates.push(manifest_dir.join("../assets/fonts/NotoSansTC-Regular.otf"));
    candidates.push(PathBuf::from("assets/fonts/NotoSansTC-Regular.otf"));

    #[cfg(target_os = "windows")]
    {
        candidates.push(PathBuf::from(r"C:\Windows\Fonts\msjh.ttc"));
        candidates.push(PathBuf::from(r"C:\Windows\Fonts\mingliu.ttc"));
        candidates.push(PathBuf::from(r"C:\Windows\Fonts\NotoSansTC-Regular.otf"));
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Songti.ttc",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        candidates.push(PathBuf::from(
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/opentype/noto/NotoSansTC-Regular.otf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/noto/NotoSansTC-Regular.ttf",
        ));
    }

    for path in candidates {
        if !path.exists() {
            continue;
        }
        match fs::read(&path) {
            Ok(bytes) => match FontArc::try_from_vec(bytes.clone()) {
                Ok(_) => {
                    log_info(format!("Loaded CJK font from {}", path.display()));
                    return Some(("cjk_fallback".into(), bytes));
                }
                Err(err) => log_warn(format!(
                    "Skipping unsupported font {}: {err}",
                    path.display()
                )),
            },
            Err(err) => log_warn(format!("Failed to read font {}: {err}", path.display())),
        }
    }
    log_warn("No CJK font found; Traditional Chinese UI may require manual font install");
    None
}

fn main() -> eframe::Result<()> {
    install_panic_logger();
    log_info("Starting RustNotePad GUI preview");
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 760.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|_cc| Box::<RustNotePadApp>::default()),
    );
    if let Err(err) = &result {
        log_error(format!("eframe shutdown error: {err}"));
    } else {
        log_info("RustNotePad GUI preview closed cleanly");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_to_traditional_chinese_locale_does_not_panic() {
        // The GUI reads localization files relative to the workspace root at runtime.
        // （圖形介面在執行時會相對於工作區根目錄讀取語系檔案。）
        // Tests run with the crate directory as the CWD, so adjust to ensure assets resolve.
        // （測試以 crate 目錄為目前路徑執行，因此需調整路徑以正確載入資源。）
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().expect("workspace root");
        std::env::set_current_dir(workspace_root).expect("set cwd to workspace root");

        let mut app = RustNotePadApp::default();
        app.cjk_font_available = true;
        let summaries = app.localization.locale_summaries();
        let zh_tw_index = summaries
            .iter()
            .enumerate()
            .find_map(|(idx, summary)| (summary.code == "zh-TW").then_some(idx))
            .expect("zh-TW locale available");
        app.apply_locale_change(zh_tw_index, &summaries);
        assert_eq!(app.selected_locale, zh_tw_index);
    }

    #[test]
    fn switching_to_traditional_chinese_without_cjk_font_is_blocked() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().expect("workspace root");
        std::env::set_current_dir(workspace_root).expect("set cwd to workspace root");

        let mut app = RustNotePadApp::default();
        app.cjk_font_available = false;
        let summaries = app.localization.locale_summaries();
        let zh_tw_index = summaries
            .iter()
            .enumerate()
            .find_map(|(idx, summary)| (summary.code == "zh-TW").then_some(idx))
            .expect("zh-TW locale available");
        let original_index = app.selected_locale;

        app.apply_locale_change(zh_tw_index, &summaries);

        assert_eq!(app.selected_locale, original_index);
        assert!(
            app.font_warning.is_some(),
            "expected a font warning when CJK font is unavailable"
        );
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsPage {
    Preferences,
    StyleConfigurator,
    ShortcutMapper,
    ContextMenu,
}

#[derive(Debug, Clone)]
struct PreferencesState {
    autosave_enabled: bool,
    autosave_interval_minutes: u32,
    show_line_numbers: bool,
    highlight_active_line: bool,
}

impl Default for PreferencesState {
    fn default() -> Self {
        Self {
            autosave_enabled: true,
            autosave_interval_minutes: 5,
            show_line_numbers: true,
            highlight_active_line: true,
        }
    }
}
