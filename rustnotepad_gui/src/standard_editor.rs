use egui::{Color32, vec2};
use crate::RustNotePadApp;
use rustnotepad_highlight::HighlightKind;

pub trait StandardEditorExt {
    fn render_standard_editor(&mut self, ui: &mut egui::Ui, editor_bg_color: Color32);
}

impl StandardEditorExt for RustNotePadApp {
    fn render_standard_editor(&mut self, ui: &mut egui::Ui, editor_bg_color: Color32) {
        let editor_font_size = self.theme_manager.active_theme().fonts.editor_size as f32;
        let editor_line_height = editor_font_size * 1.25;
        
        let previous_text = self.editor_preview.clone();
        let mut buffer = previous_text.clone();

        let line_count = {
            let count = buffer.chars().filter(|&c| c == '\n').count();
            if buffer.is_empty() { 1 } else if buffer.ends_with('\n') { count.max(1) } else { count + 1 }
        };

        let char_width_approx = editor_font_size * 0.6;
        let line_number_width = format!("{}", line_count).len() as f32 * char_width_approx + 16.0;

        egui::ScrollArea::vertical()
            .id_source("std_editor_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // 1. Gutter
                    let (gutter_rect, _) = ui.allocate_at_least(vec2(line_number_width, ui.available_height().max(100.0)), egui::Sense::hover());
                    ui.painter().rect_filled(gutter_rect, 0.0, editor_bg_color);

                    // 2. Text Area
                    ui.vertical(|ui| {
                        ui.set_min_width(ui.available_width());
                        
                        let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                                let mut layout_job = egui::text::LayoutJob::default();
                                let font_id = egui::FontId::new(editor_font_size, egui::FontFamily::Monospace);
                                let default_color = ui.visuals().text_color();

                                if let Ok(tokens) = self.highlight_registry.highlight(&self.current_language_id, string) {
                                    let mut last_idx = 0;
                                    for token in tokens {
                                        if token.range.start > last_idx {
                                            layout_job.append(&string[last_idx..token.range.start], 0.0, egui::text::TextFormat {
                                                font_id: font_id.clone(), color: default_color, line_height: Some(editor_line_height), valign: egui::Align::Center, ..Default::default()
                                            });
                                        }
                                        let mut format = egui::text::TextFormat {
                                            font_id: font_id.clone(), color: default_color, line_height: Some(editor_line_height), valign: egui::Align::Center, ..Default::default()
                                        };
                                        match &token.kind {
                                            HighlightKind::Keyword => format.color = Color32::from_rgb(197, 134, 192),
                                            HighlightKind::Comment => format.color = Color32::from_rgb(106, 153, 85),
                                            HighlightKind::String => format.color = Color32::from_rgb(206, 145, 120),
                                            HighlightKind::Number => format.color = Color32::from_rgb(181, 206, 168),
                                            HighlightKind::Operator => format.color = Color32::from_rgb(100, 100, 100),
                                            _ => {}
                                        }
                                        layout_job.append(&string[token.range.start..token.range.end], 0.0, format);
                                        last_idx = token.range.end;
                                    }
                                    if last_idx < string.len() {
                                        layout_job.append(&string[last_idx..], 0.0, egui::text::TextFormat {
                                            font_id: font_id.clone(), color: default_color, line_height: Some(editor_line_height), valign: egui::Align::Center, ..Default::default()
                                        });
                                    }
                                } else {
                                    layout_job.append(string, 0.0, egui::text::TextFormat {
                                        font_id: font_id, color: default_color, line_height: Some(editor_line_height), valign: egui::Align::Center, ..Default::default()
                                    });
                                }
                                layout_job.wrap.max_width = f32::INFINITY;
                                ui.fonts(|f| f.layout_job(layout_job))
                            };

                            let editor_id = ui.make_persistent_id("std_editor");
                            let edit_output = egui::TextEdit::multiline(&mut buffer)
                                .id(editor_id)
                                .layouter(&mut layouter)
                                .desired_width(f32::INFINITY)
                                .frame(false)
                                .show(ui);
                            
                            if edit_output.response.changed() && buffer != previous_text {
                                self.record_undo_snapshot(previous_text.clone());
                                self.apply_editor_text(buffer);
                            }
                            self.update_editor_selection(edit_output.cursor_range.map(|r| r.as_ccursor_range()));
                    });
                });
            });
    }
}
