use crate::dsp::processors::PROCESSOR_NAMES;

pub struct ChainEditorOutput {
    pub add_name: Option<String>,
    pub remove_idx: Option<usize>,
}

pub struct ChainEditor {
    selected_add_idx: usize,
}

impl ChainEditor {
    pub fn new() -> Self {
        Self {
            selected_add_idx: 0,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, current_names: &[String]) -> ChainEditorOutput {
        let mut output = ChainEditorOutput {
            add_name: None,
            remove_idx: None,
        };

        ui.horizontal_wrapped(|ui| {
            ui.label("Chain:");

            for (i, name) in current_names.iter().enumerate() {
                if i > 0 {
                    ui.label("→");
                }
                let btn = egui::Button::new(format!("✕ {name}")).small();
                if ui.add(btn).clicked() {
                    output.remove_idx = Some(i);
                }
            }

            ui.separator();

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

impl Default for ChainEditor {
    fn default() -> Self {
        Self::new()
    }
}
