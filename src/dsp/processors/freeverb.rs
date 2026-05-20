use crate::dsp::processor::{Processor, ProcessorParam};

const NUM_COMBS: usize = 8;
const NUM_ALLPASSES: usize = 4;
const FIXED_GAIN: f32 = 0.015;
const SCALE_WET: f32 = 3.0;
const SCALE_DAMPING: f32 = 0.4;
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;

const COMB_SIZES: [usize; NUM_COMBS] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_SIZES: [usize; NUM_ALLPASSES] = [556, 441, 341, 225];

static PARAMS: &[ProcessorParam] = &[
    ProcessorParam {
        name: "Room Size",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        unit: "",
    },
    ProcessorParam {
        name: "Damping",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        unit: "",
    },
    ProcessorParam {
        name: "Mix",
        min: 0.0,
        max: 1.0,
        default: 0.33,
        unit: "",
    },
];

struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    filter_store: f32,
    damp1: f32,
    damp2: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            index: 0,
            feedback: 0.84,
            filter_store: 0.0,
            damp1: 0.2,
            damp2: 0.8,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        self.filter_store = output * self.damp2 + self.filter_store * self.damp1;
        self.buffer[self.index] = input + self.filter_store * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }

    fn set_feedback(&mut self, val: f32) {
        self.feedback = val;
    }

    fn set_damp(&mut self, val: f32) {
        self.damp1 = val;
        self.damp2 = 1.0 - val;
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.filter_store = 0.0;
        self.index = 0;
    }
}

struct AllpassFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            index: 0,
            feedback: 0.5,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.index];
        let output = -input + buf_out;
        self.buffer[self.index] = input + buf_out * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
    }
}

pub struct Freeverb {
    combs: [CombFilter; NUM_COMBS],
    allpasses: [AllpassFilter; NUM_ALLPASSES],
    room_size: f32,
    damping: f32,
    mix: f32,
    wet: f32,
    dry: f32,
}

impl Freeverb {
    pub fn new(sample_rate: f32) -> Self {
        let combs = std::array::from_fn(|i| {
            let size = (COMB_SIZES[i] as f32 * sample_rate / 44100.0) as usize;
            CombFilter::new(size)
        });
        let allpasses = std::array::from_fn(|i| {
            let size = (ALLPASS_SIZES[i] as f32 * sample_rate / 44100.0) as usize;
            AllpassFilter::new(size)
        });
        let mut freeverb = Self {
            combs,
            allpasses,
            room_size: 0.5,
            damping: 0.5,
            mix: 0.33,
            wet: 0.0,
            dry: 0.0,
        };
        freeverb.update();
        freeverb
    }

    fn update(&mut self) {
        let room = self.room_size * SCALE_ROOM + OFFSET_ROOM;
        let damp = self.damping * SCALE_DAMPING;
        self.wet = self.mix * SCALE_WET;
        self.dry = 1.0 - self.wet;
        for comb in &mut self.combs {
            comb.set_feedback(room);
            comb.set_damp(damp);
        }
    }
}

impl Processor for Freeverb {
    fn process(&mut self, sample: f32) -> f32 {
        let input = sample * FIXED_GAIN;
        let mut out = 0.0f32;
        for comb in &mut self.combs {
            out += comb.process(input);
        }
        for allpass in &mut self.allpasses {
            out = allpass.process(out);
        }
        sample * self.dry + out * self.wet
    }

    fn reset(&mut self) {
        for comb in &mut self.combs {
            comb.reset();
        }
        for allpass in &mut self.allpasses {
            allpass.reset();
        }
    }

    fn name(&self) -> &'static str {
        "Freeverb"
    }

    fn params(&self) -> &[ProcessorParam] {
        PARAMS
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                self.room_size = value.clamp(0.0, 1.0);
                self.update();
            }
            1 => {
                self.damping = value.clamp(0.0, 1.0);
                self.update();
            }
            2 => {
                self.mix = value.clamp(0.0, 1.0);
                self.update();
            }
            _ => {}
        }
    }

    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        None
    }
}
