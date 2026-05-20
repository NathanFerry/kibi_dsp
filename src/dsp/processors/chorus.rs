use std::f32::consts::TAU;

use crate::dsp::processor::{Processor, ProcessorParam};

const NUM_VOICES: usize = 3;
const MAX_DELAY_SAMPLES: usize = 192_000;
const BASE_DELAY_MS: f32 = 20.0;

static PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Rate",
        min: 0.1,
        max: 5.0,
        default: 0.5,
        unit: "Hz",
    },
    ProcessorParam {
        name: "Depth",
        min: 0.1,
        max: 10.0,
        default: 2.0,
        unit: "ms",
    },
    ProcessorParam {
        name: "Mix",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        unit: "",
    },
];

pub struct Chorus {
    buffer: Vec<f32>,
    write_head: usize,
    lfo_phases: [f32; NUM_VOICES],
    lfo_increment: f32,
    depth_samples: f32,
    base_delay_samples: f32,
    mix: f32,
    sample_rate: f32,
}

impl Chorus {
    pub fn new(sample_rate: f32) -> Self {
        let lfo_increment = TAU * 0.5 / sample_rate;
        let depth_samples = 2.0 * sample_rate / 1000.0;
        let base_delay_samples = BASE_DELAY_MS * sample_rate / 1000.0;
        Self {
            buffer: vec![0.0; MAX_DELAY_SAMPLES],
            write_head: 0,
            lfo_phases: [0.0, TAU / 3.0, 2.0 * TAU / 3.0],
            lfo_increment,
            depth_samples,
            base_delay_samples,
            mix: 0.5,
            sample_rate,
        }
    }
}

impl Processor for Chorus {
    fn process(&mut self, sample: f32) -> f32 {
        self.buffer[self.write_head] = sample;

        let mut wet = 0.0f32;
        for i in 0..NUM_VOICES {
            let lfo = self.lfo_phases[i].sin();
            let delay = (self.base_delay_samples + lfo * self.depth_samples).max(1.0);
            let delay_int = delay as usize;
            let frac = delay - delay_int as f32;

            let pos1 = (self.write_head + MAX_DELAY_SAMPLES - delay_int) % MAX_DELAY_SAMPLES;
            let pos2 = (self.write_head + MAX_DELAY_SAMPLES - delay_int - 1) % MAX_DELAY_SAMPLES;
            wet += self.buffer[pos1] * (1.0 - frac) + self.buffer[pos2] * frac;

            self.lfo_phases[i] = (self.lfo_phases[i] + self.lfo_increment) % TAU;
        }
        wet /= NUM_VOICES as f32;

        self.write_head = (self.write_head + 1) % MAX_DELAY_SAMPLES;

        sample * (1.0 - self.mix) + wet * self.mix
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_head = 0;
        self.lfo_phases = [0.0, TAU / 3.0, 2.0 * TAU / 3.0];
    }

    fn name(&self) -> &'static str {
        "Chorus"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                self.lfo_increment = TAU * value / self.sample_rate;
            }
            1 => {
                self.depth_samples = value * self.sample_rate / 1000.0;
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
