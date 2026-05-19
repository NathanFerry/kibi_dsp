use egui_plot::{GridInput, GridMark, Line, LineStyle, Plot, PlotPoints, VLine};

use crate::dsp::bode::compute_bode;

const N_POINTS: usize = 512;

// Log-spaced tick positions and labels for the frequency axis.
static LOG_TICKS: &[(f64, &str)] = &[
    (20.0, "20"),
    (50.0, "50"),
    (100.0, "100"),
    (200.0, "200"),
    (500.0, "500"),
    (1_000.0, "1k"),
    (2_000.0, "2k"),
    (5_000.0, "5k"),
    (10_000.0, "10k"),
    (20_000.0, "20k"),
];

pub struct BodeView {
    /// Pre-allocated point buffers: X = log10(Hz), Y = dB or degrees.
    mag_points: Vec<[f64; 2]>,
    phase_points: Vec<[f64; 2]>,
    dirty: bool,
}

impl BodeView {
    pub fn new() -> Self {
        Self {
            mag_points: Vec::with_capacity(N_POINTS),
            phase_points: Vec::with_capacity(N_POINTS),
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Render both the magnitude and phase panels.
    ///
    /// `tf`        — (b_coeffs, a_coeffs) of the selected processor, or `None`.
    /// `cutoff_hz` — first parameter value (used to draw the cutoff marker).
    /// `sample_rate` — audio sample rate in Hz.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        tf: Option<(&[f32], &[f32])>,
        cutoff_hz: Option<f32>,
        sample_rate: f32,
    ) {
        if self.dirty {
            self.mag_points.clear();
            self.phase_points.clear();

            if let Some((b, a)) = tf {
                let plot = compute_bode(b, a, sample_rate, N_POINTS);
                for i in 0..plot.frequencies.len() {
                    let log_f = (plot.frequencies[i] as f64).log10();
                    self.mag_points.push([log_f, plot.magnitude_db[i] as f64]);
                    self.phase_points.push([log_f, plot.phase_deg[i] as f64]);
                }
            }

            self.dirty = false;
        }

        let cutoff_log = cutoff_hz.map(|f| (f as f64).log10());
        let f_min_log = 20_f64.log10();
        let f_max_log = (sample_rate as f64 / 2.0).min(20_000.0).log10();

        // — Magnitude —
        let mag_pts = PlotPoints::new(self.mag_points.clone());
        Plot::new("bode_magnitude")
            .height(160.0)
            .include_y(-80.0)
            .include_y(10.0)
            .include_x(f_min_log)
            .include_x(f_max_log)
            .x_axis_label("Frequency (Hz)")
            .y_axis_label("dB")
            .x_axis_formatter(fmt_freq_axis)
            .x_grid_spacer(freq_grid_spacer)
            .y_grid_spacer(mag_y_grid_spacer)
            .show_axes([true, true])
            .show_grid([true, true])
            .label_formatter(|name, v| {
                if name.is_empty() {
                    String::new()
                } else {
                    let f = 10_f64.powf(v.x);
                    format!("{name}: {:.0} Hz  {:.1} dB", f, v.y)
                }
            })
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new("Magnitude", mag_pts).color(egui::Color32::from_rgb(100, 200, 100)),
                );
                if let Some(c) = cutoff_log {
                    plot_ui.vline(
                        VLine::new("Cutoff", c)
                            .color(egui::Color32::from_rgb(255, 80, 80))
                            .style(LineStyle::Dashed { length: 10.0 }),
                    );
                }
            });

        // — Phase —
        let phase_pts = PlotPoints::new(self.phase_points.clone());
        Plot::new("bode_phase")
            .height(120.0)
            .include_y(-200.0)
            .include_y(200.0)
            .include_x(f_min_log)
            .include_x(f_max_log)
            .x_axis_label("Frequency (Hz)")
            .y_axis_label("°")
            .x_axis_formatter(fmt_freq_axis)
            .x_grid_spacer(freq_grid_spacer)
            .y_grid_spacer(phase_y_grid_spacer)
            .show_axes([true, true])
            .show_grid([true, true])
            .label_formatter(|name, v| {
                if name.is_empty() {
                    String::new()
                } else {
                    let f = 10_f64.powf(v.x);
                    format!("{name}: {:.0} Hz  {:.1}°", f, v.y)
                }
            })
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new("Phase", phase_pts).color(egui::Color32::from_rgb(200, 150, 50)),
                );
                if let Some(c) = cutoff_log {
                    plot_ui.vline(
                        VLine::new("Cutoff", c)
                            .color(egui::Color32::from_rgb(255, 80, 80))
                            .style(LineStyle::Dashed { length: 10.0 }),
                    );
                }
            });
    }
}

impl Default for BodeView {
    fn default() -> Self {
        Self::new()
    }
}

// ── axis helpers ──────────────────────────────────────────────────────────────

fn fmt_freq_axis(mark: GridMark, _range: &std::ops::RangeInclusive<f64>) -> String {
    let f = 10_f64.powf(mark.value);
    if f >= 1_000.0 {
        format!("{:.0}k", f / 1_000.0)
    } else {
        format!("{:.0}", f)
    }
}

fn freq_grid_spacer(_input: GridInput) -> Vec<GridMark> {
    LOG_TICKS
        .iter()
        .map(|(f, _)| GridMark {
            value: f.log10(),
            step_size: 1.0,
        })
        .collect()
}

fn mag_y_grid_spacer(_input: GridInput) -> Vec<GridMark> {
    (-4..=1)
        .map(|i| GridMark {
            value: i as f64 * 20.0,
            step_size: 20.0,
        })
        .collect()
}

fn phase_y_grid_spacer(_input: GridInput) -> Vec<GridMark> {
    [-180.0, -90.0, 0.0, 90.0, 180.0]
        .iter()
        .map(|&v| GridMark {
            value: v,
            step_size: 90.0,
        })
        .collect()
}
