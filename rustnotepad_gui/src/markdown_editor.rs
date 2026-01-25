use egui::{Color32, vec2, Margin};
use crate::RustNotePadApp;
use rustnotepad_highlight::HighlightKind;

pub trait MarkdownEditorExt {
    fn render_markdown_editor(&mut self, ui: &mut egui::Ui, editor_bg_color: Color32);
}

impl MarkdownEditorExt for RustNotePadApp {
    fn render_markdown_editor(&mut self, ui: &mut egui::Ui, editor_bg_color: Color32) {
        let editor_font_size = self.theme_manager.active_theme().fonts.editor_size as f32;
        let editor_line_height = editor_font_size * 1.5; 
        
        let previous_text = self.editor_preview.clone();
        let mut buffer = previous_text.clone();

        let line_count = {
            let count = buffer.chars().filter(|&c| c == '\n').count();
            if buffer.is_empty() { 1 } else if buffer.ends_with('\n') { count.max(1) } else { count + 1 }
        };

        let char_width_approx = editor_font_size * 0.6;
        let line_number_width = format!("{}", line_count).len() as f32 * char_width_approx + 16.0;
        let md_margin = Margin::symmetric(30.0, 5.0); // Reduced vertical as main.rs now has 10.0
        
        egui::Frame::none()
            .inner_margin(md_margin)
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());

                egui::ScrollArea::vertical()
                    .id_source("md_editor_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Use a horizontal layout for Gutter + Text
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            
                            // 1. Gutter (Line numbers column)
                            ui.allocate_ui_with_layout(
                                vec2(line_number_width, ui.available_height().max(10.0)),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, editor_bg_color);
                                }
                            );

                            // 2. Text Area
                            ui.vertical(|ui| {
                                ui.set_min_width(ui.available_width());
                                
                                let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                                    let mut layout_job = egui::text::LayoutJob::default();
                                    let font_id = egui::FontId::new(editor_font_size, egui::FontFamily::Monospace);
                                    let default_color = ui.visuals().text_color();

                                    let base_format = egui::text::TextFormat {
                                        font_id: font_id.clone(),
                                        color: default_color,
                                        line_height: Some(editor_line_height),
                                        valign: egui::Align::Center,
                                        ..Default::default()
                                    };

                                    if let Ok(tokens) = self.highlight_registry.highlight(&self.current_language_id, string) {
                                        let mut last_idx = 0;
                                        for token in tokens {
                                            if token.range.start > last_idx {
                                                layout_job.append(&string[last_idx..token.range.start], 0.0, base_format.clone());
                                            }
                                            let mut format = base_format.clone();
                                            match &token.kind {
                                                HighlightKind::Custom(s) => match s.as_str() {
                                                    "h1" => {
                                                        format.color = Color32::from_rgb(0, 191, 255);
                                                        format.font_id = egui::FontId::new(editor_font_size * 1.05, egui::FontFamily::Monospace);
                                                        format.line_height = Some(editor_line_height * 1.05);
                                                        format.extra_letter_spacing = 0.2;
                                                    },
                                                    "h2" => {
                                                        format.color = Color32::from_rgb(100, 200, 255);
                                                        format.font_id = egui::FontId::new(editor_font_size * 1.03, egui::FontFamily::Monospace);
                                                        format.line_height = Some(editor_line_height * 1.03);
                                                    },
                                                    "h3" => {
                                                        format.color = Color32::from_rgb(100, 255, 218);
                                                        format.font_id = egui::FontId::new(editor_font_size * 1.01, egui::FontFamily::Monospace);
                                                        format.line_height = Some(editor_line_height * 1.01);
                                                    },
                                                    "md_code" => {
                                                        // 代碼片段：反引號保持普通文本，內容用棕色顯示
                                                        let slice = &string[token.range.start..token.range.end];
                                                        if slice.starts_with('`') && slice.ends_with('`') && slice.len() >= 2 {
                                                            layout_job.append("`", 0.0, base_format.clone());
                                                            if slice.len() > 2 {
                                                                let code_format = egui::text::TextFormat {
                                                                    color: Color32::from_rgb(206, 145, 120),
                                                                    ..base_format.clone()
                                                                };
                                                                layout_job.append(&slice[1..slice.len() - 1], 0.0, code_format);
                                                            }
                                                            layout_job.append("`", 0.0, base_format.clone());
                                                            last_idx = token.range.end;
                                                            continue;
                                                        }
                                                    },
                                                    _ => {
                                                        // 保持默認顏色
                                                    }
                                                },
                                                HighlightKind::Keyword => format.color = Color32::from_rgb(197, 134, 192),
                                                HighlightKind::Comment => format.color = Color32::from_rgb(106, 153, 85),
                                                HighlightKind::String => {
                                                    format.color = Color32::from_rgb(206, 145, 120);
                                                },
                                                HighlightKind::Number => format.color = Color32::from_rgb(181, 206, 168),
                                                HighlightKind::Operator => {
                                                    // 操作符保持默認顏色（普通文本）
                                                    format.color = default_color;
                                                },
                                                _ => {
                                                    // 所有其他情況保持默認顏色
                                                }
                                            }
                                            layout_job.append(&string[token.range.start..token.range.end], 0.0, format.clone());
                                            last_idx = token.range.end;
                                        }
                                        if last_idx < string.len() {
                                            layout_job.append(&string[last_idx..], 0.0, base_format.clone());
                                        }
                                    } else {
                                        layout_job.append(string, 0.0, base_format);
                                    }
                                    layout_job.wrap.max_width = f32::INFINITY;
                                    ui.fonts(|f| f.layout_job(layout_job))
                                };

                                let editor_id = ui.make_persistent_id("md_editor");
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
            });
    }
}
