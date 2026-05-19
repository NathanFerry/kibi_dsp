use crossbeam_channel::Sender;

use crate::dsp::chain::{ParamUpdate, ProcessorChain};

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

#[derive(Default)]
pub struct Controls {
    processors: Vec<ProcessorState>,
}

impl Controls {
    pub fn from_chain(chain: &ProcessorChain) -> Self {
        Self {
            processors: chain
                .processors()
                .iter()
                .map(|p| ProcessorState {
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
                })
                .collect(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, param_tx: Option<&Sender<ParamUpdate>>) {
        for (proc_idx, proc) in self.processors.iter_mut().enumerate() {
            ui.label(egui::RichText::new(proc.name.as_str()).strong());
            for (param_idx, param) in proc.params.iter_mut().enumerate() {
                let label = format!("{} ({})", param.name, param.unit);
                let r =
                    ui.add(egui::Slider::new(&mut param.value, param.min..=param.max).text(label));
                if r.changed()
                    && let Some(tx) = param_tx
                {
                    let _ = tx.send(ParamUpdate {
                        processor_idx: proc_idx,
                        param_idx,
                        value: param.value,
                    });
                }
            }
        }
    }
}
