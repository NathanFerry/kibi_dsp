use crate::dsp::processor::Processor;
use crate::dsp::processors::moving_average::MovingAverage;

#[derive(Clone)]
pub struct ParamUpdate {
    pub processor_idx: usize,
    pub param_idx: usize,
    pub value: f32,
}

pub enum ChainCommand {
    Add(Box<dyn Processor>),
    Remove(usize),
}

pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    pub fn new() -> Self {
        let mut chain = Self {
            processors: Vec::new(),
        };
        chain.add(Box::new(MovingAverage::new()));
        chain
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
        if index < self.processors.len() {
            self.processors.remove(index);
        }
    }

    pub fn apply_update(&mut self, update: ParamUpdate) {
        if let Some(p) = self.processors.get_mut(update.processor_idx) {
            p.set_param(update.param_idx, update.value);
        }
    }

    pub fn apply_command(&mut self, cmd: ChainCommand) {
        match cmd {
            ChainCommand::Add(p) => self.add(p),
            ChainCommand::Remove(idx) => self.remove(idx),
        }
    }

    pub fn processors(&self) -> &[Box<dyn Processor>] {
        &self.processors
    }
}
