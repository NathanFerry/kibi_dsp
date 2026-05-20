use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};

use crate::ui::theme::{ACCENT_GREEN, ACCENT_YELLOW, DANGER};

const METER_WIDTH: f32 = 16.0;
const METER_HEIGHT: f32 = 36.0;
const NUM_SEGMENTS: usize = 20;
/// Gap between adjacent segments in pixels.
const GAP: f32 = 1.0;
/// Seconds before peak hold starts decaying.
const PEAK_HOLD_SECS: f64 = 2.0;
/// dB floor below which all segments are dark.
const DB_FLOOR: f32 = -40.0;

pub struct VuMeter {
    peak_db: f32,
    peak_set_at: f64,
}

impl Default for VuMeter {
    fn default() -> Self {
        Self {
            peak_db: DB_FLOOR,
            peak_set_at: 0.0,
        }
    }
}

impl VuMeter {
    /// Draw the meter. `level` is a linear amplitude in `[0.0, 1.0]` (1.0 = 0 dBFS).
    pub fn show(&mut self, ui: &mut Ui, level: f32) {
        let now = ui.input(|i| i.time);

        let level_db = if level > 1e-5 {
            20.0 * level.log10()
        } else {
            -100.0
        };
        let level_db = level_db.clamp(DB_FLOOR, 0.0);

        // Peak hold: refresh if new peak; decay after hold period
        if level_db >= self.peak_db {
            self.peak_db = level_db;
            self.peak_set_at = now;
        } else if now - self.peak_set_at > PEAK_HOLD_SECS {
            self.peak_db = (self.peak_db - 1.0).max(DB_FLOOR);
        }

        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(METER_WIDTH, METER_HEIGHT), egui::Sense::hover());

        if !ui.is_rect_visible(rect) {
            return;
        }

        let db_range = -DB_FLOOR; // 40 dB total
        let seg_h = (METER_HEIGHT - (NUM_SEGMENTS - 1) as f32 * GAP) / NUM_SEGMENTS as f32;
        let painter = ui.painter();

        for i in 0..NUM_SEGMENTS {
            // Each segment covers 2 dB; segment 0 = bottom = most negative dB
            let seg_floor_db = DB_FLOOR + i as f32 * (db_range / NUM_SEGMENTS as f32);
            let lit = level_db >= seg_floor_db;

            let active_color = segment_color(i);
            let dim = Color32::from_rgba_unmultiplied(
                active_color.r(),
                active_color.g(),
                active_color.b(),
                35,
            );

            // Segments are drawn bottom-up: i=0 is at the bottom of rect
            let y_bottom = rect.bottom() - i as f32 * (seg_h + GAP);
            let seg_rect = Rect::from_min_size(
                Pos2::new(rect.left(), y_bottom - seg_h),
                Vec2::new(METER_WIDTH, seg_h),
            );
            painter.rect_filled(seg_rect, 0.0, if lit { active_color } else { dim });
        }

        // Peak hold line
        if self.peak_db > DB_FLOOR {
            let peak_frac = (self.peak_db - DB_FLOOR) / db_range;
            let peak_y = rect.bottom() - peak_frac * METER_HEIGHT;
            painter.line_segment(
                [
                    Pos2::new(rect.left(), peak_y),
                    Pos2::new(rect.right(), peak_y),
                ],
                Stroke::new(2.0, Color32::WHITE),
            );
        }
    }
}

fn segment_color(i: usize) -> Color32 {
    if i < 14 {
        ACCENT_GREEN
    } else if i < 18 {
        ACCENT_YELLOW
    } else {
        DANGER
    }
}
