use crate::dsp::processor::{Processor, ProcessorParam};

static PARAMS: &[ProcessorParam] = &[ProcessorParam {
    name: "Gain",
    min: -40.0,
    max: 40.0,
    default: 0.0,
    unit: "dB",
}];

pub struct Gain {
    gain_db: f32,
    gain_linear: f32,
}

impl Gain {
    pub fn new() -> Self {
        Self {
            gain_db: 0.0,
            gain_linear: 1.0,
        }
    }
}

impl Processor for Gain {
    fn process(&mut self, sample: f32) -> f32 {
        sample * self.gain_linear
    }

    fn reset(&mut self) {}

    fn name(&self) -> &'static str {
        "Gain"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        if index == 0 {
            let db = value.clamp(-40.0, 40.0);
            self.gain_db = db;
            self.gain_linear = 10f32.powf(db / 20.0);
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        Some((vec![self.gain_linear], vec![1.0]))
    }
}
