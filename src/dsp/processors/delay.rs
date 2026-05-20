use std::f32::consts::TAU;

use crate::dsp::processor::{Processor, ProcessorParam};

const LP_CUTOFF_HZ: f32 = 4000.0;

static PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Delay Time",
        min: 1.0,
        max: 2000.0,
        default: 300.0,
        unit: "ms",
    },
    ProcessorParam {
        name: "Feedback",
        min: 0.0,
        max: 0.95,
        default: 0.4,
        unit: "",
    },
    ProcessorParam {
        name: "Mix",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        unit: "",
    },
];

pub struct Delay {
    sample_rate: f32,
    // Circular delay buffer — pre-allocated, never resized.
    buffer: Vec<f32>,
    max_delay_samples: usize,
    write_head: usize,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
    // One-pole IIR low-pass on the feedback path.
    feedback_lp_coeff: f32,
    feedback_lp_x1: f32,
}

impl Delay {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay_samples = (sample_rate * 2.0) as usize;
        let delay_samples = (0.3 * sample_rate) as usize; // default 300 ms
        let feedback_lp_coeff = 1.0 - (-TAU * LP_CUTOFF_HZ / sample_rate).exp();
        Self {
            sample_rate,
            buffer: vec![0.0; max_delay_samples],
            max_delay_samples,
            write_head: 0,
            delay_samples,
            feedback: 0.4,
            mix: 0.5,
            feedback_lp_coeff,
            feedback_lp_x1: 0.0,
        }
    }

    #[inline]
    fn apply_feedback_lp(&mut self, x: f32) -> f32 {
        let y = self.feedback_lp_coeff * x + (1.0 - self.feedback_lp_coeff) * self.feedback_lp_x1;
        self.feedback_lp_x1 = y;
        y
    }
}

impl Processor for Delay {
    fn process(&mut self, sample: f32) -> f32 {
        let read_head = (self.write_head + self.max_delay_samples - self.delay_samples)
            % self.max_delay_samples;
        let delayed = self.buffer[read_head];

        let lp_out = self.apply_feedback_lp(delayed);

        self.buffer[self.write_head] = sample + lp_out * self.feedback;
        self.write_head = (self.write_head + 1) % self.max_delay_samples;

        sample * (1.0 - self.mix) + delayed * self.mix
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.feedback_lp_x1 = 0.0;
        self.write_head = 0;
    }

    fn name(&self) -> &'static str {
        "Delay"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                let ms = value.clamp(1.0, 2000.0);
                self.delay_samples =
                    ((ms / 1000.0 * self.sample_rate) as usize).min(self.max_delay_samples - 1);
            }
            1 => {
                self.feedback = value.clamp(0.0, 0.95);
            }
            2 => {
                self.mix = value.clamp(0.0, 1.0);
            }
            _ => {}
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        None
    }
}
