use std::f32::consts::PI;

use crate::dsp::processor::{Processor, ProcessorParam};

const MAX_ORDER: usize = 256;
const MAX_TAPS: usize = MAX_ORDER + 1;

static LP_PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Cutoff",
        min: 20.0,
        max: 20000.0,
        default: 1000.0,
        unit: "Hz",
    },
    ProcessorParam {
        name: "Order",
        min: 8.0,
        max: 256.0,
        default: 64.0,
        unit: "",
    },
];

static HP_PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Cutoff",
        min: 20.0,
        max: 20000.0,
        default: 1000.0,
        unit: "Hz",
    },
    ProcessorParam {
        name: "Order",
        min: 8.0,
        max: 256.0,
        default: 64.0,
        unit: "",
    },
];

fn fill_lp_coeffs(coeffs: &mut [f32], order: usize, cutoff_hz: f32, sample_rate: f32) {
    let order = order & !1;
    let taps = order + 1;
    let fc = (cutoff_hz / sample_rate).clamp(0.0001, 0.4999);
    let m = order as f32;

    for (n, c) in coeffs[..taps].iter_mut().enumerate() {
        let hann = 0.5 * (1.0 - (2.0 * PI * n as f32 / m).cos());
        let center = n as f32 - m / 2.0;
        let sinc = if center.abs() < 1e-10 {
            2.0 * fc
        } else {
            (2.0 * PI * fc * center).sin() / (PI * center)
        };
        *c = hann * sinc;
    }
    for c in coeffs[taps..].iter_mut() {
        *c = 0.0;
    }

    let sum: f32 = coeffs[..taps].iter().sum();
    if sum.abs() > 1e-10 {
        for c in coeffs[..taps].iter_mut() {
            *c /= sum;
        }
    }
}

fn fill_hp_coeffs(coeffs: &mut [f32], order: usize, cutoff_hz: f32, sample_rate: f32) {
    fill_lp_coeffs(coeffs, order, cutoff_hz, sample_rate);
    let taps = (order & !1) + 1;
    for (n, c) in coeffs[..taps].iter_mut().enumerate() {
        if n % 2 != 0 {
            *c = -*c;
        }
    }
    let abs_sum: f32 = coeffs[..taps].iter().map(|c| c.abs()).sum();
    if abs_sum > 1e-10 {
        for c in coeffs[..taps].iter_mut() {
            *c /= abs_sum;
        }
    }
}

#[inline]
fn fir_process(
    sample: f32,
    coeffs: &[f32],
    delay: &mut [f32],
    head: &mut usize,
    taps: usize,
) -> f32 {
    delay[*head] = sample;
    let mut out = 0.0f32;
    for (n, &coeff) in coeffs[..taps].iter().enumerate() {
        let i = (*head + MAX_TAPS - n) % MAX_TAPS;
        out += coeff * delay[i];
    }
    *head = (*head + 1) % MAX_TAPS;
    out
}

pub struct FirLowPass {
    coeffs: Vec<f32>,
    delay: Vec<f32>,
    head: usize,
    order: usize,
    cutoff: f32,
    sample_rate: f32,
}

impl FirLowPass {
    pub fn new(sample_rate: f32) -> Self {
        let order = 64usize;
        let cutoff = 1000.0f32;
        let mut coeffs = vec![0.0f32; MAX_TAPS];
        fill_lp_coeffs(&mut coeffs, order, cutoff, sample_rate);
        Self {
            coeffs,
            delay: vec![0.0f32; MAX_TAPS],
            head: 0,
            order,
            cutoff,
            sample_rate,
        }
    }
}

impl Processor for FirLowPass {
    fn process(&mut self, sample: f32) -> f32 {
        let taps = (self.order & !1) + 1;
        fir_process(sample, &self.coeffs, &mut self.delay, &mut self.head, taps)
    }

    fn reset(&mut self) {
        self.delay.fill(0.0);
        self.head = 0;
    }

    fn name(&self) -> &'static str {
        "FIR Low-Pass"
    }

    fn params(&self) -> &[ProcessorParam] {
        LP_PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                self.cutoff = value.clamp(20.0, 20000.0);
                fill_lp_coeffs(&mut self.coeffs, self.order, self.cutoff, self.sample_rate);
            }
            1 => {
                let new_order = (value as usize).clamp(8, MAX_ORDER) & !1;
                if new_order != self.order {
                    self.order = new_order;
                    fill_lp_coeffs(&mut self.coeffs, self.order, self.cutoff, self.sample_rate);
                    self.reset();
                }
            }
            _ => {}
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        let taps = (self.order & !1) + 1;
        Some((self.coeffs[..taps].to_vec(), vec![1.0f32]))
    }
}

pub struct FirHighPass {
    coeffs: Vec<f32>,
    delay: Vec<f32>,
    head: usize,
    order: usize,
    cutoff: f32,
    sample_rate: f32,
}

impl FirHighPass {
    pub fn new(sample_rate: f32) -> Self {
        let order = 64usize;
        let cutoff = 1000.0f32;
        let mut coeffs = vec![0.0f32; MAX_TAPS];
        fill_hp_coeffs(&mut coeffs, order, cutoff, sample_rate);
        Self {
            coeffs,
            delay: vec![0.0f32; MAX_TAPS],
            head: 0,
            order,
            cutoff,
            sample_rate,
        }
    }
}

impl Processor for FirHighPass {
    fn process(&mut self, sample: f32) -> f32 {
        let taps = (self.order & !1) + 1;
        fir_process(sample, &self.coeffs, &mut self.delay, &mut self.head, taps)
    }

    fn reset(&mut self) {
        self.delay.fill(0.0);
        self.head = 0;
    }

    fn name(&self) -> &'static str {
        "FIR High-Pass"
    }

    fn params(&self) -> &[ProcessorParam] {
        HP_PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                self.cutoff = value.clamp(20.0, 20000.0);
                fill_hp_coeffs(&mut self.coeffs, self.order, self.cutoff, self.sample_rate);
            }
            1 => {
                let new_order = (value as usize).clamp(8, MAX_ORDER) & !1;
                if new_order != self.order {
                    self.order = new_order;
                    fill_hp_coeffs(&mut self.coeffs, self.order, self.cutoff, self.sample_rate);
                    self.reset();
                }
            }
            _ => {}
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        let taps = (self.order & !1) + 1;
        Some((self.coeffs[..taps].to_vec(), vec![1.0f32]))
    }
}
