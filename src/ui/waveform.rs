use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_queue::ArrayQueue;
use egui::Color32;
use egui_plot::{GridMark, HLine, Line, Plot, PlotPoint, PlotPoints, Text};

use crate::ui::theme::{ACCENT_BLUE, ACCENT_ORANGE, TEXT_MUTED, draw_panel_header};

const BUF_SIZE: usize = 4096;

pub struct Waveform {
    processed_buf: Vec<f32>,
    write_head: usize,
    scratch: Vec<f32>,
    orig_points_buf: Vec<[f64; 2]>,
    proc_points_buf: Vec<[f64; 2]>,
}

impl Default for Waveform {
    fn default() -> Self {
        Self {
            processed_buf: vec![0.0f32; BUF_SIZE],
            write_head: 0,
            scratch: vec![0.0f32; BUF_SIZE],
            orig_points_buf: Vec::with_capacity(512),
            proc_points_buf: Vec::with_capacity(512),
        }
    }
}

fn downsample(data: &[f32], target_points: usize, display_ms: f64, out: &mut Vec<[f64; 2]>) {
    out.clear();
    if data.is_empty() || target_points == 0 {
        return;
    }
    let chunks = (target_points / 2).max(1);
    let chunk_size = (data.len() / chunks).max(1);
    for (chunk_idx, chunk) in data.chunks(chunk_size).enumerate() {
        let x = chunk_idx as f64 * (display_ms / chunks as f64);
        let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min) as f64;
        let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max) as f64;
        out.push([x, min]);
        out.push([x, max]);
    }
}

impl Waveform {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        samples: &Arc<Vec<f32>>,
        cursor: &Arc<AtomicUsize>,
        sample_rate: u32,
        waveform_queue: &Arc<ArrayQueue<f32>>,
        playing: bool,
    ) {
        if playing {
            while let Some(s) = waveform_queue.pop() {
                self.processed_buf[self.write_head] = s;
                self.write_head = (self.write_head + 1) % BUF_SIZE;
            }
        }

        let display = ((sample_rate as usize * 100) / 1000).clamp(256, BUF_SIZE);

        let pos = cursor.load(Ordering::Relaxed);
        let half = display / 2;
        let start = pos.saturating_sub(half);
        let end = (start + display).min(samples.len());
        downsample(&samples[start..end], 512, 100.0, &mut self.orig_points_buf);

        let n = display;
        let ring_start = (self.write_head + BUF_SIZE - n) % BUF_SIZE;
        if ring_start + n <= BUF_SIZE {
            self.scratch[..n].copy_from_slice(&self.processed_buf[ring_start..ring_start + n]);
        } else {
            let first = BUF_SIZE - ring_start;
            self.scratch[..first].copy_from_slice(&self.processed_buf[ring_start..]);
            self.scratch[first..n].copy_from_slice(&self.processed_buf[..n - first]);
        }
        downsample(&self.scratch[..n], 512, 100.0, &mut self.proc_points_buf);

        let orig_points = PlotPoints::new(self.orig_points_buf.clone());
        let proc_points = PlotPoints::new(self.proc_points_buf.clone());

        let grid_color =
            Color32::from_rgba_unmultiplied(TEXT_MUTED.r(), TEXT_MUTED.g(), TEXT_MUTED.b(), 51);
        let center_line_color = Color32::from_rgba_unmultiplied(
            ACCENT_BLUE.r(),
            ACCENT_BLUE.g(),
            ACCENT_BLUE.b(),
            76,
        );

        draw_panel_header(ui, "WAVEFORM", ACCENT_BLUE, ACCENT_BLUE);

        ui.columns(2, |cols| {
            Plot::new("waveform_original")
                .height(150.0)
                .include_y(-1.0)
                .include_y(1.0)
                .x_axis_label("Time (ms)")
                .y_axis_label("Amplitude")
                .show_axes([true, true])
                .show_grid([true, true])
                .y_grid_spacer(|_input| {
                    [-1.0_f64, -0.5, 0.0, 0.5, 1.0]
                        .iter()
                        .map(|&v| GridMark {
                            value: v,
                            step_size: 0.5,
                        })
                        .collect()
                })
                .label_formatter(|_, _| String::new())
                .show(&mut cols[0], |plot_ui| {
                    for (i, &y) in [-1.0_f64, -0.5, 0.0, 0.5, 1.0].iter().enumerate() {
                        plot_ui.hline(
                            HLine::new(format!("g{i}"), y)
                                .color(grid_color)
                                .width(0.5),
                        );
                    }
                    plot_ui.hline(
                        HLine::new("center", 0.0)
                            .color(center_line_color)
                            .width(1.0),
                    );
                    plot_ui.text(
                        Text::new("orig_label", PlotPoint::new(1.0, 0.90), "ORIGINAL")
                            .color(TEXT_MUTED)
                            .anchor(egui::Align2::LEFT_TOP),
                    );
                    plot_ui.line(Line::new("Original", orig_points).color(ACCENT_BLUE));
                });

            Plot::new("waveform_processed")
                .height(150.0)
                .include_y(-1.0)
                .include_y(1.0)
                .x_axis_label("Time (ms)")
                .y_axis_label("Amplitude")
                .show_axes([true, true])
                .show_grid([true, true])
                .y_grid_spacer(|_input| {
                    [-1.0_f64, -0.5, 0.0, 0.5, 1.0]
                        .iter()
                        .map(|&v| GridMark {
                            value: v,
                            step_size: 0.5,
                        })
                        .collect()
                })
                .label_formatter(|_, _| String::new())
                .show(&mut cols[1], |plot_ui| {
                    for (i, &y) in [-1.0_f64, -0.5, 0.0, 0.5, 1.0].iter().enumerate() {
                        plot_ui.hline(
                            HLine::new(format!("g{i}"), y)
                                .color(grid_color)
                                .width(0.5),
                        );
                    }
                    plot_ui.hline(
                        HLine::new("center", 0.0)
                            .color(center_line_color)
                            .width(1.0),
                    );
                    plot_ui.text(
                        Text::new("proc_label", PlotPoint::new(1.0, 0.90), "PROCESSED")
                            .color(TEXT_MUTED)
                            .anchor(egui::Align2::LEFT_TOP),
                    );
                    plot_ui.line(Line::new("Processed", proc_points).color(ACCENT_ORANGE));
                });
        });
    }
}
