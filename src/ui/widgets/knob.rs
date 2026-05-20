use egui::{Color32, Key, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

use crate::ui::theme::{SURFACE_HIGH, TEXT_MUTED, TEXT_PRIMARY};

const KNOB_RADIUS: f32 = 20.0;
const WIDGET_WIDTH: f32 = 64.0;
const WIDGET_HEIGHT: f32 = KNOB_RADIUS * 2.0 + 28.0;

const START_ANGLE: f32 = 135.0 * std::f32::consts::PI / 180.0;
const SWEEP: f32 = 270.0 * std::f32::consts::PI / 180.0;

#[derive(Clone, Default)]
struct KnobState {
    editing: bool,
    edit_buffer: String,
    just_opened: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn knob(
    ui: &mut Ui,
    id: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    label: &str,
    unit: &str,
    color: Color32,
) -> Response {
    let state_id = egui::Id::new(id);
    let mut state: KnobState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();

    ui.push_id(id, |ui| {
        let sense = if state.editing {
            Sense::hover()
        } else {
            Sense::click_and_drag()
        };
        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(WIDGET_WIDTH, WIDGET_HEIGHT), sense);

        if !state.editing {
            if response.double_clicked() && ui.input(|i| i.modifiers.ctrl) {
                *value = default;
                response.mark_changed();
            } else if response.double_clicked() {
                state.editing = true;
                state.edit_buffer = fmt(*value);
                state.just_opened = true;
            } else if response.dragged() {
                let fine = ui.input(|i| i.modifiers.ctrl);
                let scale = if fine { 0.0005 } else { 0.005 };
                let delta = -response.drag_delta().y * scale * (max - min);
                let new_val = (*value + delta).clamp(min, max);
                if (new_val - *value).abs() > f32::EPSILON {
                    *value = new_val;
                    response.mark_changed();
                }
            }
        }

        if ui.is_rect_visible(rect) {
            if state.editing {
                let te_rect = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.top() + KNOB_RADIUS),
                    Vec2::new(WIDGET_WIDTH - 4.0, 22.0),
                );
                let te_response = ui.put(
                    te_rect,
                    egui::TextEdit::singleline(&mut state.edit_buffer)
                        .desired_width(WIDGET_WIDTH - 4.0)
                        .font(egui::FontId::proportional(11.0)),
                );

                if state.just_opened {
                    te_response.request_focus();
                    state.just_opened = false;
                }

                let enter = ui.input(|i| i.key_pressed(Key::Enter));
                let escape = ui.input(|i| i.key_pressed(Key::Escape));

                if escape {
                    state.editing = false;
                } else if enter || te_response.lost_focus() {
                    if let Ok(parsed) = state.edit_buffer.parse::<f32>() {
                        let new_val = parsed.clamp(min, max);
                        if (new_val - *value).abs() > f32::EPSILON {
                            *value = new_val;
                            response.mark_changed();
                        }
                    }
                    state.editing = false;
                }

                let font = egui::FontId::proportional(10.0);
                ui.painter().text(
                    Pos2::new(rect.center().x, rect.top() + KNOB_RADIUS * 2.0 + 3.0),
                    egui::Align2::CENTER_TOP,
                    label,
                    font,
                    TEXT_MUTED,
                );
            } else {
                let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
                let center = Pos2::new(rect.center().x, rect.top() + KNOB_RADIUS);
                render(ui, center, t, color, label, *value, unit);
            }
        }

        let was_editing = state.editing;
        ui.data_mut(|d| d.insert_temp(state_id, state));

        if was_editing {
            response
        } else {
            response.on_hover_text(format!("{label}: {} {unit}", fmt(*value)))
        }
    })
    .inner
}

fn render(ui: &mut Ui, center: Pos2, t: f32, color: Color32, label: &str, value: f32, unit: &str) {
    let painter = ui.painter();
    let arc_r = KNOB_RADIUS - 5.0;

    painter.circle_filled(center, KNOB_RADIUS, SURFACE_HIGH);
    painter.circle_stroke(center, KNOB_RADIUS, Stroke::new(1.5, TEXT_MUTED));

    let track_color =
        Color32::from_rgba_unmultiplied(TEXT_MUTED.r(), TEXT_MUTED.g(), TEXT_MUTED.b(), 70);
    painter.add(arc(
        center,
        arc_r,
        START_ANGLE,
        START_ANGLE + SWEEP,
        60,
        Stroke::new(3.0, track_color),
    ));

    if t > 0.001 {
        let end_angle = START_ANGLE + SWEEP * t;
        let n = ((60.0 * t) as usize).max(2);
        painter.add(arc(
            center,
            arc_r,
            START_ANGLE,
            end_angle,
            n,
            Stroke::new(3.0, color),
        ));
    }

    let angle = START_ANGLE + SWEEP * t;
    let dir = Vec2::new(angle.cos(), angle.sin());
    painter.line_segment(
        [center + 4.0 * dir, center + (KNOB_RADIUS - 5.0) * dir],
        Stroke::new(2.0, TEXT_PRIMARY),
    );

    let font = egui::FontId::proportional(10.0);
    painter.text(
        Pos2::new(center.x, center.y + KNOB_RADIUS + 3.0),
        egui::Align2::CENTER_TOP,
        label,
        font.clone(),
        TEXT_MUTED,
    );
    painter.text(
        Pos2::new(center.x, center.y + KNOB_RADIUS + 15.0),
        egui::Align2::CENTER_TOP,
        format!("{} {unit}", fmt(value)),
        font,
        TEXT_PRIMARY,
    );
}

fn arc(center: Pos2, radius: f32, start: f32, end: f32, n: usize, stroke: Stroke) -> Shape {
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = start + (end - start) * i as f32 / n as f32;
            Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
        })
        .collect();
    Shape::line(pts, stroke)
}

fn fmt(v: f32) -> String {
    if v.abs() >= 1000.0 {
        format!("{:.0}", v)
    } else if v.abs() >= 100.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}
