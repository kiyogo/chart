use egui::{Color32, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2, ViewportCommand};

pub fn render_titlebar(ui: &mut Ui, is_maximized: bool) {
    let titlebar_height = 32.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), titlebar_height),
        Sense::hover(),
    );

    // Background of titlebar: Dark #18181B
    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 27));

    // Bottom border #27272A
    ui.painter().line_segment(
        [
            rect.left_bottom(),
            rect.right_bottom(),
        ],
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

    // Draw little chat bubble icon
    let (icon_rect, _) = left_ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
    left_ui.painter().rect_filled(icon_rect, 4.0, logo_color);
    left_ui.painter().text(
        icon_rect.center(),
        egui::Align2::CENTER_CENTER,
        "💬",
        egui::FontId::proportional(11.0),
        Color32::WHITE,
    );

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
        egui::pos2(rect.right() - controls_width, rect.top()),
        Vec2::new(controls_width, titlebar_height),
    );

    // Center draggable area: between left title and right controls
    let drag_rect = Rect::from_min_max(
        egui::pos2(rect.left() + 260.0, rect.top()),
        egui::pos2(rect.right() - controls_width, rect.bottom()),
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

    // 1. Minimize button
    if custom_button(&mut controls_ui, "—", btn_width, titlebar_height, false).clicked() {
        controls_ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }

    // 2. Maximize / Restore button
    let max_icon = if is_maximized { "❐" } else { "□" };
    if custom_button(&mut controls_ui, max_icon, btn_width, titlebar_height, false).clicked() {
        controls_ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    // 3. Close button (with Red hover)
    if custom_button(&mut controls_ui, "✕", btn_width, titlebar_height, true).clicked() {
        controls_ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
}

fn custom_button(
    ui: &mut Ui,
    text: &str,
    width: f32,
    height: f32,
    is_close: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

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

    let text_color = if resp.hovered() && is_close {
        Color32::WHITE
    } else if resp.hovered() {
        Color32::from_rgb(244, 244, 245)
    } else {
        Color32::from_rgb(212, 212, 216)
    };

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        text_color,
    );

    resp
}
