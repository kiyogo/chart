use egui::{
    pos2, vec2, Color32, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
    ViewportCommand,
};

/// Draws the authentic web LogoIcon: hollow chat bubble outline with a small tail
pub fn draw_svg_logo_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);

    // SVG path: M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z
    let points = [
        pt(21.0, 5.0),
        pt(21.0, 15.0),
        pt(19.0, 17.0),
        pt(7.0, 17.0),
        pt(3.0, 21.0),
        pt(3.0, 5.0),
        pt(5.0, 3.0),
        pt(19.0, 3.0),
        pt(21.0, 5.0),
    ];

    for i in 0..points.len() - 1 {
        painter.line_segment([points[i], points[i + 1]], stroke);
    }
}

pub fn render_titlebar(ui: &mut Ui, is_maximized: bool) {
    let titlebar_height = 32.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), titlebar_height),
        Sense::hover(),
    );

    // Background of titlebar: Dark #18181B matching web custom titlebar
    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 27));

    // Bottom border #27272A
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(39, 39, 42)),
    );

    // Left area: App logo & Title
    let logo_color = Color32::from_rgb(59, 130, 246); // Blue #3B82F6
    let title_color = Color32::from_rgb(161, 161, 170); // Light gray #A1A1AA

    let mut left_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(rect.min, Vec2::new(260.0, titlebar_height)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left_ui.add_space(12.0);

    // Draw authentic web LogoIcon (outline SVG chat bubble)
    let (icon_rect, _) = left_ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
    draw_svg_logo_icon(left_ui.painter(), icon_rect, logo_color);

    left_ui.add_space(8.0);
    left_ui.colored_label(
        title_color,
        egui::RichText::new("chatw – Secure Messaging")
            .size(12.0)
            .strong(),
    );

    // Right area: 3 Window buttons (Minimize, Maximize, Close)
    let btn_width = 46.0;
    let controls_width = btn_width * 3.0;
    let controls_rect = Rect::from_min_size(
        pos2(rect.right() - controls_width, rect.top()),
        Vec2::new(controls_width, titlebar_height),
    );

    // Center draggable area: between left title and right controls
    let drag_rect = Rect::from_min_max(
        pos2(rect.left() + 260.0, rect.top()),
        pos2(rect.right() - controls_width, rect.bottom()),
    );
    let drag_resp = ui.interact(drag_rect, ui.id().with("titlebar_drag"), Sense::click_and_drag());
    if drag_resp.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag_resp.double_clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    // Render Window control buttons
    let mut controls_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(controls_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );

    // 1. Minimize button (crisp vector 10x1 bar)
    if window_control_button(&mut controls_ui, WindowButtonKind::Minimize, btn_width, titlebar_height).clicked() {
        controls_ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }

    // 2. Maximize / Restore button (crisp vector 10x10 square or double square)
    let max_kind = if is_maximized {
        WindowButtonKind::Restore
    } else {
        WindowButtonKind::Maximize
    };
    if window_control_button(&mut controls_ui, max_kind, btn_width, titlebar_height).clicked() {
        controls_ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    // 3. Close button (crisp vector X with red hover)
    if window_control_button(&mut controls_ui, WindowButtonKind::Close, btn_width, titlebar_height).clicked() {
        controls_ui.ctx().send_viewport_cmd(ViewportCommand::Close);
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
    kind: WindowButtonKind,
    width: f32,
    height: f32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
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

    let icon_color = if resp.hovered() && is_close {
        Color32::WHITE
    } else if resp.hovered() {
        Color32::from_rgb(244, 244, 245)
    } else {
        Color32::from_rgb(212, 212, 216)
    };

    let center = rect.center();

    match kind {
        WindowButtonKind::Minimize => {
            // 10x1 horizontal bar matching web <svg width="10" height="1"><path d="M0 0h10v1H0z" /></svg>
            let bar_rect = Rect::from_center_size(center + vec2(0.0, 1.0), vec2(10.0, 1.0));
            ui.painter().rect_filled(bar_rect, 0.0, icon_color);
        }
        WindowButtonKind::Maximize => {
            // 10x10 hollow square matching web <svg width="10" height="10"><path d="M0.5 0.5h9v9h-9z" /></svg>
            let sq_rect = Rect::from_center_size(center, vec2(10.0, 10.0));
            ui.painter().rect_stroke(
                sq_rect,
                0.0,
                Stroke::new(1.0_f32, icon_color),
                StrokeKind::Inside,
            );
        }
        WindowButtonKind::Restore => {
            // 2 overlapping squares matching web maximize restore state
            let back_rect = Rect::from_min_size(center + vec2(-3.0, -5.0), vec2(8.0, 8.0));
            let front_rect = Rect::from_min_size(center + vec2(-5.0, -3.0), vec2(8.0, 8.0));
            ui.painter().rect_stroke(
                back_rect,
                0.0,
                Stroke::new(1.0_f32, icon_color),
                StrokeKind::Inside,
            );
            ui.painter().rect_filled(front_rect, 0.0, bg_color);
            ui.painter().rect_stroke(
                front_rect,
                0.0,
                Stroke::new(1.0_f32, icon_color),
                StrokeKind::Inside,
            );
        }
        WindowButtonKind::Close => {
            // 10x10 crisp vector X
            let half = 4.5_f32;
            let stroke = Stroke::new(1.1_f32, icon_color);
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
