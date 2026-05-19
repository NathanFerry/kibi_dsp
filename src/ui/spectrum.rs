use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use egui_plot::{Line, Plot, PlotPoints};
use rustfft::{Fft, FftPlanner, num_complex::Complex};

const FFT_SIZE: usize = 2048;

pub struct Spectrum {
    // Ring buffers — one for the raw pre-filter signal, one for the processed signal.
    orig_ring: Vec<f32>,
    orig_write: usize,
    proc_ring: Vec<f32>,
    proc_write: usize,
    // Magnitude output in dB, FFT_SIZE / 2 bins each.
    orig_mag: Vec<f32>,
    proc_mag: Vec<f32>,
    // Reusable FFT workspace — allocated once, never reallocated.
    fft_buf: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    fft_plan: Arc<dyn Fft<f32>>,
    hann: Vec<f32>,
    // Dirty flags: FFT is only recomputed when new samples arrived.
    orig_dirty: bool,
    proc_dirty: bool,
    // Pre-allocated [freq_hz, mag_db] pairs for egui_plot.
    orig_points: Vec<[f64; 2]>,
    proc_points: Vec<[f64; 2]>,
}

impl Spectrum {
    pub fn new() -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft_plan = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft_plan.get_inplace_scratch_len();
        let hann = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
            })
            .collect();
        Self {
            orig_ring: vec![0.0f32; FFT_SIZE],
            orig_write: 0,
            proc_ring: vec![0.0f32; FFT_SIZE],
            proc_write: 0,
            orig_mag: vec![-120.0f32; FFT_SIZE / 2],
            proc_mag: vec![-120.0f32; FFT_SIZE / 2],
            fft_buf: vec![Complex::new(0.0f32, 0.0f32); FFT_SIZE],
            fft_scratch: vec![Complex::new(0.0f32, 0.0f32); scratch_len],
            fft_plan,
            hann,
            orig_dirty: false,
            proc_dirty: false,
            orig_points: vec![[0.0, -120.0]; FFT_SIZE / 2],
            proc_points: vec![[0.0, -120.0]; FFT_SIZE / 2],
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        orig_queue: &Arc<ArrayQueue<f32>>,
        fft_queue: &Arc<ArrayQueue<f32>>,
        sample_rate: u32,
    ) {
        while let Some(s) = orig_queue.pop() {
            self.orig_ring[self.orig_write] = s;
            self.orig_write = (self.orig_write + 1) % FFT_SIZE;
            self.orig_dirty = true;
        }
        while let Some(s) = fft_queue.pop() {
            self.proc_ring[self.proc_write] = s;
            self.proc_write = (self.proc_write + 1) % FFT_SIZE;
            self.proc_dirty = true;
        }

        if self.orig_dirty {
            compute_fft(
                &self.orig_ring,
                self.orig_write,
                &mut self.fft_buf,
                &mut self.fft_scratch,
                &self.fft_plan,
                &self.hann,
                &mut self.orig_mag,
                &mut self.orig_points,
                sample_rate,
            );
            self.orig_dirty = false;
        }
        if self.proc_dirty {
            compute_fft(
                &self.proc_ring,
                self.proc_write,
                &mut self.fft_buf,
                &mut self.fft_scratch,
                &self.fft_plan,
                &self.hann,
                &mut self.proc_mag,
                &mut self.proc_points,
                sample_rate,
            );
            self.proc_dirty = false;
        }

        // Clone the filled point slices — ≤ 1024 × 16 bytes each.
        let orig_plot = PlotPoints::new(self.orig_points.clone());
        let proc_plot = PlotPoints::new(self.proc_points.clone());

        Plot::new("spectrum")
            .height(200.0)
            .include_y(0.0)
            .include_y(-120.0)
            .include_x(0.0)
            .label_formatter(|name, v| {
                if name.is_empty() {
                    String::new()
                } else {
                    format!("{name}: {:.0} Hz  {:.1} dB", v.x, v.y)
                }
            })
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new("Original", orig_plot).color(egui::Color32::from_rgb(100, 160, 255)),
                );
                plot_ui.line(
                    Line::new("Processed", proc_plot).color(egui::Color32::from_rgb(255, 140, 60)),
                );
            });
    }
}

impl Default for Spectrum {
    fn default() -> Self {
        Self::new()
    }
}

/// Fill `mag` and `points` from the ring buffer, applying a Hann window and running
/// the FFT in-place. No allocation — all workspace is passed in by the caller.
#[allow(clippy::too_many_arguments)]
fn compute_fft(
    ring: &[f32],
    write: usize,
    fft_buf: &mut [Complex<f32>],
    fft_scratch: &mut [Complex<f32>],
    fft_plan: &Arc<dyn Fft<f32>>,
    hann: &[f32],
    mag: &mut [f32],
    points: &mut [[f64; 2]],
    sample_rate: u32,
) {
    // Unwrap the ring buffer oldest→newest into fft_buf, applying the window.
    for i in 0..FFT_SIZE {
        let ring_pos = (write + i) % FFT_SIZE;
        fft_buf[i] = Complex {
            re: ring[ring_pos] * hann[i],
            im: 0.0,
        };
    }
    fft_plan.process_with_scratch(fft_buf, fft_scratch);
    // Normalise by FFT_SIZE so a full-scale sine reads ~ 0 dBFS.
    let norm_factor = FFT_SIZE as f32;
    for i in 0..FFT_SIZE / 2 {
        let db = 20.0_f32 * (fft_buf[i].norm() / norm_factor).max(1e-6_f32).log10();
        mag[i] = db;
        let freq = i as f64 * sample_rate as f64 / FFT_SIZE as f64;
        points[i] = [freq, db as f64];
    }
}
