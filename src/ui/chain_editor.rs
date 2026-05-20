use crate::dsp::processors::PROCESSOR_NAMES;
use crate::ui::theme::{SURFACE_HIGH, TEXT_MUTED, TEXT_PRIMARY, processor_color};

pub struct ChainEditorOutput {
    pub add_name: Option<String>,
    pub remove_idx: Option<usize>,
    pub selected_idx: Option<usize>,
}

pub struct ChainEditor {
    selected_add_idx: usize,
}

impl Default for ChainEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainEditor {
    pub fn new() -> Self {
        Self {
            selected_add_idx: 0,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        current_names: &[String],
        selected: Option<usize>,
    ) -> ChainEditorOutput {
        let mut output = ChainEditorOutput {
            add_name: None,
            remove_idx: None,
            selected_idx: None,
        };

        for (i, name) in current_names.iter().enumerate() {
            let is_selected = selected == Some(i);
            let color = processor_color(name);
            let bg = if is_selected {
                SURFACE_HIGH
            } else {
                egui::Color32::TRANSPARENT
            };

            egui::Frame::new()
                .fill(bg)
                .corner_radius(egui::CornerRadius::same(4))
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui
                            .allocate_exact_size(egui::Vec2::new(4.0, 18.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(2), color);
                        ui.add_space(4.0);

                        if ui
                            .selectable_label(
                                is_selected,
                                egui::RichText::new(name).color(TEXT_PRIMARY),
                            )
                            .clicked()
                        {
                            output.selected_idx = Some(i);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("×").clicked() {
                                output.remove_idx = Some(i);
                            }
                        });
                    });
                });
        }

        if current_names.is_empty() {
            ui.label(
                egui::RichText::new("No processors yet")
                    .color(TEXT_MUTED)
                    .size(11.0),
            );
        }

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("add_processor_combo")
                .selected_text(PROCESSOR_NAMES[self.selected_add_idx])
                .show_ui(ui, |ui| {
                    for (i, &name) in PROCESSOR_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_add_idx, i, name);
                    }
                });
            if ui.button("+ Add").clicked() {
                output.add_name = Some(PROCESSOR_NAMES[self.selected_add_idx].to_string());
            }
        });

        output
    }
}
