use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Pos2, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use crate::titlebar::draw_svg_logo_icon;

pub struct LoginView {
    pub username_input: String,
    pub server_url_input: String,
    pub show_server_config: bool,
    pub is_loading: bool,
    pub is_google_pending: bool,
    pub error_msg: Option<String>,
}

impl Default for LoginView {
    fn default() -> Self {
        Self {
            username_input: String::new(),
            server_url_input: "http://127.0.0.1:8082".to_string(),
            show_server_config: false,
            is_loading: false,
            is_google_pending: false,
            error_msg: None,
        }
    }
}

pub enum LoginAction {
    GoogleLogin,
    DevLogin(String),
    ChangeServer(String),
    CancelGoogleLogin,
}

/// Draws authentic Google 4-color "G" logo
fn draw_svg_google_g_icon(painter: &egui::Painter, center: Pos2, radius: f32) {
    let stroke_w = (radius * 0.45).max(2.8);
    let r = radius * 0.78;

    // 1. Red top arc (225° to 315°)
    let c_red = Color32::from_rgb(234, 67, 53); // #EA4335
    let stroke_red = Stroke::new(stroke_w, c_red);
    let steps = 6;
    for i in 0..steps {
        let a1 = std::f32::consts::PI * (1.25 + 0.5 * (i as f32 / steps as f32));
        let a2 = std::f32::consts::PI * (1.25 + 0.5 * ((i + 1) as f32 / steps as f32));
        let p1 = center + vec2(a1.cos() * r, a1.sin() * r);
        let p2 = center + vec2(a2.cos() * r, a2.sin() * r);
        painter.line_segment([p1, p2], stroke_red);
    }

    // 2. Yellow left arc (135° to 225°)
    let c_yellow = Color32::from_rgb(251, 188, 5); // #FBBC05
    let stroke_yellow = Stroke::new(stroke_w, c_yellow);
    for i in 0..steps {
        let a1 = std::f32::consts::PI * (0.75 + 0.5 * (i as f32 / steps as f32));
        let a2 = std::f32::consts::PI * (0.75 + 0.5 * ((i + 1) as f32 / steps as f32));
        let p1 = center + vec2(a1.cos() * r, a1.sin() * r);
        let p2 = center + vec2(a2.cos() * r, a2.sin() * r);
        painter.line_segment([p1, p2], stroke_yellow);
    }

    // 3. Green bottom arc (45° to 135°)
    let c_green = Color32::from_rgb(52, 168, 83); // #34A853
    let stroke_green = Stroke::new(stroke_w, c_green);
    for i in 0..steps {
        let a1 = std::f32::consts::PI * (0.25 + 0.5 * (i as f32 / steps as f32));
        let a2 = std::f32::consts::PI * (0.25 + 0.5 * ((i + 1) as f32 / steps as f32));
        let p1 = center + vec2(a1.cos() * r, a1.sin() * r);
        let p2 = center + vec2(a2.cos() * r, a2.sin() * r);
        painter.line_segment([p1, p2], stroke_green);
    }

    // 4. Blue right arc (-45° to 45°) & crossbar
    let c_blue = Color32::from_rgb(66, 133, 244); // #4285F4
    let stroke_blue = Stroke::new(stroke_w, c_blue);
    for i in 0..(steps / 2) {
        let a1 = std::f32::consts::PI * (0.0 + 0.25 * (i as f32 / (steps / 2) as f32));
        let a2 = std::f32::consts::PI * (0.0 + 0.25 * ((i + 1) as f32 / (steps / 2) as f32));
        let p1 = center + vec2(a1.cos() * r, a1.sin() * r);
        let p2 = center + vec2(a2.cos() * r, a2.sin() * r);
        painter.line_segment([p1, p2], stroke_blue);
    }
    // Blue horizontal bar
    painter.line_segment([center - vec2(0.5, 0.0), center + vec2(r, 0.0)], stroke_blue);
}

impl LoginView {
    pub fn show(&mut self, ui: &mut Ui) -> Option<LoginAction> {
        let mut action = None;

        // Container: White background (#FFFFFF)
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(255, 255, 255));

        // Centered layout matching web .auth-container
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.12);

            // Card box matching web .auth-box (max-width: 320px)
            let card_width = 320.0;
            ui.set_max_width(card_width);

            // 1. Logo: Outline SVG chat bubble icon (44px) in #2563EB matching web LogoIcon
            let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), Sense::hover());
            draw_svg_logo_icon(ui.painter(), logo_rect, Color32::from_rgb(37, 99, 235));

            ui.add_space(14.0);

            // 2. Title: chatw (20px, font-weight 600, color #171717)
            ui.label(
                RichText::new("chatw")
                    .size(20.0)
                    .strong()
                    .color(Color32::from_rgb(23, 23, 23)),
            );

            // 3. Subtitle: Ứng dụng nhắn tin bảo mật E2EE (13px, color #737373)
            ui.label(
                RichText::new("Ứng dụng nhắn tin bảo mật E2EE")
                    .size(13.0)
                    .color(Color32::from_rgb(115, 115, 115)),
            );

            ui.add_space(20.0);

            // 4. Google Login Button / Pending state
            if self.is_google_pending {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(254, 243, 199)) // Amber-100
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(253, 230, 138)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Đang chờ xác thực Google...")
                                    .size(12.0)
                                    .color(Color32::from_rgb(180, 83, 9)), // Amber-700
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .button(
                                        RichText::new("Hủy")
                                            .size(11.0)
                                            .color(Color32::from_rgb(220, 38, 38)),
                                    )
                                    .clicked()
                                {
                                    self.is_google_pending = false;
                                    self.is_loading = false;
                                    action = Some(LoginAction::CancelGoogleLogin);
                                }
                            });
                        });
                    });
            } else {
                let (btn_rect, btn_resp) = ui.allocate_exact_size(
                    Vec2::new(card_width, 44.0),
                    if self.is_loading { Sense::hover() } else { Sense::click() },
                );

                let btn_bg = if btn_resp.is_pointer_button_down_on() {
                    Color32::from_rgb(29, 78, 216) // Blue 700 #1D4ED8
                } else if btn_resp.hovered() {
                    Color32::from_rgb(30, 64, 175) // Blue 800
                } else {
                    Color32::from_rgb(37, 99, 235) // Blue 600 #2563EB
                };

                ui.painter().rect_filled(btn_rect, CornerRadius::same(8), btn_bg);

                // Draw authentic Google 4-color G badge on white circle
                let badge_center = pos2(btn_rect.left() + 24.0, btn_rect.center().y);
                ui.painter().circle_filled(badge_center, 11.0, Color32::WHITE);
                draw_svg_google_g_icon(ui.painter(), badge_center, 7.5);

                // Button label matching web .auth-btn-primary
                ui.painter().text(
                    pos2(btn_rect.center().x + 6.0, btn_rect.center().y),
                    Align2::CENTER_CENTER,
                    "Tiếp tục với Google",
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );

                if btn_resp.clicked() && !self.is_loading {
                    action = Some(LoginAction::GoogleLogin);
                }
            }

            ui.add_space(18.0);

            // 5. Divider: "chế độ phát triển cục bộ" with border #E5E7EB
            let line_y = ui.cursor().top() + 8.0;
            let center_x = ui.cursor().center().x;
            let text_half_w = 78.0;
            let line_color = Color32::from_rgb(229, 231, 235); // #E5E7EB

            ui.painter().line_segment(
                [
                    pos2(center_x - card_width / 2.0, line_y),
                    pos2(center_x - text_half_w, line_y),
                ],
                Stroke::new(1.0_f32, line_color),
            );
            ui.painter().line_segment(
                [
                    pos2(center_x + text_half_w, line_y),
                    pos2(center_x + card_width / 2.0, line_y),
                ],
                Stroke::new(1.0_f32, line_color),
            );

            ui.label(
                RichText::new("chế độ phát triển cục bộ")
                    .size(12.0)
                    .color(Color32::from_rgb(115, 115, 115)), // #737373
            );

            ui.add_space(14.0);

            // 6. Dev input matching web .auth-input: 1px border #E5E7EB, 8px radius, white background
            let (input_box_rect, _) = ui.allocate_exact_size(Vec2::new(card_width, 40.0), Sense::hover());
            ui.painter().rect_filled(input_box_rect, CornerRadius::same(8), Color32::WHITE);
            ui.painter().rect_stroke(
                input_box_rect,
                CornerRadius::same(8),
                Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)), // #E5E7EB
                StrokeKind::Inside,
            );

            let mut input_child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(input_box_rect.shrink2(vec2(12.0, 4.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let input_edit = egui::TextEdit::singleline(&mut self.username_input)
                .hint_text("Tên tài khoản thử (ví dụ: alice)")
                .frame(false)
                .desired_width(card_width - 24.0);
            let input_resp = input_child.add(input_edit);

            ui.add_space(10.0);

            // 7. Dev login button matching web .auth-btn-secondary
            let (dev_rect, dev_resp) = ui.allocate_exact_size(
                Vec2::new(card_width, 38.0),
                if self.is_loading { Sense::hover() } else { Sense::click() },
            );

            let dev_bg = if dev_resp.hovered() {
                Color32::from_rgb(236, 238, 242) // #ECEEF2
            } else {
                Color32::from_rgb(247, 248, 250) // #F7F8FA
            };

            ui.painter().rect_filled(dev_rect, CornerRadius::same(8), dev_bg);
            ui.painter().rect_stroke(
                dev_rect,
                CornerRadius::same(8),
                Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)), // #E5E7EB
                StrokeKind::Inside,
            );

            let dev_text = if self.is_loading && !self.is_google_pending {
                "Đang đăng nhập"
            } else {
                "Đăng nhập nhanh"
            };

            ui.painter().text(
                dev_rect.center(),
                Align2::CENTER_CENTER,
                dev_text,
                egui::FontId::proportional(13.5),
                Color32::from_rgb(23, 23, 23), // #171717
            );

            let should_dev_login = (dev_resp.clicked()
                || (input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.username_input.trim().is_empty();

            if should_dev_login && !self.is_loading {
                action = Some(LoginAction::DevLogin(self.username_input.trim().to_string()));
            }

            // 8. Error display matching web .auth-error
            if let Some(ref err) = self.error_msg {
                ui.add_space(14.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(254, 242, 242)) // #FEF2F2
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(254, 226, 226))) // #FEE2E2
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(card_width);
                        ui.label(
                            RichText::new(err)
                                .size(13.0)
                                .color(Color32::from_rgb(220, 38, 38)),
                        );
                    });
            }

            ui.add_space(24.0);

            // 9. Server configuration expandable
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Máy chủ:")
                        .size(12.0)
                        .color(Color32::from_rgb(115, 115, 115)),
                );

                let is_default = self.server_url_input == "http://127.0.0.1:8082";
                let summary_text = if is_default {
                    "Local (127.0.0.1:8082)"
                } else {
                    &self.server_url_input
                };

                let config_btn = ui.button(
                    RichText::new(summary_text)
                        .size(11.5)
                        .color(Color32::from_rgb(37, 99, 235)),
                );
                if config_btn.clicked() {
                    self.show_server_config = !self.show_server_config;
                }
            });

            if self.show_server_config {
                ui.add_space(8.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(247, 248, 250))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(card_width);
                        ui.label(
                            RichText::new("Địa chỉ Backend API:")
                                .size(12.0)
                                .color(Color32::from_rgb(115, 115, 115)),
                        );
                        ui.add_space(4.0);
                        let inp = ui.add(
                            egui::TextEdit::singleline(&mut self.server_url_input)
                                .desired_width(card_width - 24.0),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("Lưu & Kết nối").size(12.0)).clicked()
                                || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                action = Some(LoginAction::ChangeServer(
                                    self.server_url_input.trim().to_string(),
                                ));
                                self.show_server_config = false;
                            }
                            if ui.button(RichText::new("Mặc định").size(12.0)).clicked() {
                                self.server_url_input = "http://127.0.0.1:8082".to_string();
                                action = Some(LoginAction::ChangeServer(
                                    self.server_url_input.clone(),
                                ));
                                self.show_server_config = false;
                            }
                        });
                    });
            }
        });

        action
    }
}
