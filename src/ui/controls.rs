use egui::Vec2;

use crate::dsp::chain::{ParamUpdate, ProcessorChain};
use crate::dsp::processor::Processor;
use crate::ui::widgets::knob::knob;

pub struct ParamState {
    pub name: &'static str,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: &'static str,
}

pub struct ProcessorState {
    pub name: String,
    pub params: Vec<ParamState>,
}

pub struct ControlsOutput {
    pub updates: Vec<ParamUpdate>,
    pub selected_proc: usize,
    pub selection_changed: bool,
}

#[derive(Default)]
pub struct Controls {
    processors: Vec<ProcessorState>,
    selected_proc: usize,
}

impl Controls {
    pub fn from_chain(chain: &ProcessorChain) -> Self {
        Self {
            processors: chain
                .processors()
                .iter()
                .map(|p| processor_state_from(p.as_ref()))
                .collect(),
            selected_proc: 0,
        }
    }

    pub fn push_processor(&mut self, p: &dyn Processor) {
        self.processors.push(processor_state_from(p));
    }

    pub fn remove_processor(&mut self, index: usize) {
        if index < self.processors.len() {
            self.processors.remove(index);
            if self.selected_proc >= self.processors.len() && !self.processors.is_empty() {
                self.selected_proc = self.processors.len() - 1;
            }
        }
    }

    pub fn processor_names(&self) -> Vec<String> {
        self.processors.iter().map(|p| p.name.clone()).collect()
    }

    pub fn selected_proc(&self) -> usize {
        self.selected_proc
    }

    pub fn selected_cutoff_hz(&self) -> Option<f32> {
        self.processors
            .get(self.selected_proc)?
            .params
            .first()
            .map(|p| p.value)
    }

    pub fn cutoff_hz_for(&self, proc_idx: usize) -> Option<f32> {
        self.processors
            .get(proc_idx)?
            .params
            .first()
            .map(|p| p.value)
    }

    pub fn show_params_only(&mut self, ui: &mut egui::Ui, proc_idx: usize) -> Vec<ParamUpdate> {
        let mut updates = Vec::new();
        if let Some(proc) = self.processors.get_mut(proc_idx) {
            let color = crate::ui::theme::processor_color(&proc.name);
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 8.0);
            ui.horizontal_wrapped(|ui| {
                for (param_idx, param) in proc.params.iter_mut().enumerate() {
                    let id = format!("knob_{proc_idx}_{param_idx}");
                    let r = knob(
                        ui,
                        &id,
                        &mut param.value,
                        param.min,
                        param.max,
                        param.default,
                        param.name,
                        param.unit,
                        color,
                    );
                    if r.changed() {
                        updates.push(ParamUpdate {
                            processor_idx: proc_idx,
                            param_idx,
                            value: param.value,
                        });
                    }
                }
            });
        }
        updates
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> ControlsOutput {
        let prev_selected = self.selected_proc;
        let mut updates = Vec::new();

        for (proc_idx, proc) in self.processors.iter_mut().enumerate() {
            let is_selected = self.selected_proc == proc_idx;
            let color = crate::ui::theme::processor_color(&proc.name);
            let proc_name = proc.name.clone();

            let mut clicked = false;
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::new(4.0, 18.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(2), color);
                let header = egui::RichText::new(&proc_name).strong().color(color);
                if ui.selectable_label(is_selected, header).clicked() {
                    clicked = true;
                }
            });
            if clicked {
                self.selected_proc = proc_idx;
            }

            for (param_idx, param) in proc.params.iter_mut().enumerate() {
                let label = format!("{} ({})", param.name, param.unit);
                let r =
                    ui.add(egui::Slider::new(&mut param.value, param.min..=param.max).text(label));
                if r.changed() {
                    updates.push(ParamUpdate {
                        processor_idx: proc_idx,
                        param_idx,
                        value: param.value,
                    });
                }
            }
        }

        ControlsOutput {
            updates,
            selected_proc: self.selected_proc,
            selection_changed: self.selected_proc != prev_selected,
        }
    }
}

fn processor_state_from(p: &dyn Processor) -> ProcessorState {
    ProcessorState {
        name: p.name().to_string(),
        params: p
            .params()
            .iter()
            .map(|pp| ParamState {
                name: pp.name,
                value: pp.default,
                min: pp.min,
                max: pp.max,
                default: pp.default,
                unit: pp.unit,
            })
            .collect(),
    }
}
