use std::f32::consts::TAU;

use crate::dsp::processor::{Processor, ProcessorParam};

static PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Center",
        min: 20.0,
        max: 20000.0,
        default: 1000.0,
        unit: "Hz",
    },
    ProcessorParam {
        name: "Q",
        min: 0.1,
        max: 20.0,
        default: 1.0,
        unit: "Q",
    },
];

fn compute_coeffs(cutoff: f32, q: f32, sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    let w0 = TAU * cutoff / sample_rate;
    let cos_w0 = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    let b0 = 1.0 / a0;
    let b1 = -2.0 * cos_w0 / a0;
    let b2 = 1.0 / a0;
    let a1 = -2.0 * cos_w0 / a0;
    let a2 = (1.0 - alpha) / a0;
    (vec![b0, b1, b2], vec![1.0, a1, a2])
}

pub struct Notch {
    cutoff: f32,
    q: f32,
    sample_rate: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    b_coeffs: Vec<f32>,
    a_coeffs: Vec<f32>,
}

impl Notch {
    pub fn new(sample_rate: f32) -> Self {
        let cutoff = 1000.0f32;
        let q = 1.0f32;
        let (b_coeffs, a_coeffs) = compute_coeffs(cutoff, q, sample_rate);
        Self {
            cutoff,
            q,
            sample_rate,
            b0: b_coeffs[0],
            b1: b_coeffs[1],
            b2: b_coeffs[2],
            a1: a_coeffs[1],
            a2: a_coeffs[2],
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            b_coeffs,
            a_coeffs,
        }
    }

    fn rebuild(&mut self) {
        let (b, a) = compute_coeffs(self.cutoff, self.q, self.sample_rate);
        self.b0 = b[0];
        self.b1 = b[1];
        self.b2 = b[2];
        self.a1 = a[1];
        self.a2 = a[2];
        self.b_coeffs = b;
        self.a_coeffs = a;
    }
}

impl Processor for Notch {
    fn process(&mut self, sample: f32) -> f32 {
        let y = self.b0 * sample + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = sample;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    fn name(&self) -> &'static str {
        "Notch"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                let c = value.clamp(20.0, 20000.0);
                if (c - self.cutoff).abs() > 0.01 {
                    self.cutoff = c;
                    self.rebuild();
                }
            }
            1 => {
                let q = value.clamp(0.1, 20.0);
                if (q - self.q).abs() > 1e-6 {
                    self.q = q;
                    self.rebuild();
                }
            }
            _ => {}
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        Some((self.b_coeffs.clone(), self.a_coeffs.clone()))
    }
}
