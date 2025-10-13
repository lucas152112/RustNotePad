use eframe::egui;
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("RustNotePad"),
        ..Default::default()
    };
    eframe::run_native(
        "RustNotePad",
        options,
        Box::new(|_cc| Ok(Box::new(RustNotePad::default()))),
    )
}

#[derive(Default)]
struct RustNotePad {
    text: String,
    current_file: Option<PathBuf>,
}

impl eframe::App for RustNotePad {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_file();
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        self.save_file_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Cut").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Copy").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Paste").clicked() {
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let text_edit = egui::TextEdit::multiline(&mut self.text)
                .desired_width(f32::INFINITY)
                .desired_rows(25)
                .font(egui::TextStyle::Monospace);
            ui.add(text_edit);
        });
    }
}

impl RustNotePad {
    fn new_file(&mut self) {
        self.text.clear();
        self.current_file = None;
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text Files", &["txt"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.text = content;
                self.current_file = Some(path);
            }
        }
    }

    fn save_file(&mut self) {
        if let Some(path) = &self.current_file {
            let _ = std::fs::write(path, &self.text);
        } else {
            self.save_file_as();
        }
    }

    fn save_file_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text Files", &["txt"])
            .add_filter("All Files", &["*"])
            .save_file()
        {
            if std::fs::write(&path, &self.text).is_ok() {
                self.current_file = Some(path);
            }
        }
    }
}
