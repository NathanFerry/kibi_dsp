use egui::{Align2, Color32, Context, CornerRadius, FontId, Margin, Pos2, Sense, Shadow, Stroke, Vec2, Visuals};

pub const BACKGROUND: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x2e);
pub const SURFACE: Color32 = Color32::from_rgb(0x16, 0x21, 0x3e);
pub const SURFACE_HIGH: Color32 = Color32::from_rgb(0x0f, 0x34, 0x60);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0x4c, 0xc9, 0xf0);
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(0xf7, 0x7f, 0x00);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(0x06, 0xd6, 0xa0);
pub const ACCENT_PINK: Color32 = Color32::from_rgb(0xf7, 0x25, 0x85);
pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(0xff, 0xd6, 0x0a);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe2, 0xe8, 0xf0);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x94, 0xa3, 0xb8);
pub const SUCCESS: Color32 = Color32::from_rgb(0x06, 0xd6, 0xa0);
pub const DANGER: Color32 = Color32::from_rgb(0xef, 0x23, 0x3c);

pub fn apply_theme(ctx: &Context) {
    let mut visuals = Visuals::dark();

    visuals.dark_mode = true;
    visuals.panel_fill = SURFACE;
    visuals.window_fill = BACKGROUND;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = SURFACE_HIGH;

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = SURFACE_HIGH;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(0x4c, 0xc9, 0xf0, 102);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, ACCENT_BLUE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = ACCENT_BLUE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, BACKGROUND);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    visuals.widgets.open.corner_radius = CornerRadius::same(6);

    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(6);

    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0xf7, 0x25, 0x85, 153);

    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;

    ctx.set_visuals(visuals);
}

pub fn processor_color(name: &str) -> Color32 {
    let n = name.to_lowercase();
    if n.contains("filter")
        || n.contains("fir")
        || n.contains("butterworth")
        || n.contains("biquad")
        || n.contains("notch")
        || n.contains("bessel")
        || n.contains("chebyshev")
        || n.contains("cauer")
    {
        ACCENT_BLUE
    } else if n.contains("delay")
        || n.contains("chorus")
        || n.contains("tremolo")
        || n.contains("reverb")
    {
        ACCENT_ORANGE
    } else if n.contains("gain") || n.contains("volume") || n.contains("clip") {
        ACCENT_GREEN
    } else {
        ACCENT_PINK
    }
}

pub fn draw_panel_header(
    ui: &mut egui::Ui,
    title: &str,
    dot_color: Color32,
    accent_color: Color32,
) {
    let available_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_w, 24.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, SURFACE_HIGH);

    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(accent_color.r(), accent_color.g(), accent_color.b(), 102),
        ),
    );

    let dot_cx = rect.left() + 11.0;
    let dot_cy = rect.center().y;
    painter.circle_filled(Pos2::new(dot_cx, dot_cy), 3.0, dot_color);

    painter.text(
        Pos2::new(dot_cx + 8.0, dot_cy),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(11.0),
        TEXT_MUTED,
    );
}

pub fn section_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
}
