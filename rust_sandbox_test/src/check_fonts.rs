use egui::FontDefinitions;

fn main() {
    let defs = FontDefinitions::default();
    for (name, _) in defs.font_data {
        println!("{}", name);
    }
}
