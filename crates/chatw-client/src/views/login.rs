use egui::{Align, Align2, Color32, CornerRadius, Layout, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

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

            // 1. Logo: Chat bubble icon in #2563EB (size 44px)
            let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), Sense::hover());
            ui.painter().rect_filled(
                logo_rect,
                CornerRadius::same(10),
                Color32::from_rgb(37, 99, 235), // Blue #2563EB
            );
            ui.painter().text(
                logo_rect.center(),
                Align2::CENTER_CENTER,
                "💬",
                egui::FontId::proportional(22.0),
                Color32::WHITE,
            );

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

                // Draw Google G white badge inside button
                let icon_center = egui::pos2(btn_rect.left() + 24.0, btn_rect.center().y);
                ui.painter().circle_filled(icon_center, 10.0, Color32::WHITE);
                ui.painter().text(
                    icon_center,
                    Align2::CENTER_CENTER,
                    "G",
                    egui::FontId::proportional(12.0),
                    Color32::from_rgb(66, 133, 244), // Google Blue #4285F4
                );

                // Button label
                ui.painter().text(
                    egui::pos2(btn_rect.center().x + 6.0, btn_rect.center().y),
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
                    egui::pos2(center_x - card_width / 2.0, line_y),
                    egui::pos2(center_x - text_half_w, line_y),
                ],
                Stroke::new(1.0_f32, line_color),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(center_x + text_half_w, line_y),
                    egui::pos2(center_x + card_width / 2.0, line_y),
                ],
                Stroke::new(1.0_f32, line_color),
            );

            ui.label(
                RichText::new("chế độ phát triển cục bộ")
                    .size(12.0)
                    .color(Color32::from_rgb(115, 115, 115)), // #737373
            );

            ui.add_space(14.0);

            // 6. Dev input matching web .auth-input
            let input = egui::TextEdit::singleline(&mut self.username_input)
                .hint_text("Tên tài khoản thử (ví dụ: alice)")
                .min_size(Vec2::new(card_width, 38.0));
            let input_resp = ui.add(input);

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
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(err)
                                .size(13.0)
                                .color(Color32::from_rgb(220, 38, 38)), // #DC2626
                        );
                    });
            }

            // 9. Server configuration toggle (clean and minimal)
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Máy chủ:")
                        .size(11.5)
                        .color(Color32::from_rgb(115, 115, 115)),
                );
                let current_label = if self.server_url_input.contains("127.0.0.1")
                    || self.server_url_input.contains("localhost")
                {
                    "Local (127.0.0.1:8082)"
                } else if self.server_url_input.contains("chat.kiyo.vn") {
                    "Cloud (api.chat.kiyo.vn)"
                } else {
                    "Tùy chỉnh"
                };
                if ui
                    .button(
                        RichText::new(current_label)
                            .size(11.5)
                            .color(Color32::from_rgb(37, 99, 235)),
                    )
                    .clicked()
                {
                    self.show_server_config = !self.show_server_config;
                }
            });

            if self.show_server_config {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let input = egui::TextEdit::singleline(&mut self.server_url_input)
                        .hint_text("http://127.0.0.1:8082")
                        .desired_width(220.0);
                    ui.add(input);
                    if ui.button(RichText::new("Lưu").size(11.0)).clicked() {
                        action = Some(LoginAction::ChangeServer(
                            self.server_url_input.trim().to_string(),
                        ));
                        self.show_server_config = false;
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Dùng Local: 8082").size(11.0)).clicked() {
                        self.server_url_input = "http://127.0.0.1:8082".to_string();
                        action = Some(LoginAction::ChangeServer(self.server_url_input.clone()));
                        self.show_server_config = false;
                    }
                    if ui.button(RichText::new("Dùng api.chat.kiyo.vn").size(11.0)).clicked() {
                        self.server_url_input = "https://api.chat.kiyo.vn".to_string();
                        action = Some(LoginAction::ChangeServer(self.server_url_input.clone()));
                        self.show_server_config = false;
                    }
                });
            }
        });

        action
    }
}
