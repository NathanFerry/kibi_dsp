pub struct ProcessorParam {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: &'static str,
}

pub trait Processor: Send {
    fn process(&mut self, sample: f32) -> f32;
    fn reset(&mut self);
    fn name(&self) -> &'static str;
    fn params(&self) -> &[ProcessorParam];
    fn set_param(&mut self, index: usize, value: f32);
    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)>;
}
