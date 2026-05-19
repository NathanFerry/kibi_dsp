use std::f32::consts::TAU;

use crate::dsp::processor::{Processor, ProcessorParam};

static PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Type",
        min: 0.0,
        max: 2.0,
        default: 0.0,
        unit: "0=LP 1=HP 2=BP",
    },
    ProcessorParam {
        name: "Cutoff",
        min: 20.0,
        max: 20000.0,
        default: 1000.0,
        unit: "Hz",
    },
    ProcessorParam {
        name: "Q",
        min: 0.1,
        max: 20.0,
        default: 0.707,
        unit: "Q",
    },
];

/// Audio EQ Cookbook (Bristow-Johnson) biquad coefficients, normalized by a0.
/// Returns ([b0, b1, b2], [1.0, a1, a2]).
fn compute_biquad_coeffs(
    filter_type: u8,
    cutoff: f32,
    q: f32,
    sample_rate: f32,
) -> (Vec<f32>, Vec<f32>) {
    let w0 = TAU * cutoff / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let (b0, b1, b2) = match filter_type {
        0 => {
            // Low-pass
            let b0 = (1.0 - cos_w0) / 2.0;
            (b0, 1.0 - cos_w0, b0)
        }
        1 => {
            // High-pass
            let b0 = (1.0 + cos_w0) / 2.0;
            (b0, -(1.0 + cos_w0), b0)
        }
        _ => {
            // Band-pass
            let b0 = sin_w0 / 2.0;
            (b0, 0.0, -b0)
        }
    };

    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;

    (
        vec![b0 / a0, b1 / a0, b2 / a0],
        vec![1.0, a1 / a0, a2 / a0],
    )
}

pub struct Biquad {
    filter_type: u8,
    cutoff: f32,
    q: f32,
    sample_rate: f32,
    // feedforward coefficients (normalized, a0=1)
    b0: f32,
    b1: f32,
    b2: f32,
    // feedback coefficients (normalized)
    a1: f32,
    a2: f32,
    // delay registers
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    // cached for transfer_function()
    b_coeffs: Vec<f32>,
    a_coeffs: Vec<f32>,
}

impl Biquad {
    pub fn new(sample_rate: f32) -> Self {
        let filter_type = 0u8;
        let cutoff = 1000.0f32;
        let q = 0.707f32;
        let (b_coeffs, a_coeffs) = compute_biquad_coeffs(filter_type, cutoff, q, sample_rate);
        Self {
            filter_type,
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
        let (b, a) = compute_biquad_coeffs(self.filter_type, self.cutoff, self.q, self.sample_rate);
        self.b0 = b[0];
        self.b1 = b[1];
        self.b2 = b[2];
        self.a1 = a[1];
        self.a2 = a[2];
        self.b_coeffs = b;
        self.a_coeffs = a;
    }
}

impl Processor for Biquad {
    fn process(&mut self, sample: f32) -> f32 {
        let y = self.b0 * sample
            + self.b1 * self.x1
            + self.b2 * self.x2
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
        "Biquad"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                let t = (value.round() as u8).clamp(0, 2);
                if t != self.filter_type {
                    self.filter_type = t;
                    self.rebuild();
                }
            }
            1 => {
                let c = value.clamp(20.0, 20000.0);
                if (c - self.cutoff).abs() > 0.01 {
                    self.cutoff = c;
                    self.rebuild();
                }
            }
            2 => {
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
