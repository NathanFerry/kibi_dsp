use std::f32::consts::{PI, SQRT_2};

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
        name: "Order",
        min: 1.0,
        max: 8.0,
        default: 4.0,
        unit: "",
    },
];

#[derive(Clone, Copy, Default)]
struct BiquadSection {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadSection {
    fn from_coefs(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            ..Default::default()
        }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
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
}

#[derive(Clone, Copy, Default)]
struct FirstOrder {
    b0: f32,
    b1: f32,
    a1: f32,
    x1: f32,
    y1: f32,
}

impl FirstOrder {
    fn from_coefs(b0: f32, b1: f32, a1: f32) -> Self {
        Self {
            b0,
            b1,
            a1,
            ..Default::default()
        }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 - self.a1 * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

fn prewarp(cutoff_hz: f32, sample_rate: f32) -> f32 {
    (PI * cutoff_hz / sample_rate).tan()
}

fn butter_q(n: usize, k: usize) -> f32 {
    1.0 / (2.0 * ((2 * k - 1) as f32 * PI / (2 * n) as f32).cos())
}

fn lp_biquad(omega: f32, q: f32) -> BiquadSection {
    let w2 = omega * omega;
    let d = 1.0 + omega / q + w2;
    let b0 = w2 / d;
    BiquadSection::from_coefs(
        b0,
        2.0 * b0,
        b0,
        2.0 * (w2 - 1.0) / d,
        (1.0 - omega / q + w2) / d,
    )
}

fn hp_biquad(omega: f32, q: f32) -> BiquadSection {
    let w2 = omega * omega;
    let d = 1.0 + omega / q + w2;
    let b0 = 1.0 / d;
    BiquadSection::from_coefs(
        b0,
        -2.0 * b0,
        b0,
        2.0 * (w2 - 1.0) / d,
        (1.0 - omega / q + w2) / d,
    )
}

fn lp_first_order(omega: f32) -> FirstOrder {
    let d = 1.0 + omega;
    FirstOrder::from_coefs(omega / d, omega / d, (omega - 1.0) / d)
}

fn hp_first_order(omega: f32) -> FirstOrder {
    let d = 1.0 + omega;
    FirstOrder::from_coefs(1.0 / d, -1.0 / d, (omega - 1.0) / d)
}

pub struct Butterworth {
    filter_type: u8,
    cutoff: f32,
    order: usize,
    sample_rate: f32,

    lp_first: Option<FirstOrder>,
    lp_biquads: Vec<BiquadSection>,

    hp_first: Option<FirstOrder>,
    hp_biquads: Vec<BiquadSection>,
}

impl Butterworth {
    pub fn new(sample_rate: f32) -> Self {
        let mut b = Self {
            filter_type: 0,
            cutoff: 1000.0,
            order: 4,
            sample_rate,
            lp_first: None,
            lp_biquads: Vec::with_capacity(4),
            hp_first: None,
            hp_biquads: Vec::with_capacity(4),
        };
        b.rebuild();
        b
    }

    fn rebuild(&mut self) {
        self.lp_first = None;
        self.lp_biquads.clear();
        self.hp_first = None;
        self.hp_biquads.clear();

        let cutoff = self.cutoff;
        let order = self.order;

        match self.filter_type {
            0 => self.fill_lp(order, cutoff),
            1 => self.fill_hp(order, cutoff),
            _ => {
                let lp_cut = (cutoff * SQRT_2).min(self.sample_rate * 0.45);
                let hp_cut = (cutoff / SQRT_2).max(20.0);
                self.fill_lp(order, lp_cut);
                self.fill_hp(order, hp_cut);
            }
        }
    }

    fn fill_lp(&mut self, n: usize, cutoff: f32) {
        let omega = prewarp(cutoff, self.sample_rate);
        for k in 1..=(n / 2) {
            self.lp_biquads.push(lp_biquad(omega, butter_q(n, k)));
        }
        if n % 2 == 1 {
            self.lp_first = Some(lp_first_order(omega));
        }
    }

    fn fill_hp(&mut self, n: usize, cutoff: f32) {
        let omega = prewarp(cutoff, self.sample_rate);
        for k in 1..=(n / 2) {
            self.hp_biquads.push(hp_biquad(omega, butter_q(n, k)));
        }
        if n % 2 == 1 {
            self.hp_first = Some(hp_first_order(omega));
        }
    }
}

impl Processor for Butterworth {
    fn process(&mut self, sample: f32) -> f32 {
        let mut s = sample;
        if let Some(ref mut fo) = self.lp_first {
            s = fo.tick(s);
        }
        for bq in &mut self.lp_biquads {
            s = bq.tick(s);
        }
        if let Some(ref mut fo) = self.hp_first {
            s = fo.tick(s);
        }
        for bq in &mut self.hp_biquads {
            s = bq.tick(s);
        }
        s
    }

    fn reset(&mut self) {
        if let Some(ref mut fo) = self.lp_first {
            fo.reset();
        }
        for bq in &mut self.lp_biquads {
            bq.reset();
        }
        if let Some(ref mut fo) = self.hp_first {
            fo.reset();
        }
        for bq in &mut self.hp_biquads {
            bq.reset();
        }
    }

    fn name(&self) -> &'static str {
        "Butterworth"
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
                let o = (value.round() as usize).clamp(1, 8);
                if o != self.order {
                    self.order = o;
                    self.rebuild();
                }
            }
            _ => {}
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        let mut b = vec![1.0f32];
        let mut a = vec![1.0f32];

        if let Some(ref fo) = self.lp_first {
            b = poly_mul(&b, &[fo.b0, fo.b1]);
            a = poly_mul(&a, &[1.0, fo.a1]);
        }
        for bq in &self.lp_biquads {
            b = poly_mul(&b, &[bq.b0, bq.b1, bq.b2]);
            a = poly_mul(&a, &[1.0, bq.a1, bq.a2]);
        }
        if let Some(ref fo) = self.hp_first {
            b = poly_mul(&b, &[fo.b0, fo.b1]);
            a = poly_mul(&a, &[1.0, fo.a1]);
        }
        for bq in &self.hp_biquads {
            b = poly_mul(&b, &[bq.b0, bq.b1, bq.b2]);
            a = poly_mul(&a, &[1.0, bq.a1, bq.a2]);
        }

        Some((b, a))
    }
}

fn poly_mul(a: &[f32], b: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0f32; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    out
}
