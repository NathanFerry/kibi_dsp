use crate::dsp::chain::{ParamUpdate, ProcessorChain};
use crate::dsp::processor::Processor;

pub struct ParamState {
    pub name: &'static str,
    pub value: f32,
    pub min: f32,
    pub max: f32,
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

    pub fn show(&mut self, ui: &mut egui::Ui) -> ControlsOutput {
        let prev_selected = self.selected_proc;
        let mut updates = Vec::new();

        for (proc_idx, proc) in self.processors.iter_mut().enumerate() {
            let is_selected = self.selected_proc == proc_idx;

            let header = egui::RichText::new(proc.name.as_str()).strong();
            if ui.selectable_label(is_selected, header).clicked() {
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
                unit: pp.unit,
            })
            .collect(),
    }
}
