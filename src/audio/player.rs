use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream};
use crossbeam_channel::{Receiver, Sender};
use crossbeam_queue::ArrayQueue;

use crate::dsp::chain::{ParamUpdate, ProcessorChain};

pub const FFT_QUEUE_SIZE: usize = 4096;

pub struct Player {
    _stream: Stream,
    pub param_tx: Sender<ParamUpdate>,
    pub fft_queue: Arc<ArrayQueue<f32>>,
    pub cursor: Arc<AtomicUsize>,
    pub playing: Arc<AtomicBool>,
    pub sample_count: usize,
    pub sample_rate: u32,
}

impl Player {
    pub fn new(samples: Arc<Vec<f32>>, sample_rate: u32) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default audio output device")?;

        let output_config = select_output_config(&device, sample_rate)?;
        let sample_format = output_config.sample_format();
        let stream_config: cpal::StreamConfig = output_config.into();

        let (param_tx, param_rx) = crossbeam_channel::unbounded::<ParamUpdate>();
        let fft_queue = Arc::new(ArrayQueue::<f32>::new(FFT_QUEUE_SIZE));
        let cursor = Arc::new(AtomicUsize::new(0));
        let playing = Arc::new(AtomicBool::new(true));

        let chain = ProcessorChain::new();
        let sample_count = samples.len();

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &stream_config,
                param_rx,
                Arc::clone(&fft_queue),
                chain,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&playing),
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &stream_config,
                param_rx,
                Arc::clone(&fft_queue),
                chain,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&playing),
            )?,
            SampleFormat::I32 => build_stream::<i32>(
                &device,
                &stream_config,
                param_rx,
                Arc::clone(&fft_queue),
                chain,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&playing),
            )?,
            SampleFormat::F64 => build_stream::<f64>(
                &device,
                &stream_config,
                param_rx,
                Arc::clone(&fft_queue),
                chain,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&playing),
            )?,
            fmt => bail!("unsupported sample format: {fmt:?}"),
        };

        stream.play()?;

        Ok(Self {
            _stream: stream,
            param_tx,
            fft_queue,
            cursor,
            playing,
            sample_count,
            sample_rate,
        })
    }
}

fn select_output_config(
    device: &cpal::Device,
    preferred_rate: u32,
) -> Result<cpal::SupportedStreamConfig> {
    if let Ok(configs) = device.supported_output_configs() {
        for config in configs {
            if config.min_sample_rate() <= preferred_rate
                && preferred_rate <= config.max_sample_rate()
            {
                return Ok(config.with_sample_rate(preferred_rate));
            }
        }
    }
    log::warn!(
        "device does not support {}Hz; falling back to device default (audio may be pitch-shifted)",
        preferred_rate
    );
    device
        .default_output_config()
        .context("no default output config")
}

#[allow(clippy::too_many_arguments)]
fn build_stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    param_rx: Receiver<ParamUpdate>,
    fft_queue: Arc<ArrayQueue<f32>>,
    mut chain: ProcessorChain,
    samples: Arc<Vec<f32>>,
    cursor: Arc<AtomicUsize>,
    playing: Arc<AtomicBool>,
) -> Result<Stream> {
    let channels = config.channels as usize;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            while let Ok(update) = param_rx.try_recv() {
                chain.apply_update(update);
            }
            for frame in data.chunks_mut(channels) {
                let raw = if playing.load(Ordering::Relaxed) {
                    let pos = cursor.fetch_add(1, Ordering::Relaxed);
                    if pos < samples.len() {
                        samples[pos]
                    } else {
                        // Reached end: stop and rewind
                        playing.store(false, Ordering::Relaxed);
                        cursor.store(0, Ordering::Relaxed);
                        0.0f32
                    }
                } else {
                    0.0f32
                };

                let processed = chain.process(raw);
                let _ = fft_queue.push(processed);
                let out = T::from_sample(processed);
                for ch in frame.iter_mut() {
                    *ch = out;
                }
            }
        },
        |err| log::error!("audio stream error: {err}"),
        None,
    )?;

    Ok(stream)
}
