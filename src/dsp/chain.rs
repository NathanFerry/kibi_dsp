use crate::dsp::processor::Processor;

pub struct ParamUpdate {
    pub processor_idx: usize,
    pub param_idx: usize,
    pub value: f32,
}

pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        self.processors.iter_mut().fold(sample, |s, p| p.process(s))
    }

    pub fn reset(&mut self) {
        for p in &mut self.processors {
            p.reset();
        }
    }

    pub fn add(&mut self, processor: Box<dyn Processor>) {
        self.processors.push(processor);
    }

    pub fn remove(&mut self, index: usize) {
        self.processors.remove(index);
    }

    pub fn apply_update(&mut self, update: ParamUpdate) {
        if let Some(p) = self.processors.get_mut(update.processor_idx) {
            p.set_param(update.param_idx, update.value);
        }
    }

    pub fn processors(&self) -> &[Box<dyn Processor>] {
        &self.processors
    }
}
