use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use egui_plot::{GridMark, Line, Plot, PlotPoints};
use rustfft::{Fft, FftPlanner, num_complex::Complex};

const FFT_SIZE: usize = 2048;

pub struct Spectrum {
    orig_ring: Vec<f32>,
    orig_write: usize,
    orig_mag: Vec<f32>,
    orig_dirty: bool,
    orig_points: Vec<[f64; 2]>,
    fft_buf: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    fft_plan: Arc<dyn Fft<f32>>,
    hann: Vec<f32>,
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
            orig_mag: vec![-120.0f32; FFT_SIZE / 2],
            orig_dirty: false,
            orig_points: vec![[0.0, -120.0]; FFT_SIZE / 2],
            fft_buf: vec![Complex::new(0.0f32, 0.0f32); FFT_SIZE],
            fft_scratch: vec![Complex::new(0.0f32, 0.0f32); scratch_len],
            fft_plan,
            hann,
        }
    }

    /// `proc_mag` is the latest spectrogram FFT frame (magnitude in dB, len = HALF).
    /// The spectrogram is the sole consumer of `fft_queue`; we receive the pre-computed
    /// processed magnitudes here to avoid double-draining the queue.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        orig_queue: &Arc<ArrayQueue<f32>>,
        proc_mag: &[f32],
        sample_rate: u32,
        playing: bool,
    ) {
        if playing {
            while let Some(s) = orig_queue.pop() {
                self.orig_ring[self.orig_write] = s;
                self.orig_write = (self.orig_write + 1) % FFT_SIZE;
                self.orig_dirty = true;
            }
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

        // Build processed plot points from the spectrogram's latest frame.
        let proc_fft_size = proc_mag.len() * 2;
        let proc_plot = PlotPoints::new(
            proc_mag
                .iter()
                .enumerate()
                .map(|(i, &db)| {
                    let freq = i as f64 * sample_rate as f64 / proc_fft_size as f64;
                    [freq, db as f64]
                })
                .collect::<Vec<_>>(),
        );

        let orig_plot = PlotPoints::new(self.orig_points.clone());

        Plot::new("spectrum")
            .height(200.0)
            .include_y(0.0)
            .include_y(-120.0)
            .include_x(0.0)
            .x_axis_label("Frequency (Hz)")
            .y_axis_label("dBFS")
            .show_axes([true, true])
            .show_grid([true, true])
            .x_grid_spacer(|_input| {
                [
                    20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
                ]
                .iter()
                .map(|&v| GridMark {
                    value: v,
                    step_size: v,
                })
                .collect()
            })
            .y_grid_spacer(|_input| {
                (-6..=0)
                    .map(|i| GridMark {
                        value: i as f64 * 20.0,
                        step_size: 20.0,
                    })
                    .collect()
            })
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
    for i in 0..FFT_SIZE {
        let ring_pos = (write + i) % FFT_SIZE;
        fft_buf[i] = Complex {
            re: ring[ring_pos] * hann[i],
            im: 0.0,
        };
    }
    fft_plan.process_with_scratch(fft_buf, fft_scratch);
    let norm_factor = (FFT_SIZE as f32).sqrt();
    for i in 0..FFT_SIZE / 2 {
        let db = 20.0_f32 * (fft_buf[i].norm() / norm_factor).max(1e-6_f32).log10();
        mag[i] = db;
        let freq = i as f64 * sample_rate as f64 / FFT_SIZE as f64;
        points[i] = [freq, db as f64];
    }
}
