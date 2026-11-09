use egui::emath::GuiRounding as _;
use egui::{
    Color32, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
    ViewportCommand, pos2, vec2,
};

pub fn font_regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn font_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

#[allow(dead_code)]
pub fn font_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

#[allow(dead_code)]
pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn font_mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

#[inline]
pub fn snap_pos(p: Pos2, ppp: f32) -> Pos2 {
    p.round_to_pixels(ppp)
}

#[inline]
pub fn snap_xy(x: f32, y: f32, ppp: f32) -> Pos2 {
    pos2(x.round_to_pixels(ppp), y.round_to_pixels(ppp))
}

/// Draws a connected polyline with round caps and round joins (`stroke-linecap="round"`, `stroke-linejoin="round"`)
pub fn draw_smooth_polyline(
    painter: &egui::Painter,
    points: &[Pos2],
    stroke: Stroke,
    closed: bool,
) {
    if points.is_empty() {
        return;
    }
    let join_r = (stroke.width * 0.45).max(0.6);
    for i in 0..points.len().saturating_sub(1) {
        painter.line_segment([points[i], points[i + 1]], stroke);
    }
    if closed && points.len() > 2 {
        painter.line_segment([points[points.len() - 1], points[0]], stroke);
    }
    for &p in points {
        painter.circle_filled(p, join_r, stroke.color);
    }
}

/// Appends points along a circular arc from `start_angle` to `end_angle` (in radians)
pub fn push_arc_points(
    out: &mut Vec<Pos2>,
    center: Pos2,
    radius: Vec2,
    start_angle: f32,
    end_angle: f32,
    steps: usize,
) {
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let a = start_angle + (end_angle - start_angle) * t;
        out.push(pos2(
            center.x + a.cos() * radius.x,
            center.y + a.sin() * radius.y,
        ));
    }
}

/// Draws the authentic web LogoIcon:
/// `<svg viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" /></svg>`
pub fn draw_svg_logo_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let r2 = vec2(2.0 * s.x, 2.0 * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    let mut pts = Vec::with_capacity(32);
    let pi = std::f32::consts::PI;

    // Top-right corner arc (center 19,5, angle -pi/2 to 0)
    push_arc_points(&mut pts, pt(19.0, 5.0), r2, -pi * 0.5, 0.0, 6);
    // Bottom-right corner arc (center 19,15, angle 0 to pi/2)
    push_arc_points(&mut pts, pt(19.0, 15.0), r2, 0.0, pi * 0.5, 6);
    // Bottom edge to tail
    pts.push(pt(7.0, 17.0));
    pts.push(pt(3.0, 21.0));
    // Left edge up to top-left corner arc (center 5,5, angle pi to 1.5*pi)
    push_arc_points(&mut pts, pt(5.0, 5.0), r2, pi, pi * 1.5, 6);

    draw_smooth_polyline(painter, &pts, stroke, true);
}

#[allow(dead_code)]
pub fn render_titlebar(ui: &mut Ui, is_maximized: bool) {
    let titlebar_height = 32.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), titlebar_height),
        Sense::hover(),
    );

    // Background of titlebar: Dark #18181B matching web .custom-titlebar
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 27));

    // Bottom border #27272A
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(39, 39, 42)),
    );

    // Left area: App logo & Title (.titlebar-left: padding-left 12px, gap 8px)
    let logo_color = Color32::from_rgb(59, 130, 246); // #3B82F6
    let title_color = Color32::from_rgb(161, 161, 170); // #A1A1AA

    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + 12.0 + 8.0, rect.center().y),
        vec2(16.0, 16.0),
    );
    draw_svg_logo_icon(ui.painter(), icon_rect, logo_color);

    let ppp = ui.pixels_per_point();
    ui.painter().text(
        snap_xy(icon_rect.right() + 8.0, rect.center().y, ppp),
        egui::Align2::LEFT_CENTER,
        "chatw – Secure Messaging",
        font_regular(12.0),
        title_color,
    );

    // Right area: 3 Window buttons (Minimize, Maximize, Close) — width 46px each
    let btn_width = 46.0;
    let controls_width = btn_width * 3.0;
    let controls_rect = Rect::from_min_size(
        pos2(rect.right() - controls_width, rect.top()),
        Vec2::new(controls_width, titlebar_height),
    );

    // Draggable area: entire titlebar except right controls
    let drag_rect =
        Rect::from_min_max(rect.min, pos2(rect.right() - controls_width, rect.bottom()));
    let drag_resp = ui.interact(
        drag_rect,
        ui.id().with("titlebar_drag"),
        Sense::click_and_drag(),
    );
    if drag_resp.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag_resp.double_clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    // Render Window control buttons at exact rects (no item_spacing shift)
    let min_rect = Rect::from_min_size(controls_rect.min, vec2(btn_width, titlebar_height));
    if window_control_button(ui, min_rect, "titlebar_min", WindowButtonKind::Minimize).clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }

    let max_rect = Rect::from_min_size(
        pos2(controls_rect.min.x + btn_width, controls_rect.min.y),
        vec2(btn_width, titlebar_height),
    );
    let max_kind = if is_maximized {
        WindowButtonKind::Restore
    } else {
        WindowButtonKind::Maximize
    };
    if window_control_button(ui, max_rect, "titlebar_max", max_kind).clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    let close_rect = Rect::from_min_size(
        pos2(controls_rect.min.x + btn_width * 2.0, controls_rect.min.y),
        vec2(btn_width, titlebar_height),
    );
    if window_control_button(ui, close_rect, "titlebar_close", WindowButtonKind::Close).clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
}

enum WindowButtonKind {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn window_control_button(
    ui: &mut Ui,
    rect: Rect,
    id_str: &str,
    kind: WindowButtonKind,
) -> Response {
    let resp = ui.interact(rect, ui.id().with(id_str), Sense::click());
    let is_close = matches!(kind, WindowButtonKind::Close);

    let bg_color = if resp.is_pointer_button_down_on() {
        if is_close {
            Color32::from_rgb(185, 28, 28)
        } else {
            Color32::from_rgb(63, 63, 70)
        }
    } else if resp.hovered() {
        if is_close {
            Color32::from_rgb(232, 17, 35) // Windows Close Red #E81123
        } else {
            Color32::from_rgb(39, 39, 42) // Highlight #27272A
        }
    } else {
        Color32::TRANSPARENT
    };

    ui.painter().rect_filled(rect, 0.0, bg_color);

    let icon_color = if resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(212, 212, 216) // #D4D4D8
    };

    let center = rect.center();

    match kind {
        WindowButtonKind::Minimize => {
            let bar_rect = Rect::from_center_size(center, vec2(10.0, 1.0));
            ui.painter().rect_filled(bar_rect, 0.0, icon_color);
        }
        WindowButtonKind::Maximize => {
            let sq_rect = Rect::from_center_size(center, vec2(10.0, 10.0));
            ui.painter().rect_stroke(
                sq_rect,
                0.0,
                Stroke::new(1.0_f32, icon_color),
                StrokeKind::Inside,
            );
        }
        WindowButtonKind::Restore => {
            let back_rect = Rect::from_min_size(center + vec2(-3.0, -5.0), vec2(8.0, 8.0));
            let front_rect = Rect::from_min_size(center + vec2(-5.0, -3.0), vec2(8.0, 8.0));
            ui.painter().rect_stroke(
                back_rect,
                0.0,
                Stroke::new(1.0_f32, icon_color),
                StrokeKind::Inside,
            );
            let fill_under = if resp.hovered() {
                Color32::from_rgb(39, 39, 42)
            } else {
                Color32::from_rgb(24, 24, 27)
            };
            ui.painter().rect_filled(front_rect, 0.0, fill_under);
            ui.painter().rect_stroke(
                front_rect,
                0.0,
                Stroke::new(1.0_f32, icon_color),
                StrokeKind::Inside,
            );
        }
        WindowButtonKind::Close => {
            let half = 4.5_f32;
            let stroke = Stroke::new(1.2_f32, icon_color);
            ui.painter().line_segment(
                [center + vec2(-half, -half), center + vec2(half, half)],
                stroke,
            );
            ui.painter().line_segment(
                [center + vec2(half, -half), center + vec2(-half, half)],
                stroke,
            );
        }
    }

    resp
}
