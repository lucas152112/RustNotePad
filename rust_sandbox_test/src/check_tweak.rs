use egui::text::{LayoutJob, TextFormat};

fn main() {
    let _ = TextFormat {
        line_height: Some(12.0),
        ..Default::default()
    };
}
