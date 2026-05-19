pub mod bessel;
pub mod biquad;
pub mod butterworth;
pub mod cauer;
pub mod chebyshev;
pub mod chorus;
pub mod delay;
pub mod fir;
pub mod gain;
pub mod moving_average;
pub mod notch;

use crate::dsp::processor::Processor;
use biquad::Biquad;
use butterworth::Butterworth;
use chorus::Chorus;
use delay::Delay;
use fir::{FirHighPass, FirLowPass};
use gain::Gain;
use moving_average::MovingAverage;
use notch::Notch;

pub const PROCESSOR_NAMES: &[&str] = &[
    "Moving Average",
    "FIR Low-Pass",
    "FIR High-Pass",
    "Butterworth",
    "Biquad",
    "Notch",
    "Gain",
    "Delay",
    "Chorus",
];

pub fn make_processor(name: &str, sample_rate: f32) -> Option<Box<dyn Processor>> {
    match name {
        "Moving Average" => Some(Box::new(MovingAverage::new())),
        "FIR Low-Pass" => Some(Box::new(FirLowPass::new(sample_rate))),
        "FIR High-Pass" => Some(Box::new(FirHighPass::new(sample_rate))),
        "Butterworth" => Some(Box::new(Butterworth::new(sample_rate))),
        "Biquad" => Some(Box::new(Biquad::new(sample_rate))),
        "Notch" => Some(Box::new(Notch::new(sample_rate))),
        "Gain" => Some(Box::new(Gain::new())),
        "Delay" => Some(Box::new(Delay::new(sample_rate))),
        "Chorus" => Some(Box::new(Chorus::new(sample_rate))),
        _ => None,
    }
}
