use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use rustfft::{Fft, FftPlanner, num_complex::Complex};

use crate::ui::theme::{
    ACCENT_BLUE, ACCENT_PINK, ACCENT_YELLOW, BACKGROUND, draw_panel_header,
};

const FFT_SIZE: usize = 1024;
pub const HALF: usize = FFT_SIZE / 2;
const MAX_FRAMES: usize = 512;
const HOP_SIZE: usize = 256;
const PLOT_HEIGHT: f32 = 200.0;

pub struct Spectrogram {
    ring: Vec<f32>,
    ring_write: usize,
    hop_counter: usize,
    /// Ring of FFT frames; each frame is HALF magnitude_db values.
    frames: Vec<Vec<f32>>,
    /// Next write position in `frames` (oldest frame when reading).
    frame_write: usize,
    hann: Vec<f32>,
    fft_plan: Arc<dyn Fft<f32>>,
    fft_buf: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    db_floor: f32,
}

impl Spectrogram {
    pub fn new() -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft_plan = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft_plan.get_inplace_scratch_len();
        let hann: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
            })
            .collect();
        Self {
            ring: vec![0.0f32; FFT_SIZE],
            ring_write: 0,
            hop_counter: 0,
            frames: vec![vec![-120.0f32; HALF]; MAX_FRAMES],
            frame_write: 0,
            hann,
            fft_plan,
            fft_buf: vec![Complex::new(0.0f32, 0.0f32); FFT_SIZE],
            fft_scratch: vec![Complex::new(0.0f32, 0.0f32); scratch_len],
            db_floor: -60.0,
        }
    }

    /// Drain `fft_queue`, update the ring buffer, and compute new FFT frames.
    /// Call this once per UI frame before reading `latest_magnitude_db`.
    /// When `playing` is false the queue is left untouched so the last frame stays frozen.
    pub fn update(&mut self, fft_queue: &Arc<ArrayQueue<f32>>, playing: bool) {
        if !playing {
            return;
        }
        while let Some(s) = fft_queue.pop() {
            self.ring[self.ring_write] = s;
            self.ring_write = (self.ring_write + 1) % FFT_SIZE;
            self.hop_counter += 1;
            if self.hop_counter >= HOP_SIZE {
                self.hop_counter = 0;
                self.compute_frame();
            }
        }
    }

    /// The most recently computed FFT frame (HALF magnitude_db values).
    pub fn latest_magnitude_db(&self) -> &[f32] {
        let idx = (self.frame_write + MAX_FRAMES - 1) % MAX_FRAMES;
        &self.frames[idx]
    }

    pub fn reset(&mut self) {
        for frame in &mut self.frames {
            frame.fill(-120.0);
        }
        self.ring.fill(0.0);
        self.ring_write = 0;
        self.hop_counter = 0;
        self.frame_write = 0;
    }

    fn compute_frame(&mut self) {
        for i in 0..FFT_SIZE {
            let ring_pos = (self.ring_write + i) % FFT_SIZE;
            self.fft_buf[i] = Complex {
                re: self.ring[ring_pos] * self.hann[i],
                im: 0.0,
            };
        }
        self.fft_plan
            .process_with_scratch(&mut self.fft_buf, &mut self.fft_scratch);
        let norm = (FFT_SIZE as f32).sqrt();
        let frame = &mut self.frames[self.frame_write];
        for (i, slot) in frame.iter_mut().enumerate().take(HALF) {
            *slot = 20.0 * (self.fft_buf[i].norm() / norm).max(1e-6).log10();
        }
        self.frame_write = (self.frame_write + 1) % MAX_FRAMES;
    }

    /// Render the heatmap and the dB floor slider.
    pub fn show(&mut self, ui: &mut egui::Ui, sample_rate: u32) {
        draw_panel_header(ui, "SPECTROGRAM", ACCENT_PINK, ACCENT_PINK);

        ui.add(
            egui::Slider::new(&mut self.db_floor, -120.0f32..=-40.0)
                .text("Spectrogram dB floor")
                .suffix(" dB"),
        );

        let available_w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(available_w, PLOT_HEIGHT), Sense::hover());

        if !ui.is_rect_visible(rect) {
            return;
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, BACKGROUND);

        // Reserve 8px on the right for the color legend
        let legend_w = 8.0;
        let heatmap_right = rect.right() - legend_w - 20.0; // extra space for labels
        let heatmap_rect = Rect::from_min_max(rect.min, Pos2::new(heatmap_right, rect.max.y));

        let col_w = heatmap_rect.width() / MAX_FRAMES as f32;
        let n_rows = PLOT_HEIGHT as usize; // one pixel per row

        for col in 0..MAX_FRAMES {
            let frame_idx = (self.frame_write + col) % MAX_FRAMES;
            let frame = &self.frames[frame_idx];
            let x = heatmap_rect.left() + col as f32 * col_w;

            for row in 0..n_rows {
                // row 0 = top = Nyquist, row n_rows-1 = bottom = 0 Hz
                let t = row as f32 / (n_rows.saturating_sub(1).max(1)) as f32;
                let bin = ((1.0 - t) * (HALF - 1) as f32).round() as usize;
                let bin = bin.min(HALF - 1);

                let db = frame[bin];
                if db <= self.db_floor {
                    continue;
                }

                let cell_rect = Rect::from_min_size(
                    Pos2::new(x, rect.top() + row as f32),
                    Vec2::new(col_w.max(1.0), 1.0),
                );
                painter.rect_filled(cell_rect, 0.0, db_to_color(db, self.db_floor));
            }
        }

        // Frequency markers
        let nyquist = sample_rate as f32 / 2.0;
        let markers: &[(f32, &str)] = &[
            (500.0, "500 Hz"),
            (1_000.0, "1kHz"),
            (5_000.0, "5kHz"),
            (10_000.0, "10kHz"),
            (20_000.0, "20kHz"),
        ];
        for &(freq, label) in markers {
            if freq > nyquist {
                continue;
            }
            let frac = freq / nyquist;
            let y = rect.bottom() - frac * rect.height();
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(heatmap_right, y)],
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 60)),
            );
            painter.text(
                Pos2::new(rect.left() + 2.0, y - 1.0),
                Align2::LEFT_BOTTOM,
                label,
                FontId::proportional(10.0),
                Color32::from_gray(200),
            );
        }

        // Axis labels
        painter.text(
            Pos2::new(heatmap_rect.center().x, rect.bottom() - 4.0),
            Align2::CENTER_BOTTOM,
            "Time",
            FontId::proportional(11.0),
            Color32::from_gray(180),
        );
        painter.text(
            Pos2::new(rect.left() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            "Frequency (Hz)",
            FontId::proportional(11.0),
            Color32::from_gray(180),
        );

        // ── Color legend ───────────────────────────────────────────────────────
        let legend_bar_rect = Rect::from_min_size(
            Pos2::new(rect.right() - legend_w, rect.top()),
            Vec2::new(legend_w, rect.height()),
        );

        // Gradient bar: bottom = BACKGROUND (lowest energy), top = WHITE (peak energy)
        let n_legend = rect.height() as usize;
        for row in 0..n_legend {
            // row 0 = top = max energy, row n_legend-1 = bottom = min energy
            let t = 1.0 - row as f32 / (n_legend as f32 - 1.0).max(1.0);
            let db = self.db_floor + t * (-self.db_floor);
            let color = db_to_color(db, self.db_floor);
            let cell = Rect::from_min_size(
                Pos2::new(legend_bar_rect.left(), rect.top() + row as f32),
                Vec2::new(legend_w, 1.0),
            );
            painter.rect_filled(cell, 0.0, color);
        }

        // dB labels at fixed positions
        for &db in &[-60.0_f32, -40.0, -20.0, 0.0] {
            if db < self.db_floor {
                continue;
            }
            let t = (db - self.db_floor) / (-self.db_floor);
            let y = rect.bottom() - t * rect.height();
            // Tick mark
            painter.line_segment(
                [
                    Pos2::new(legend_bar_rect.left() - 3.0, y),
                    Pos2::new(legend_bar_rect.left(), y),
                ],
                Stroke::new(0.5, Color32::from_gray(180)),
            );
            // Label
            painter.text(
                Pos2::new(legend_bar_rect.left() - 5.0, y),
                Align2::RIGHT_CENTER,
                format!("{:.0}", db),
                FontId::proportional(9.0),
                Color32::from_gray(180),
            );
        }
    }
}

fn db_to_color(db: f32, db_floor: f32) -> Color32 {
    // Normalize to [0.0, 1.0] over the range [db_floor, 0 dB].
    let t = ((db - db_floor) / (0.0_f32 - db_floor)).clamp(0.0, 1.0);

    // Accent-palette colormap:
    // 0.00 → BACKGROUND (#1a1a2e)  — below noise floor
    // 0.33 → ACCENT_BLUE (#4cc9f0) — low energy
    // 0.66 → ACCENT_PINK (#f72585) — mid energy
    // 1.00 → WHITE                 — peak energy
    if t < 0.33 {
        lerp_color(BACKGROUND, ACCENT_BLUE, t / 0.33)
    } else if t < 0.66 {
        lerp_color(ACCENT_BLUE, ACCENT_PINK, (t - 0.33) / 0.33)
    } else if t < 0.90 {
        lerp_color(ACCENT_PINK, ACCENT_YELLOW, (t - 0.66) / 0.24)
    } else {
        lerp_color(ACCENT_YELLOW, Color32::WHITE, (t - 0.90) / 0.10)
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

impl Default for Spectrogram {
    fn default() -> Self {
        Self::new()
    }
}
