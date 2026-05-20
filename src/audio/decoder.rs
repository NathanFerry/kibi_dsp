use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn decode_audio_file(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = File::open(path).context("failed to open audio file")?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("unsupported or unrecognised audio format")?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .context("audio file contains no tracks")?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("track has no sample rate")?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("unsupported codec")?;

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => return Err(anyhow!("format read error: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(e)) => {
                log::warn!("decode error, skipping packet: {e}");
                continue;
            }
            Err(e) => return Err(anyhow!("decode failed: {e}")),
        };

        let spec = *decoded.spec();
        let n_frames = decoded.frames();
        let ch_count = spec.channels.count();

        let mut sample_buf = SampleBuffer::<f32>::new(n_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        for frame in sample_buf.samples().chunks(ch_count) {
            let mono = frame.iter().sum::<f32>() / ch_count as f32;
            samples.push(mono);
        }
    }

    log::info!(
        "decoded {} frames at {} Hz from {}",
        samples.len(),
        sample_rate,
        path.display()
    );

    Ok((samples, sample_rate))
}
