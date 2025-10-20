use eframe::{egui, App, Frame, NativeOptions};
use egui::{
    vec2, Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, RichText,
    TextStyle,
};
use once_cell::sync::Lazy;
use rustnotepad_autocomplete::{
    CompletionEngine, CompletionItem, CompletionRequest, DocumentIndex, DocumentWordsProvider,
};
use rustnotepad_function_list::{FunctionKind, ParserRegistry, RegexParser, RegexRule, TextRange};
use rustnotepad_highlight::LanguageRegistry;
use rustnotepad_settings::{
    Color, LayoutConfig, PaneLayout, PaneRole, ResolvedPalette, TabColorTag, TabView,
    ThemeDefinition, ThemeKind, ThemeManager,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const APP_TITLE: &str = "RustNotePad – UI Preview";
const SAMPLE_EDITOR_CONTENT: &str = r#"// RustNotePad UI Preview / RustNotePad 介面預覽
fn main() {
    let mut search_engine = SearchEngine::new("alpha beta gamma");
    let options = SearchOptions::new("beta");
    if let Some(hit) = search_engine.find(0, &options).expect("search") {
        println!("Found match at byte {}", hit.start);
    }
}
"#;

const PREVIEW_DOCUMENT_ID: &str = "preview.rs";
const PREVIEW_LANGUAGE_ID: &str = "rust";

#[derive(Clone, Copy)]
struct MenuSection {
    title: &'static str,
    items: &'static [&'static str],
}

impl MenuSection {
    const fn new(title: &'static str, items: &'static [&'static str]) -> Self {
        Self { title, items }
    }
}

#[derive(Clone, Copy)]
struct ProjectNode {
    name: &'static str,
    children: &'static [ProjectNode],
}

impl ProjectNode {
    const fn leaf(name: &'static str) -> Self {
        Self {
            name,
            children: &[],
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

static MENU_STRUCTURE: Lazy<Vec<MenuSection>> = Lazy::new(|| {
    vec![
        MenuSection::new(
            "File",
            &[
                "New",
                "Open...",
                "Save",
                "Save As...",
                "Save All",
                "Close",
                "Close All",
                "Exit",
            ],
        ),
        MenuSection::new(
            "Edit",
            &[
                "Undo",
                "Redo",
                "Cut",
                "Copy",
                "Paste",
                "Delete",
                "Select All",
                "Column Editor...",
            ],
        ),
        MenuSection::new(
            "Search",
            &[
                "Find...",
                "Find Next",
                "Find Previous",
                "Replace...",
                "Find in Files...",
                "Bookmark ▸",
            ],
        ),
        MenuSection::new(
            "View",
            &[
                "Toggle Full Screen",
                "Restore Default Zoom",
                "Document Map",
                "Function List",
                "Project Panel ▸",
            ],
        ),
        MenuSection::new(
            "Settings",
            &[
                "Preferences...",
                "Style Configurator...",
                "Shortcut Mapper...",
                "Edit Popup Context Menu...",
            ],
        ),
        MenuSection::new("Macro", &["Start Recording", "Stop Recording", "Playback"]),
        MenuSection::new("Run", &["Run...", "Launch in Chrome", "Launch in Firefox"]),
        MenuSection::new("Plugins", &["Plugins Admin...", "Open Plugins Folder..."]),
        MenuSection::new(
            "Window",
            &["Duplicate", "Clone to Other View", "Move to Other View"],
        ),
        MenuSection::new("Help", &["User Manual", "Debug Info", "About"]),
    ]
});

const CORE_FILES: &[ProjectNode] = &[
    ProjectNode::leaf("document.rs"),
    ProjectNode::leaf("editor.rs"),
    ProjectNode::leaf("search_session.rs"),
];

const SEARCH_FILES: &[ProjectNode] = &[
    ProjectNode::leaf("Cargo.toml"),
    ProjectNode::leaf("src/lib.rs"),
];

const CRATES_CHILDREN: &[ProjectNode] = &[
    ProjectNode {
        name: "core",
        children: CORE_FILES,
    },
    ProjectNode {
        name: "search",
        children: SEARCH_FILES,
    },
];

const FEATURE_PARITY_DOCS: &[ProjectNode] = &[
    ProjectNode::leaf("03-search-replace/design.md"),
    ProjectNode::leaf("04-view-interface/design.md"),
];

const DOCS_CHILDREN: &[ProjectNode] = &[ProjectNode {
    name: "feature_parity",
    children: FEATURE_PARITY_DOCS,
}];

const TEST_FILES: &[ProjectNode] = &[
    ProjectNode::leaf("search_workflow.rs"),
    ProjectNode::leaf("search_large_workspace.rs"),
];

static PROJECT_TREE: Lazy<Vec<ProjectNode>> = Lazy::new(|| {
    vec![
        ProjectNode {
            name: "crates",
            children: CRATES_CHILDREN,
        },
        ProjectNode {
            name: "docs",
            children: DOCS_CHILDREN,
        },
        ProjectNode {
            name: "tests",
            children: TEST_FILES,
        },
    ]
});

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

    fn set_ui_language(&mut self, locale: &str) {
        self.ui_language = locale.to_string();
    }

    fn set_theme(&mut self, theme_name: &str) {
        self.theme = theme_name.to_string();
    }
}

struct RustNotePadApp {
    layout: LayoutConfig,
    editor_preview: String,
    bottom_tab_index: usize,
    available_locales: Vec<&'static str>,
    selected_locale: usize,
    theme_manager: ThemeManager,
    palette: ResolvedPalette,
    status: StatusBarState,
    pending_theme_refresh: bool,
    fonts_installed: bool,
    font_warning: Option<String>,
    highlight_registry: LanguageRegistry,
    function_registry: ParserRegistry,
    document_index: Arc<DocumentIndex>,
    autocomplete_engine: CompletionEngine,
    completion_prefix: String,
    completion_results: Vec<CompletionItem>,
}

impl Default for RustNotePadApp {
    fn default() -> Self {
        let theme_manager = ThemeManager::load_from_dir("assets/themes").unwrap_or_else(|_| {
            ThemeManager::new(vec![
                ThemeDefinition::builtin_dark(),
                ThemeDefinition::builtin_light(),
            ])
            .expect("built-in themes")
        });
        let palette = theme_manager.active_palette().clone();
        let layout = LayoutConfig::default();
        let locales = vec!["English (en-US)", "正體中文 (zh-TW)", "Deutsch (de-DE)"];
        let selected_locale = 0usize;
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

        let status = StatusBarState::new(
            &layout,
            &theme_manager.active_theme().name,
            locales[selected_locale],
        );

        let highlight_registry = LanguageRegistry::with_defaults();
        let function_registry = build_function_registry();
        let document_index = Arc::new(DocumentIndex::new());
        document_index.update_document(PREVIEW_DOCUMENT_ID, SAMPLE_EDITOR_CONTENT);
        let mut autocomplete_engine = CompletionEngine::new();
        autocomplete_engine.register_provider(
            "document_words",
            5,
            DocumentWordsProvider::new(document_index.clone())
                .with_prefix_minimum(1)
                .with_max_items(40),
        );
        let completion_prefix = "ma".to_string();

        let mut app = Self {
            layout,
            editor_preview: SAMPLE_EDITOR_CONTENT.to_string(),
            bottom_tab_index: active_panel_index,
            available_locales: locales,
            selected_locale,
            theme_manager,
            palette,
            status,
            pending_theme_refresh: true,
            fonts_installed: false,
            font_warning: None,
            highlight_registry,
            function_registry,
            document_index,
            autocomplete_engine,
            completion_prefix,
            completion_results: Vec::new(),
        };
        app.status.refresh_cursor(&app.editor_preview);
        app.refresh_completions();
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
        let request = CompletionRequest::new(
            Some(PREVIEW_DOCUMENT_ID.to_string()),
            self.completion_prefix.clone(),
        )
        .with_max_items(16);
        let result = self.autocomplete_engine.request(request);
        self.completion_results = result.items;
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
        } else if self.font_warning.is_none() {
            self.font_warning = Some(
                "未找到系統內建的正體中文字型。請安裝（或放置於 assets/fonts/）Noto Sans TC、微軟正黑體、蘋方等支援正體中文的字型後重啟 RustNotePad。"
                    .into(),
            );
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
                        ui.menu_button(section.title, |ui| {
                            for item in section.items {
                                ui.add_enabled(false, egui::Button::new(*item));
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
                    ui.label(RichText::new("Workspace: rustnotepad").strong());
                    ui.separator();

                    let theme_names: Vec<&str> = self.theme_manager.theme_names().collect();
                    let mut active_index = self.theme_manager.active_index();
                    egui::ComboBox::from_id_source("theme_selector")
                        .width(200.0)
                        .selected_text(theme_names[active_index])
                        .show_ui(ui, |ui| {
                            for (idx, name) in theme_names.iter().enumerate() {
                                let selected = idx == active_index;
                                if ui.selectable_label(selected, *name).clicked() {
                                    active_index = idx;
                                }
                            }
                        });
                    if active_index != self.theme_manager.active_index() {
                        if self.theme_manager.set_active_index(active_index).is_some() {
                            self.pending_theme_refresh = true;
                        }
                    }

                    ui.separator();
                    let mut locale_index = self.selected_locale;
                    egui::ComboBox::from_id_source("locale_selector")
                        .width(180.0)
                        .selected_text(self.available_locales[locale_index])
                        .show_ui(ui, |ui| {
                            for (idx, locale) in self.available_locales.iter().enumerate() {
                                let selected = idx == locale_index;
                                if ui.selectable_label(selected, *locale).clicked() {
                                    locale_index = idx;
                                }
                            }
                        });
                    if locale_index != self.selected_locale {
                        self.selected_locale = locale_index;
                        self.status
                            .set_ui_language(self.available_locales[self.selected_locale]);
                    }

                    ui.separator();
                    let mut ratio = self.layout.split_ratio;
                    if ui
                        .add(
                            egui::Slider::new(&mut ratio, 0.3..=0.7)
                                .prefix("Split ")
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
                        ui.label(format!("Pinned tabs: {pinned_count}"));
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
                    ui.heading("Project Panel / 專案面板");
                    ui.separator();
                    for node in PROJECT_TREE.iter() {
                        self.render_project_node(ui, node, 0);
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_function_list_panel(ui);
                    ui.add_space(10.0);
                    ui.separator();
                    self.render_completion_panel(ui);
                });
            });
    }

    fn show_right_sidebar(&self, ctx: &egui::Context) {
        egui::SidePanel::right("document_map")
            .default_width(180.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Document Map");
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
                let panels = &self.layout.bottom_dock.visible_panels;
                if panels.is_empty() {
                    ui.label("No panels configured.");
                    return;
                }

                ui.horizontal(|ui| {
                    for (idx, panel) in panels.iter().enumerate() {
                        let title = match panel.as_str() {
                            "find_results" => "Find Results",
                            "console" => "Console",
                            "notifications" => "Notifications",
                            "lsp" => "LSP Diagnostics",
                            other => other,
                        };
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

                let active_panel = panels
                    .get(self.bottom_tab_index)
                    .cloned()
                    .or_else(|| self.layout.bottom_dock.active_panel.clone())
                    .unwrap_or_else(|| "find_results".into());

                match active_panel.as_str() {
                    "find_results" => {
                        ui.label("Find Results: 5 hits across 3 files.");
                        ui.label("search/src/lib.rs:42  let matches = engine.find_all(&options)?;");
                        ui.label(
                            "core/src/search_session.rs:155  document.set_contents(replaced_text);",
                        );
                    }
                    "console" => {
                        ui.label("cargo check --workspace");
                        ui.label("Finished dev [unoptimized + debuginfo] target(s) in 2.34s");
                    }
                    "notifications" => {
                        ui.label("✔ All background tasks are idle.");
                        ui.label("⚠ Theme 'Nordic Daylight' missing custom font, using fallback.");
                    }
                    "lsp" => {
                        ui.label("rust-analyzer connected");
                        ui.label("Diagnostics: none.");
                    }
                    other => {
                        ui.label(format!("Panel '{other}' has no content in preview mode."));
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
                    ui.label(format!(
                        "Ln {}, Col {} | Lines {}",
                        self.status.line, self.status.column, self.status.lines
                    ));
                    ui.separator();
                    ui.label(format!("Sel {}", self.status.selection));
                    ui.separator();
                    ui.label(self.status.mode);
                    ui.separator();
                    ui.label(self.status.encoding);
                    ui.separator();
                    ui.label(self.status.eol);
                    ui.separator();
                    ui.label(format!("Lang: {}", self.status.document_language));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.label(format!("UI: {}", self.status.ui_language));
                    ui.separator();
                    ui.label(format!("Theme: {}", self.status.theme));
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
                                                PREVIEW_DOCUMENT_ID,
                                                &self.editor_preview,
                                            );
                                            self.refresh_completions();
                                        }

                                        self.render_editor_insights(ui);
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
                                    ui.label("Secondary View (preview)");
                                    ui.add_space(6.0);
                                    if let Some(active) = pane.active_tab() {
                                        ui.label(format!("Active: {}", active.title));
                                        ui.label("Preview is read-only in UI mock.");
                                    }
                                });
                            },
                        );
                    }
                },
            );
        });
    }

    fn render_editor_insights(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(RichText::new("Editor Insights / 編輯器分析").strong());
        ui.add_space(6.0);
        ui.columns(3, |columns| {
            self.render_highlight_summary(&mut columns[0]);
            self.render_function_list_panel(&mut columns[1]);
            self.render_completion_panel(&mut columns[2]);
        });
    }

    fn render_highlight_summary(&self, ui: &mut egui::Ui) {
        ui.heading("Syntax Highlight / 語法高亮摘要");
        match self
            .highlight_registry
            .highlight(PREVIEW_LANGUAGE_ID, &self.editor_preview)
        {
            Ok(tokens) => {
                ui.label(format!("Tokens: {}", tokens.len()));
                let mut counts: BTreeMap<String, usize> = BTreeMap::new();
                for token in &tokens {
                    *counts.entry(format!("{:?}", token.kind)).or_insert(0) += 1;
                }
                for (kind, count) in counts.iter() {
                    ui.label(format!("{kind}: {count}"));
                }
                ui.add_space(6.0);
                ui.label(RichText::new("Sample / 範例").italics());
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
                    ui.label("…");
                }
            }
            Err(err) => {
                ui.colored_label(
                    Color32::from_rgb(239, 68, 68),
                    format!("Highlight error: {err}"),
                );
            }
        }
    }

    fn render_function_list_panel(&self, ui: &mut egui::Ui) {
        ui.heading("Function List / 函式清單");
        match self
            .function_registry
            .parse(PREVIEW_LANGUAGE_ID, &self.editor_preview)
        {
            Some(entries) if !entries.is_empty() => {
                for entry in entries.iter().take(10) {
                    let line = line_number_for_range(&self.editor_preview, &entry.range);
                    ui.label(format!("{:?} {} (Ln {line})", entry.kind, entry.name));
                }
                if entries.len() > 10 {
                    let remaining = entries.len() - 10;
                    ui.label(format!(
                        "… {remaining} additional symbols / 另有 {remaining} 個符號"
                    ));
                }
            }
            _ => {
                ui.label("No symbols detected. / 尚未偵測到任何符號。");
            }
        }
    }

    fn render_completion_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Autocomplete / 自動完成");
        let mut prefix = self.completion_prefix.clone();
        let response = ui.add(
            egui::TextEdit::singleline(&mut prefix)
                .hint_text("Prefix / 前綴")
                .desired_width(f32::INFINITY),
        );
        if response.changed() {
            self.completion_prefix = prefix;
            self.refresh_completions();
        }

        if self.completion_results.is_empty() {
            ui.label("No suggestions available. / 目前沒有建議。");
        } else {
            for item in self.completion_results.iter().take(10) {
                let kind = format!("{:?}", item.kind);
                let detail = item.detail.as_deref().unwrap_or("");
                if detail.is_empty() {
                    ui.label(format!("{kind} — {}", item.label));
                } else {
                    ui.label(format!("{kind} — {} ({detail})", item.label));
                }
            }
            if self.completion_results.len() > 10 {
                let remaining = self.completion_results.len() - 10;
                ui.label(format!(
                    "… {remaining} more suggestions / 另有 {remaining} 筆建議"
                ));
            }
        }
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
                if self.layout.set_active_tab(role, &tab.id) {
                    self.status.refresh_from_layout(&self.layout);
                }
            }
        });
        ui.add_space(6.0);
    }

    fn render_project_node(&self, ui: &mut egui::Ui, node: &ProjectNode, depth: usize) {
        let indent = "    ".repeat(depth);
        if node.is_leaf() {
            ui.label(format!("{indent}{}", node.name));
        } else {
            egui::CollapsingHeader::new(format!("{indent}{}", node.name))
                .default_open(depth < 2)
                .show(ui, |ui| {
                    for child in node.children.iter() {
                        self.render_project_node(ui, child, depth + 1);
                    }
                });
        }
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
    }
}

fn color32_from_color(color: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
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

    for path in candidates.into_iter().filter(|p| p.exists()) {
        if let Ok(bytes) = fs::read(&path) {
            return Some(("cjk_fallback".into(), bytes));
        }
    }
    None
}

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|_cc| Box::<RustNotePadApp>::default()),
    )
}
