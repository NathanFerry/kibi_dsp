# DSP Audio Processing App — Claude Code Guide

## Project Overview

A real-time audio processing desktop app built in Rust for a signal processing class.
The user loads a WAV/MP3 file, builds a processing chain of DSP processors, tweaks
parameters live, and hears/sees the result in real time. The UI shows the original and
processed waveform, FFT spectrum, and a Bode diagram for the currently selected processor.

The app is designed to grow: it starts as a signal filtering tool but the architecture must
support adding new DSP processors (EQ, compressor, reverb, pitch shifter…) without
refactoring the core pipeline.

---

## Tech Stack

| Concern         | Crate                          | Version  |
|-----------------|--------------------------------|----------|
| UI framework    | `egui` + `eframe`              | 0.34.2   |
| Audio I/O       | `cpal`                         | 0.17.3   |
| DSP / filters   | `fundsp`                       | 0.20.0   |
| Audio decoding  | `symphonia`                    | latest   |
| FFT             | `rustfft`                      | latest   |
| Thread comms    | `crossbeam-channel` + `crossbeam-queue` | latest |
| Error handling  | `anyhow` + `thiserror`         | latest   |
| Logging         | `log` + `env_logger`           | latest   |

> Do not introduce new audio or DSP crates without asking. Stick to the versions above for
> egui/eframe/cpal/fundsp — they have breaking APIs between minor versions.

---

## Project Structure

```
src/
├── main.rs               # eframe entry point only — no logic here
├── app.rs                # Top-level App struct implementing eframe::App
├── audio/
│   ├── mod.rs
│   ├── player.rs         # cpal stream setup, real-time callback
│   └── decoder.rs        # WAV/MP3 loading via symphonia
├── dsp/
│   ├── mod.rs
│   ├── processor.rs      # Processor trait + ProcessorParam — core abstraction
│   ├── chain.rs          # ProcessorChain: ordered list of active Processor nodes
│   ├── bode.rs           # Bode diagram computation (analytical, from transfer function)
│   └── processors/
│       ├── mod.rs
│       ├── moving_average.rs # Moving average (manual — trivial, no fundsp needed)
│       ├── fir.rs        # FIR windowed-sinc (fundsp)
│       ├── butterworth.rs# IIR Butterworth (fundsp)
│       ├── chebyshev.rs  # IIR Chebyshev type I & II (fundsp)
│       ├── bessel.rs     # Bessel / Thomson filter (fundsp)
│       ├── cauer.rs      # Elliptic / Cauer filter (fundsp)
│       └── biquad.rs     # Biquad LP/HP/BP (fundsp)
└── ui/
    ├── mod.rs
    ├── toolbar.rs        # File open, play/pause controls, seek slider
    ├── chain_editor.rs   # Add/remove/reorder processors in the chain
    ├── controls.rs       # Per-processor parameter panel (sliders, dropdowns)
    ├── waveform.rs       # Time-domain waveform plot (egui_plot)
    ├── spectrum.rs       # FFT spectrum plot (egui_plot)
    ├── spectrogram.rs    # Spectrogram (time-frequency heatmap, custom egui painter)
    └── bode.rs           # Bode diagram plot (egui_plot) — magnitude + phase
```

---

## Core Abstraction: The `Processor` Trait

Every DSP node implements this trait. It is the central abstraction — never bypass it.

```rust
// dsp/processor.rs
pub trait Processor: Send {
    /// Process a single sample. Called from the audio thread — no allocation allowed.
    fn process(&mut self, sample: f32) -> f32;

    /// Reset internal delay-line state without reallocating.
    fn reset(&mut self);

    /// Human-readable name shown in the UI chain editor.
    fn name(&self) -> &'static str;

    /// Tweakable parameters exposed to the UI.
    fn params(&self) -> &[ProcessorParam];

    /// Update a parameter by index. Must not allocate — coefficient recomputation
    /// should be done here (called from the UI thread, not the audio thread).
    fn set_param(&mut self, index: usize, value: f32);

    /// Return (b_coeffs, a_coeffs) of the transfer function H(z) = B(z)/A(z)
    /// for Bode diagram computation. Return None if not applicable.
    fn transfer_function(&self) -> Option<(Vec<f32>, Vec<f32>)>;
}

pub struct ProcessorParam {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: &'static str,  // e.g. "Hz", "dB", "Q"
}
```

### ProcessorChain

```rust
// dsp/chain.rs
pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    pub fn process(&mut self, sample: f32) -> f32 {
        self.processors.iter_mut().fold(sample, |s, p| p.process(s))
    }
}
```

Adding a new DSP effect = implement `Processor`, register in the UI dropdown. Nothing else changes.

---

## Playback Controls & Seeking

The audio file is fully decoded into a `Vec<f32>` before playback starts (offline decode,
real-time playback). This makes seeking trivial: it's just moving a read cursor index into
that buffer.

```rust
// audio/player.rs
pub struct Player {
    samples: Arc<Vec<f32>>,       // full decoded file, immutable after load
    cursor: Arc<AtomicUsize>,     // current read position, shared with UI
    playing: Arc<AtomicBool>,
}
```

Use `Arc<AtomicUsize>` for the cursor — it's a single integer that both the audio callback
and the UI thread can read/write without a mutex. The audio callback advances it each sample;
the UI writes it when the user drags the seek slider.

```rust
// Audio callback — advance cursor each sample
let pos = cursor.fetch_add(1, Ordering::Relaxed);
let sample = if pos < samples.len() { samples[pos] } else { 0.0 };

// UI thread — seek on slider interaction
cursor.store((seek_fraction * samples.len() as f32) as usize, Ordering::Relaxed);
```

### Seek slider in the UI

The slider displays current position as `MM:SS / MM:SS`. It reads `cursor` each frame to
update its position, and writes `cursor` when dragged. Use `egui::Slider` or a custom
`egui::DragValue`-style widget mapped to `[0.0, 1.0]` normalized position.

When the user seeks, also call `ProcessorChain::reset()` to flush filter delay-line state —
otherwise there will be a brief artifact as the filters drain their old state.



The UI thread and audio thread must communicate in two directions without blocking each other.
**Never use `Arc<Mutex<...>>` on the audio hot path** — if the UI thread holds the lock while
rendering, the audio thread stalls and glitches.

Use crossbeam primitives instead:

```
UI thread  ──[crossbeam_channel]──>  audio thread   (parameter updates)
audio thread  ──[crossbeam_queue]──>  UI thread     (samples for FFT / waveform)
```

### UI → Audio: Parameter Updates

```rust
// Unbounded channel; sender lives in UI, receiver in audio callback closure
let (param_tx, param_rx) = crossbeam_channel::unbounded::<ParamUpdate>();

// UI thread (on slider change):
param_tx.send(ParamUpdate { processor: 0, param: 1, value: 440.0 }).ok();

// Audio callback (non-blocking try_recv):
while let Ok(update) = param_rx.try_recv() {
    chain.apply_update(update);  // updates coefficients, no alloc
}
```

### Audio → UI: Sample Data for FFT / Waveform

```rust
// Fixed-size lock-free queue; audio writes, UI reads
let fft_queue = Arc::new(crossbeam_queue::ArrayQueue::<f32>::new(FFT_SIZE * 2));

// Audio callback:
let _ = fft_queue.push(processed_sample);  // drops sample if full — acceptable

// UI thread (each frame):
while let Some(sample) = fft_queue.pop() {
    ring_buffer.push(sample);
}
// then compute FFT on ring_buffer
```

`ArrayQueue` is fixed-size and allocation-free after construction — safe to use in the audio callback.

---

## Pre-allocation Rule (No Heap Allocation in the Audio Callback)

The cpal callback runs on the OS audio thread every ~5ms. Any heap allocation inside it
risks a priority inversion — the allocator may be locked by another thread → audio dropout.

**Allocate everything before the stream starts. Only reuse inside the callback.**

```rust
// BEFORE the stream — allocate once
let mut chain = ProcessorChain::new();       // allocates all filter state upfront
let fft_queue = Arc::new(ArrayQueue::new(FFT_SIZE * 2));

// INSIDE the cpal callback — no alloc
move |data: &mut [f32], _| {
    while let Ok(update) = param_rx.try_recv() {
        chain.apply_update(update);          // recomputes coefficients, no alloc
    }
    for sample in data.iter_mut() {
        let s = chain.process(next_sample());
        let _ = fft_queue.push(s);
        *sample = s;
    }
}
```

Filter internal state (delay lines, tap buffers, IIR state vectors) must be initialized at
construction and `reset()` must zero them — never reallocate.

---

## Spectrogram

The spectrogram is a time-frequency heatmap: the X axis is time, the Y axis is frequency
(log-scaled), and pixel color encodes magnitude in dB. It gives a full picture of how the
spectrum evolves over time — complementary to the FFT spectrum which only shows the current
frame.

### Implementation

The spectrogram is built by accumulating FFT frames over time into a 2D buffer:

```rust
// ui/spectrogram.rs
pub struct Spectrogram {
    /// Ring buffer of FFT frames: each column is one FFT frame (magnitude in dB per bin)
    frames: VecDeque<Vec<f32>>,  // pre-allocated at construction, max_frames columns
    max_frames: usize,           // width in pixels / columns
    fft_size: usize,             // height in frequency bins
}

impl Spectrogram {
    pub fn push_frame(&mut self, magnitude_db: &[f32]) {
        if self.frames.len() == self.max_frames {
            self.frames.pop_front();  // drop oldest column
        }
        // reuse pre-allocated vec from a pool if possible to avoid alloc on audio thread
        self.frames.push_back(magnitude_db.to_vec());
    }
}
```

**Important:** the spectrogram buffer is updated from the UI thread (not the audio thread)
using samples already drained from the `crossbeam_queue` FFT buffer. Do not write to it
from the audio callback.

### Rendering

`egui_plot` cannot render a heatmap natively — use `egui`'s custom painter instead:

```rust
// For each frame column, for each frequency bin row:
// map magnitude_db → color via a colormap (e.g. viridis-like: black → blue → yellow → white)
// draw a filled rectangle at the correct (time, frequency) position
painter.rect_filled(rect, 0.0, color);
```

Use a perceptually uniform colormap. A simple approximation:
- Below noise floor (e.g. < -80 dB) → black
- -80 dB → -40 dB → blue to cyan
- -40 dB → 0 dB → yellow to white

Apply a Hann window and the same FFT pipeline as the spectrum view — do not recompute
separately. The spectrogram and spectrum share the same FFT frames.

### Parameters exposed in UI
- **Time range:** how many seconds of history to display (controls `max_frames`)
- **Frequency range:** min/max frequency shown on Y axis (default 20 Hz – 20 kHz)
- **dB floor:** noise floor threshold for color mapping (default -80 dB)



The Bode diagram is computed **analytically** from the filter's transfer function — not by
running audio through it. It updates whenever filter parameters change.

```rust
// dsp/bode.rs
pub struct BodePlot {
    pub frequencies: Vec<f32>,   // log-spaced, e.g. 20 Hz – 20 kHz
    pub magnitude_db: Vec<f32>,  // 20 * log10(|H(e^jw)|)
    pub phase_deg: Vec<f32>,     // arg(H(e^jw)) in degrees
}

pub fn compute_bode(b: &[f32], a: &[f32], sample_rate: f32, n_points: usize) -> BodePlot {
    // Evaluate H(e^jw) = B(e^jw) / A(e^jw) at n_points log-spaced frequencies
    // w = 2 * pi * f / sample_rate
}
```

Each `Processor` exposes its transfer function coefficients via `transfer_function()`.
The UI calls `compute_bode()` on the selected processor whenever params change and plots
magnitude (top) and phase (bottom) using `egui_plot`.

---

## DSP Processors (Initial Set)

All IIR filters use fundsp internally. The math correctness is fundsp's responsibility;
the `Processor` wrapper handles parameter management, reset, and transfer function export.

| Processor          | Implementation | Parameters                                    | Course section |
|--------------------|----------------|-----------------------------------------------|----------------|
| Moving Average     | Manual         | N (number of samples)                         | 2.3.3          |
| FIR Low-Pass       | fundsp `fir()` | Cutoff freq, order, window type               | —              |
| FIR High-Pass      | fundsp `fir()` | Cutoff freq, order, window type               | —              |
| IIR Butterworth    | fundsp         | Cutoff freq, order, type (LP/HP/BP)           | 3.2            |
| IIR Bessel         | fundsp         | Cutoff freq, order, type (LP/HP/BP)           | 3.3            |
| IIR Chebyshev I    | fundsp         | Cutoff freq, order, passband ripple (dB)      | 3.4            |
| IIR Chebyshev II   | fundsp         | Cutoff freq, order, stopband attenuation (dB) | 3.4            |
| Elliptic (Cauer)   | fundsp         | Cutoff freq, order, ripple (dB), atten. (dB)  | 3.5            |
| Biquad             | fundsp         | Type (LP/HP/BP), cutoff freq, Q factor        | —              |

### Notes per filter

**Moving average:** implement manually as a ring buffer of N samples — no fundsp needed.
Single parameter: N (window size). Cutoff frequency is approximately
`fc ≈ (1 / (2π)) * sqrt(12 / (2 - sqrt(2))) * (fe / N)` per the course formula.
Simplest filter to implement; good first processor to test the pipeline end-to-end.

**Bessel:** optimized for linear phase / minimal group delay distortion, at the cost of a
slower roll-off than Butterworth. Unlike other filters, its cutoff can be specified in terms
of group delay (in seconds) rather than -3dB gain — expose both options if fundsp supports it.

**Chebyshev I vs II:** Type I has ripple in the passband and a monotone stopband. Type II is
the inverse: flat passband, ripple in the stopband. Implement as two separate processors
since their parameters and trade-offs differ meaningfully.

**Elliptic (Cauer):** sharpest transition band of all filters, at the cost of ripple in
both passband and stopband. Two ripple parameters: passband ripple Gp (dB) and stopband
attenuation Ga (dB). Most complex filter — implement last.

---

## UI Layout (egui)

```
+-------------------------------------------------+
|  [Open File]  [Play/Pause] [Stop]  File: foo.wav|
|  0:00 ──────●────────────────────── 3:45        |
+-------------------------------------------------+
|  Chain: [ Butterworth ] -> [ Biquad ]  [+ Add]  |
+-------------------------------------------------+
|  Waveform (original)   |  Waveform (processed)  |
+------------------------+------------------------+
|         FFT Spectrum (original vs processed)    |
+-------------------------------------------------+
|         Spectrogram (processed, time x freq)    |
+-------------------------------------------------+
|         Bode — Magnitude  (selected processor)  |
|         Bode — Phase      (selected processor)  |
+-------------------------------------------------+
|  Params: Cutoff ──●──  Q ────●─  Type [LP v]   |
+-------------------------------------------------+
```

Use `egui_plot` for all plots. All plots update at 30+ fps during playback.
The Bode diagram recomputes on every parameter change (cheap — analytical, not real-time).

---

## Coding Rules

- **No unsafe code** unless strictly required by cpal internals.
- **No heap allocation in the audio callback.** Pre-allocate all buffers and filter state.
- **No `unwrap()` in the audio thread** — a panic in a callback kills the stream silently.
- **No `Arc<Mutex<...>>` on the audio hot path** — use crossbeam primitives instead.
- Use `anyhow` for application-level errors, `thiserror` for library/module errors.
- Use `log` + `env_logger` for debugging. No `println!` in hot paths.
- Run `cargo clippy -- -D warnings` before considering any feature done.
- Format with `cargo fmt` always.

---

## Dev Workflow for Claude Code

1. **Scaffold first.** Create all modules with empty stubs and a working eframe window before
   any DSP logic.
2. **Wire the audio pipeline with a sine wave.** Before loading real files, generate a
   synthetic sine in code, pass it through `ProcessorChain`, and verify you hear it.
3. **One processor at a time.** Implement and test each `Processor` in isolation before
   wiring into the UI.
4. **Bode last.** Add the Bode diagram after all processors are working — it's a pure UI
   feature and depends on `transfer_function()` being correctly implemented.
5. Run `cargo test` and `cargo clippy` after each completed feature.
6. Use `/clear` in Claude Code between major features to keep context clean.

---

## Out of Scope (for now)

- MP3 encoding (decoding only via symphonia)
- Stereo / multi-channel — mono only for now
- Network streaming
- VST/AU plugin format
- Preset save/load (good future addition)
