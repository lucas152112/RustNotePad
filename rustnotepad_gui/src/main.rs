use eframe::{egui, App, Frame, NativeOptions};
use egui::{Align, Color32, Layout, RichText};
use once_cell::sync::Lazy;

const APP_TITLE: &str = "RustNotePad – UI Preview";

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
struct EditorTab {
    title: &'static str,
    _language: &'static str,
    is_active: bool,
    is_dirty: bool,
}

impl EditorTab {
    const fn new(
        title: &'static str,
        language: &'static str,
        is_active: bool,
        is_dirty: bool,
    ) -> Self {
        Self {
            title,
            _language: language,
            is_active,
            is_dirty,
        }
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
                "Line Operations ▸",
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
                "Find All in Current Doc",
                "Bookmark ▸",
            ],
        ),
        MenuSection::new(
            "View",
            &[
                "Toggle Full Screen",
                "Restore Default Zoom",
                "Post-It",
                "Always on Top",
                "Document Map",
                "Function List",
                "Project Panel ▸",
            ],
        ),
        MenuSection::new(
            "Encoding",
            &[
                "Encode in ANSI",
                "Encode in UTF-8",
                "Encode in UTF-8-BOM",
                "Encode in UTF-16 LE",
                "Convert to UTF-8",
                "Character Sets ▸",
            ],
        ),
        MenuSection::new(
            "Language",
            &[
                "Auto-Detect",
                "Plain Text",
                "C",
                "C++",
                "Rust",
                "Python",
                "JavaScript",
                "User Defined Language...",
            ],
        ),
        MenuSection::new(
            "Settings",
            &[
                "Preferences...",
                "Style Configurator...",
                "Shortcut Mapper...",
                "Edit Popup Context Menu...",
                "Import ▸",
            ],
        ),
        MenuSection::new(
            "Macro",
            &[
                "Start Recording",
                "Stop Recording",
                "Playback",
                "Run a Macro Multiple Times...",
                "Modify Shortcut...",
            ],
        ),
        MenuSection::new(
            "Run",
            &[
                "Run...",
                "Launch in Chrome",
                "Launch in Firefox",
                "Open Containing Folder",
                "CMD Here",
                "External Tools ▸",
            ],
        ),
        MenuSection::new(
            "Plugins",
            &["Plugins Admin...", "Plugin Manager", "Plugin Console", "WASM Extensions ▸"],
        ),
        MenuSection::new(
            "Window",
            &[
                "Duplicate",
                "Clone to Other View",
                "Move to New Instance",
                "Close All Others",
                "Windows...",
            ],
        ),
        MenuSection::new("?", &["About RustNotePad", "Documentation", "Check for Updates..."]),
    ]
});

static TOOLBAR_PRIMARY: &[&str] = &[
    "New",
    "Open",
    "Save",
    "Save All",
    "Print",
    "Undo",
    "Redo",
    "Cut",
    "Copy",
    "Paste",
    "Find",
    "Replace",
    "Macro",
    "Run",
];

static TOOLBAR_SECONDARY: &[&str] = &[
    "Start Recording",
    "Playback",
    "Project Panel",
    "Function List",
    "Doc Switcher",
    "Document Map",
    "Console",
    "Session Manager",
];

static OPEN_TABS: Lazy<Vec<EditorTab>> = Lazy::new(|| {
    vec![
        EditorTab::new("welcome.md", "Markdown", false, false),
        EditorTab::new("core/buffer.rs", "Rust", true, true),
        EditorTab::new("search/engine.rs", "Rust", false, false),
        EditorTab::new("themes/dark.toml", "TOML", false, false),
        EditorTab::new("sessions/default.session", "Session", false, false),
    ]
});

static PROJECT_TREE: &[ProjectNode] = &[
    ProjectNode {
        name: "rustnotepad",
        children: &[
            ProjectNode::leaf("Cargo.toml"),
            ProjectNode {
                name: "apps",
                children: &[
                    ProjectNode {
                        name: "gui-tauri",
                        children: &[ProjectNode::leaf("main.rs"), ProjectNode::leaf("lib.rs")],
                    },
                    ProjectNode {
                        name: "cli",
                        children: &[ProjectNode::leaf("main.rs")],
                    },
                ],
            },
            ProjectNode {
                name: "crates",
                children: &[
                    ProjectNode {
                        name: "core",
                        children: &[
                            ProjectNode::leaf("lib.rs"),
                            ProjectNode::leaf("buffer.rs"),
                            ProjectNode::leaf("undo.rs"),
                        ],
                    },
                    ProjectNode {
                        name: "search",
                        children: &[ProjectNode::leaf("lib.rs"), ProjectNode::leaf("engine.rs")],
                    },
                    ProjectNode {
                        name: "highlight",
                        children: &[ProjectNode::leaf("lib.rs"), ProjectNode::leaf("themes.rs")],
                    },
                ],
            },
            ProjectNode {
                name: "assets",
                children: &[
                    ProjectNode::leaf("Default (Dark).theme"),
                    ProjectNode::leaf("Light.theme"),
                    ProjectNode::leaf("langs/en-US.toml"),
                ],
            },
        ],
    },
];

static FUNCTION_LIST: &[&str] = &[
    "fn main()",
    "fn setup_logging()",
    "fn build_menu()",
    "fn highlight_selection()",
    "struct Editor",
    "impl Editor::new",
    "impl Editor::handle_action",
];

static DOC_SWITCHER_ITEMS: &[&str] = &[
    "welcome.md",
    "README.md",
    "core/buffer.rs",
    "search/engine.rs",
    "highlight/themes.rs",
    "session/workspace.json",
];

static BOTTOM_TABS: &[&str] = &["Find Results", "Console Output", "Notifications", "LSP Diagnostics"];

static SAMPLE_EDITOR_CONTENT: &str = r#"// RustNotePad UI Preview
fn main() {
    println!("Hello, RustNotePad!");
    println!("This UI preview shows menus, toolbars, tabs, side panels, and status bar.");
    println!("Implement actual editor logic inside the core crates.");
}

// TODO: Wire up events, text buffer, syntax highlighting, search, macros, and plugins.
"#;

struct RustNotePadApp {
    bottom_tab_index: usize,
    editor_preview: String,
}

impl Default for RustNotePadApp {
    fn default() -> Self {
        Self {
            bottom_tab_index: 0,
            editor_preview: SAMPLE_EDITOR_CONTENT.to_string(),
        }
    }
}

impl App for RustNotePadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.show_menu_bar(ctx);
        self.show_primary_toolbar(ctx);
        self.show_secondary_toolbar(ctx);
        self.show_left_docks(ctx);
        self.show_right_docks(ctx);
        self.show_bottom_dock(ctx);
        self.show_status_bar(ctx);
        self.show_editor(ctx);
    }
}

impl RustNotePadApp {
    fn show_menu_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar")
            .min_height(22.0)
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    for section in MENU_STRUCTURE.iter() {
                        ui.menu_button(section.title, |ui| {
                            for item in section.items.iter() {
                                ui.add_enabled(false, egui::Button::new(*item));
                            }
                        });
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Workspace: default").weak());
                    });
                });
            });
    }

    fn show_primary_toolbar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar_primary")
            .min_height(28.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for action in TOOLBAR_PRIMARY.iter() {
                        ui.add_enabled(false, egui::Button::new(*action));
                    }
                });
            });
    }

    fn show_secondary_toolbar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar_secondary")
            .min_height(24.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for action in TOOLBAR_SECONDARY.iter() {
                        ui.add_enabled(false, egui::Button::new(*action));
                    }
                });
            });
    }

    fn show_left_docks(&self, ctx: &egui::Context) {
        egui::SidePanel::left("left_dock")
            .resizable(true)
            .min_width(220.0)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Project Panel");
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    for node in PROJECT_TREE.iter() {
                        self.render_project_node(ui, node, 0);
                    }
                });
                ui.separator();
                ui.heading("Function List");
                ui.separator();
                egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                    for item in FUNCTION_LIST.iter() {
                        ui.label(*item);
                    }
                });
                ui.separator();
                ui.heading("Doc Switcher");
                ui.separator();
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    for doc in DOC_SWITCHER_ITEMS.iter() {
                        ui.label(*doc);
                    }
                });
            });
    }

    fn show_right_docks(&self, ctx: &egui::Context) {
        egui::SidePanel::right("right_dock")
            .resizable(true)
            .min_width(180.0)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Document Map");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("▮▮▮▮▮▮▮▮▮▮▮");
                    ui.label("░░░░░░░░░░░░░░░░");
                    ui.label("▮▮▮▮▮▮▮▮▮▮▮");
                    ui.label("░░░░░░░░░░░░░░░░");
                });
                ui.separator();
                ui.heading("Outline Preview");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("main");
                    ui.label("├─ setup_logging");
                    ui.label("├─ build_menu");
                    ui.label("└─ highlight_selection");
                });
            });
    }

    fn show_bottom_dock(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_dock")
            .min_height(160.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (index, title) in BOTTOM_TABS.iter().enumerate() {
                        let selected = self.bottom_tab_index == index;
                        if ui
                            .selectable_label(selected, *title)
                            .clicked()
                        {
                            self.bottom_tab_index = index;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Output Panel").weak());
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| match self.bottom_tab_index {
                    0 => {
                        ui.label("Find All: 12 hits in 4 files.");
                        ui.label("core/buffer.rs:42  let cursor = Cursor::new(position);");
                        ui.label("core/buffer.rs:88  buffer.find(text);");
                        ui.label("search/engine.rs:13  pub fn find_all()");
                    }
                    1 => {
                        ui.label("Running build task...");
                        ui.label("cargo check --workspace");
                        ui.label("Status: queued (UI preview only)");
                    }
                    2 => {
                        ui.label("No notifications. System is idle.");
                    }
                    3 => {
                        ui.label("LSP connected to rust-analyzer.");
                        ui.label("Diagnostics: none.");
                    }
                    _ => {
                        ui.label("Unknown panel");
                    }
                });
            });
    }

    fn show_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(24.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label("Ln 128, Col 21");
                    ui.separator();
                    ui.label("Sel 0 | INS");
                    ui.separator();
                    ui.label("UTF-8");
                    ui.separator();
                    ui.label("LF");
                    ui.separator();
                    ui.label("Rust");
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label("Zoom: 100%");
                    ui.separator();
                    ui.label("64-bit");
                    ui.separator();
                    ui.label("Ready");
                });
            });
    }

    fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_tab_strip(ui);
            ui.separator();
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(24, 24, 28))
                .stroke(egui::Stroke::new(1.0, Color32::from_gray(80)))
                .show(ui, |ui| {
                    ui.style_mut().visuals.extreme_bg_color = Color32::from_rgb(18, 18, 20);
                    let mut buffer = self.editor_preview.clone();
                    let text_edit = egui::TextEdit::multiline(&mut buffer)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(22)
                        .desired_width(f32::INFINITY);
                    let response = ui.add_sized(ui.available_size(), text_edit);
                    if response.changed() {
                        self.editor_preview = buffer;
                    }
                });
        });
    }

    fn show_tab_strip(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in OPEN_TABS.iter() {
                    let mut label = tab.title.to_string();
                    if tab.is_dirty {
                        label.push('*');
                    }
                    let rich = if tab.is_active {
                        RichText::new(label).color(Color32::from_rgb(240, 240, 240))
                    } else {
                        RichText::new(label).color(Color32::from_rgb(180, 180, 180))
                    };
                    ui.add_enabled(false, egui::SelectableLabel::new(tab.is_active, rich));
                    ui.separator();
                }
            });
        });
        ui.label(RichText::new("Language: Rust   |   Theme: Dark").weak());
    }

    fn render_project_node(&self, ui: &mut egui::Ui, node: &ProjectNode, depth: usize) {
        let indent = "    ".repeat(depth);
        if node.children.is_empty() {
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

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|_cc| Box::<RustNotePadApp>::default()),
    )
}
