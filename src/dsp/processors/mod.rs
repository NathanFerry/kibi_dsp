pub mod bessel;
pub mod biquad;
pub mod butterworth;
pub mod cauer;
pub mod chebyshev;
pub mod fir;
pub mod moving_average;

use crate::dsp::processor::Processor;
use fir::{FirHighPass, FirLowPass};
use moving_average::MovingAverage;

pub const PROCESSOR_NAMES: &[&str] = &["Moving Average", "FIR Low-Pass", "FIR High-Pass"];

pub fn make_processor(name: &str, sample_rate: f32) -> Option<Box<dyn Processor>> {
    match name {
        "Moving Average" => Some(Box::new(MovingAverage::new())),
        "FIR Low-Pass" => Some(Box::new(FirLowPass::new(sample_rate))),
        "FIR High-Pass" => Some(Box::new(FirHighPass::new(sample_rate))),
        _ => None,
    }
}
