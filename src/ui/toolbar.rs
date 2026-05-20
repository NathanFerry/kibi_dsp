use std::sync::atomic::Ordering;

use crate::audio::player::Player;
use crate::ui::widgets::vu_meter::VuMeter;

#[derive(Default)]
pub struct Toolbar {
    vu_meter: VuMeter,
}

impl Toolbar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the toolbar. Returns `true` when the user clicks "Open File".
    /// `vu_level` is the peak output amplitude in `[0.0, 1.0]` (1.0 = 0 dBFS).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        player: Option<&Player>,
        file_name: Option<&str>,
        vu_level: f32,
    ) -> bool {
        let mut open_clicked = false;

        ui.horizontal(|ui| {
            if ui.button("Open File").clicked() {
                open_clicked = true;
            }

            if let Some(p) = player {
                let is_playing = p.playing.load(Ordering::Relaxed);
                if ui
                    .button(if is_playing { "Pause" } else { "Play" })
                    .clicked()
                {
                    p.playing.store(!is_playing, Ordering::Relaxed);
                }
                if ui.button("Stop").clicked() {
                    p.playing.store(false, Ordering::Relaxed);
                    p.cursor.store(0, Ordering::Relaxed);
                    p.reset_requested.store(true, Ordering::Relaxed);
                }
                if let Some(name) = file_name {
                    ui.label(format!("File: {name}"));
                }
            } else {
                ui.add_enabled(false, egui::Button::new("Play"));
                ui.add_enabled(false, egui::Button::new("Stop"));
            }

            // Push VU meter to the far right
            let remaining = ui.available_width() - 16.0 - ui.spacing().item_spacing.x;
            if remaining > 0.0 {
                ui.add_space(remaining);
            }
            self.vu_meter.show(ui, vu_level);
        });

        // Seek slider row
        if let Some(p) = player {
            let total = p.sample_count;
            let sr = p.sample_rate as f64;
            let cur = p.cursor.load(Ordering::Relaxed).min(total);
            let mut frac = if total > 0 {
                cur as f32 / total as f32
            } else {
                0.0
            };

            let seek_response = ui
                .horizontal(|ui| {
                    ui.label(fmt_time(cur as f64 / sr));
                    let r = ui.add(egui::Slider::new(&mut frac, 0.0f32..=1.0f32).show_value(false));
                    ui.label(fmt_time(total as f64 / sr));
                    r
                })
                .inner;

            if seek_response.changed() {
                p.cursor
                    .store((frac as f64 * total as f64) as usize, Ordering::Relaxed);
                p.reset_requested.store(true, Ordering::Relaxed);
            }
        } else {
            let mut placeholder = 0.0f32;
            ui.horizontal(|ui| {
                ui.label("0:00");
                ui.add_enabled(
                    false,
                    egui::Slider::new(&mut placeholder, 0.0f32..=1.0f32).show_value(false),
                );
                ui.label("0:00");
            });
        }

        open_clicked
    }
}

fn fmt_time(secs: f64) -> String {
    let m = (secs / 60.0) as u32;
    let s = (secs % 60.0) as u32;
    format!("{m}:{s:02}")
}
