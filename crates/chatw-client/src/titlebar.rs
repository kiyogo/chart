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

pub fn render_titlebar(ui: &mut Ui, is_maximized: bool) {
    let titlebar_height = 34.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), titlebar_height),
        Sense::hover(),
    );

    // 1. Background: Clean White (#FFFFFF) with rounded top corners when not maximized
    let corner_top = if is_maximized { 0 } else { 8 };
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius {
            nw: corner_top,
            ne: corner_top,
            sw: 0,
            se: 0,
        },
        Color32::from_rgb(255, 255, 255),
    );

    // 3. Bottom border: 1px #E5E7EB
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
    );

    // 4. Left Brand Section: 16x16 SVG logo, chatw, •, Secure Messaging, E2EE badge
    let logo_color = Color32::from_rgb(37, 99, 235); // #2563EB
    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + 14.0 + 8.0, rect.center().y),
        vec2(16.0, 16.0),
    );
    draw_svg_logo_icon(ui.painter(), icon_rect, logo_color);

    let ppp = ui.pixels_per_point();
    let name_pos = snap_xy(icon_rect.right() + 8.0, rect.center().y, ppp);
    let name_galley = ui.painter().layout_no_wrap(
        "chatw".to_string(),
        font_medium(13.0),
        Color32::from_rgb(17, 24, 39), // #111827
    );
    let name_rect = egui::Align2::LEFT_CENTER.anchor_size(name_pos, name_galley.size());
    ui.painter()
        .galley(name_rect.min, name_galley, Color32::from_rgb(17, 24, 39));

    let dot_pos = snap_xy(name_rect.right() + 8.0, rect.center().y, ppp);
    let dot_galley = ui.painter().layout_no_wrap(
        "•".to_string(),
        font_regular(12.0),
        Color32::from_rgb(209, 213, 219), // #D1D5DB
    );
    let dot_rect = egui::Align2::LEFT_CENTER.anchor_size(dot_pos, dot_galley.size());
    ui.painter()
        .galley(dot_rect.min, dot_galley, Color32::from_rgb(209, 213, 219));

    let sub_pos = snap_xy(dot_rect.right() + 8.0, rect.center().y, ppp);
    let sub_galley = ui.painter().layout_no_wrap(
        "Secure Messaging".to_string(),
        font_regular(12.0),
        Color32::from_rgb(107, 114, 128), // #6B7280
    );
    let sub_rect = egui::Align2::LEFT_CENTER.anchor_size(sub_pos, sub_galley.size());
    ui.painter()
        .galley(sub_rect.min, sub_galley, Color32::from_rgb(107, 114, 128));

    // E2EE Pill Badge
    let badge_rect = Rect::from_center_size(
        pos2(sub_rect.right() + 10.0 + 24.0, rect.center().y),
        vec2(48.0, 18.0),
    );
    ui.painter().rect(
        badge_rect,
        egui::CornerRadius::same(4),
        Color32::from_rgb(240, 253, 244), // #F0FDF4
        Stroke::new(1.0_f32, Color32::from_rgb(187, 247, 208)), // #BBF7D0
        StrokeKind::Inside,
    );
    // Green dot
    ui.painter().circle_filled(
        pos2(badge_rect.left() + 7.0, badge_rect.center().y),
        2.5,
        Color32::from_rgb(22, 163, 74), // #16A34A
    );
    ui.painter().text(
        snap_xy(badge_rect.left() + 14.0, badge_rect.center().y, ppp),
        egui::Align2::LEFT_CENTER,
        "E2EE",
        font_medium(10.0),
        Color32::from_rgb(21, 128, 61), // #15803D
    );

    // 5. Draggable Region
    let btn_width = 46.0;
    let controls_width = btn_width * 3.0;
    let controls_rect = Rect::from_min_size(
        pos2(rect.right() - controls_width, rect.top()),
        Vec2::new(controls_width, titlebar_height),
    );

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

    // 6. Right Window Controls (Windows 11 exact style)
    let min_rect = Rect::from_min_size(controls_rect.min, vec2(btn_width, titlebar_height));
    if window_control_button(
        ui,
        min_rect,
        "titlebar_min",
        WindowButtonKind::Minimize,
        false,
    )
    .clicked()
    {
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
    if window_control_button(ui, max_rect, "titlebar_max", max_kind, false).clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    let close_rect = Rect::from_min_size(
        pos2(controls_rect.min.x + btn_width * 2.0, controls_rect.min.y),
        vec2(btn_width, titlebar_height),
    );
    // Close button has top-right corner rounded when not maximized
    if window_control_button(
        ui,
        close_rect,
        "titlebar_close",
        WindowButtonKind::Close,
        !is_maximized,
    )
    .clicked()
    {
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
    round_ne: bool,
) -> Response {
    let resp = ui.interact(rect, ui.id().with(id_str), Sense::click());
    let is_close = matches!(kind, WindowButtonKind::Close);

    let bg_color = if resp.is_pointer_button_down_on() {
        if is_close {
            Color32::from_rgb(196, 43, 28) // Windows Close Active #C42B1C
        } else {
            Color32::from_rgb(229, 231, 235) // Light Gray Active #E5E7EB
        }
    } else if resp.hovered() {
        if is_close {
            Color32::from_rgb(232, 17, 35) // Windows Close Hover #E81123
        } else {
            Color32::from_rgb(243, 244, 246) // Light Gray Hover #F3F4F6
        }
    } else {
        Color32::TRANSPARENT
    };

    let corner = if round_ne {
        egui::CornerRadius {
            nw: 0,
            ne: 8,
            sw: 0,
            se: 0,
        }
    } else {
        egui::CornerRadius::ZERO
    };

    ui.painter().rect_filled(rect, corner, bg_color);

    let icon_color = if resp.hovered() && is_close {
        Color32::WHITE
    } else if resp.hovered() {
        Color32::from_rgb(17, 24, 39) // #111827 Dark icon on hover
    } else {
        Color32::from_rgb(75, 85, 99) // #4B5563 Medium gray
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
                Color32::from_rgb(243, 244, 246)
            } else {
                Color32::WHITE
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
            let stroke = Stroke::new(1.15_f32, icon_color);
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

#[cfg(windows)]
pub fn apply_windows_dwm_effects() -> bool {
    #[repr(C)]
    struct Margins {
        cx_left: i32,
        cx_right: i32,
        cy_top: i32,
        cy_bottom: i32,
    }

    type Hwnd = *mut std::ffi::c_void;
    type Hresult = i32;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: Hwnd,
            dw_attribute: u32,
            pv_attribute: *const std::ffi::c_void,
            cb_attribute: u32,
        ) -> Hresult;

        fn DwmExtendFrameIntoClientArea(hwnd: Hwnd, p_mar_inset: *const Margins) -> Hresult;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetCurrentProcessId() -> u32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, lpdw_process_id: *mut u32) -> u32;
        fn EnumWindows(
            lp_enum_func: Option<unsafe extern "system" fn(Hwnd, isize) -> i32>,
            l_param: isize,
        ) -> i32;
        fn GetWindowLongW(hwnd: Hwnd, n_index: i32) -> i32;
        fn SetWindowLongW(hwnd: Hwnd, n_index: i32, new_long: i32) -> i32;
        fn SetWindowPos(
            hwnd: Hwnd,
            hwnd_insert_after: Hwnd,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
        fn GetClassLongPtrW(hwnd: Hwnd, n_index: i32) -> usize;
        fn SetClassLongPtrW(hwnd: Hwnd, n_index: i32, dw_new_long: usize) -> usize;
    }

    static mut FOUND: bool = false;

    unsafe extern "system" fn enum_proc(hwnd: Hwnd, target_pid: isize) -> i32 {
        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == target_pid as u32 {
                FOUND = true;

                // 1. Add CS_DROPSHADOW (0x00020000) to class style
                let old_class_style = GetClassLongPtrW(hwnd, -26);
                let _ = SetClassLongPtrW(hwnd, -26, old_class_style | 0x00020000);

                // 2. Add WS_THICKFRAME (0x00040000) so DWM enables rounded corners and dropshadow
                let cur_style = GetWindowLongW(hwnd, -16);
                let new_style = cur_style | 0x00040000;
                SetWindowLongW(hwnd, -16, new_style);

                // SWP_NOMOVE(2) | SWP_NOSIZE(1) | SWP_NOZORDER(4) | SWP_FRAMECHANGED(0x20)
                SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, 0x0027);

                // 3. DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2 (Windows 11 Rounded Corners)
                let corner_pref: u32 = 2; // DWMWCP_ROUND
                let res_corner = DwmSetWindowAttribute(
                    hwnd,
                    33,
                    &corner_pref as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<u32>() as u32,
                );

                // 4. DwmExtendFrameIntoClientArea to enable native DWM drop shadow blur on borderless window
                let margins = Margins {
                    cx_left: 1,
                    cx_right: 1,
                    cy_top: 1,
                    cy_bottom: 1,
                };
                let res_margins = DwmExtendFrameIntoClientArea(hwnd, &margins);

                // 5. DWMWA_USE_IMMERSIVE_DARK_MODE = 20 -> 0 (Light mode)
                let dark_mode: u32 = 0;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    20,
                    &dark_mode as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<u32>() as u32,
                );

                crate::log_app(&format!(
                    "apply_windows_dwm_effects: hwnd={:?}, cur_style={:#X}, res_corner={}, res_margins={}",
                    hwnd, cur_style, res_corner, res_margins
                ));
            }
        }
        1
    }

    unsafe {
        FOUND = false;
        let pid = GetCurrentProcessId();
        let _ = EnumWindows(Some(enum_proc), pid as isize);
        FOUND
    }
}

#[cfg(not(windows))]
pub fn apply_windows_dwm_effects() -> bool {
    true
}
