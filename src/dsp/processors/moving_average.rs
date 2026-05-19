use crate::dsp::processor::{Processor, ProcessorParam};

pub struct MovingAverage;

impl Processor for MovingAverage {
    fn process(&mut self, _sample: f32) -> f32 {
        todo!()
    }
    fn reset(&mut self) {
        todo!()
    }
    fn name(&self) -> &'static str {
        "Moving Average"
    }
    fn params(&self) -> &[ProcessorParam] {
        todo!()
    }
    fn set_param(&mut self, _index: usize, _value: f32) {
        todo!()
    }
    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        todo!()
    }
}
