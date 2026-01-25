use egui::ScrollArea;
use crate::RustNotePadApp;

pub trait ImageViewerExt {
    fn render_image_viewer(&mut self, ui: &mut egui::Ui, texture: &egui::TextureHandle);
}

impl ImageViewerExt for RustNotePadApp {
    fn render_image_viewer(&mut self, ui: &mut egui::Ui, texture: &egui::TextureHandle) {
        // Handle Zoom Logic (Ctrl + Scroll)
        ui.input_mut(|i| {
            let delta = i.scroll_delta.y;
            if (i.modifiers.ctrl || i.modifiers.command) && delta != 0.0 {
                let zoom_step = 1.1f32;
                if delta > 0.0 {
                    self.image_zoom *= zoom_step;
                } else {
                    self.image_zoom /= zoom_step;
                }
                i.scroll_delta = egui::Vec2::ZERO;
            }
            
            let zoom_delta = i.zoom_delta();
            if zoom_delta != 1.0 {
                self.image_zoom *= zoom_delta;
            }

            if (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Num0) {
                self.image_zoom = 1.0;
            }
        });

        ScrollArea::both()
            .id_source("image_viewer_scroll")
            .show(ui, |ui| {
                let size = texture.size_vec2() * self.image_zoom;
                ui.centered_and_justified(|ui| {
                    ui.add(egui::Image::new(texture).fit_to_exact_size(size));
                });
            });
    }
}
