use crate::dsp::processor::{Processor, ProcessorParam};

const MAX_N: usize = 4096;

static PARAMS: &[ProcessorParam] = &[ProcessorParam {
    name: "N",
    min: 2.0,
    max: 500.0,
    default: 10.0,
    unit: "samples",
}];

pub struct MovingAverage {
    buf: Box<[f32; MAX_N]>,
    head: usize,
    sum: f32,
    n: usize,
}

impl MovingAverage {
    pub fn new() -> Self {
        Self {
            buf: Box::new([0.0f32; MAX_N]),
            head: 0,
            sum: 0.0,
            n: 10,
        }
    }
}

impl Processor for MovingAverage {
    fn process(&mut self, sample: f32) -> f32 {
        self.sum -= self.buf[self.head];
        self.buf[self.head] = sample;
        self.sum += sample;
        self.head = (self.head + 1) % self.n;
        self.sum / self.n as f32
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.head = 0;
        self.sum = 0.0;
    }

    fn name(&self) -> &'static str {
        "Moving Average"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        if index == 0 {
            let new_n = (value as usize).clamp(2, MAX_N);
            if new_n != self.n {
                self.n = new_n;
                self.reset();
            }
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        let coeff = 1.0f32 / self.n as f32;
        Some((vec![coeff; self.n], vec![1.0f32]))
    }
}
