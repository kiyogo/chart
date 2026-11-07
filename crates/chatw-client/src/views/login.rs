use egui::{Align2, Color32, RichText, Sense, Ui, Vec2};

pub struct LoginView {
    pub username_input: String,
    pub is_loading: bool,
    pub error_msg: Option<String>,
}

impl Default for LoginView {
    fn default() -> Self {
        Self {
            username_input: String::new(),
            is_loading: false,
            error_msg: None,
        }
    }
}

pub enum LoginAction {
    GoogleLogin,
    DevLogin(String),
}

impl LoginView {
    pub fn show(&mut self, ui: &mut Ui) -> Option<LoginAction> {
        let mut action = None;

        // Centered container
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.15);

            // Card box
            let card_width = 340.0;
            ui.set_max_width(card_width);

            // App Logo
            let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(54.0, 54.0), Sense::hover());
            ui.painter().rect_filled(logo_rect, 12.0, Color32::from_rgb(37, 99, 235));
            ui.painter().text(
                logo_rect.center(),
                Align2::CENTER_CENTER,
                "💬",
                egui::FontId::proportional(26.0),
                Color32::WHITE,
            );

            ui.add_space(12.0);
            ui.label(RichText::new("chatw").size(24.0).strong().color(Color32::WHITE));
            ui.label(
                RichText::new("Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE")
                    .size(13.0)
                    .color(Color32::from_rgb(161, 161, 170)),
            );

            ui.add_space(24.0);

            // Google Login Button
            let google_btn = egui::Button::new(
                RichText::new("Tiếp tục với Google")
                    .size(14.0)
                    .color(Color32::WHITE)
                    .strong(),
            )
            .fill(Color32::from_rgb(37, 99, 235))
            .min_size(Vec2::new(card_width, 42.0));

            if ui.add_enabled(!self.is_loading, google_btn).clicked() {
                action = Some(LoginAction::GoogleLogin);
            }

            ui.add_space(16.0);

            // Divider
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                let (line_rect1, _) = ui.allocate_exact_size(Vec2::new(80.0, 1.0), Sense::hover());
                ui.painter().rect_filled(line_rect1, 0.0, Color32::from_rgb(63, 63, 70));

                ui.label(
                    RichText::new("hoặc chế độ thử nghiệm")
                        .size(11.0)
                        .color(Color32::from_rgb(113, 113, 122)),
                );

                let (line_rect2, _) = ui.allocate_exact_size(Vec2::new(80.0, 1.0), Sense::hover());
                ui.painter().rect_filled(line_rect2, 0.0, Color32::from_rgb(63, 63, 70));
            });

            ui.add_space(16.0);

            // Dev Username Input
            ui.label(
                RichText::new("Tên tài khoản thử nghiệm:")
                    .size(12.0)
                    .color(Color32::from_rgb(212, 212, 216)),
            );
            ui.add_space(4.0);

            let input = egui::TextEdit::singleline(&mut self.username_input)
                .hint_text("Nhập tên (ví dụ: Alice, Bob)")
                .min_size(Vec2::new(card_width, 36.0));
            let input_resp = ui.add(input);

            ui.add_space(10.0);

            // Dev Login Button
            let dev_btn = egui::Button::new(
                RichText::new(if self.is_loading {
                    "Đang đăng nhập"
                } else {
                    "Đăng nhập nhanh (Dev / Demo)"
                })
                .size(13.0)
                .color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(39, 39, 42))
            .min_size(Vec2::new(card_width, 38.0));

            if (ui.add_enabled(!self.is_loading, dev_btn).clicked()
                || (input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.username_input.trim().is_empty()
            {
                action = Some(LoginAction::DevLogin(self.username_input.trim().to_string()));
            }

            // Error display
            if let Some(ref err) = self.error_msg {
                ui.add_space(12.0);
                ui.label(RichText::new(err).color(Color32::from_rgb(248, 113, 113)).size(12.0));
            }
        });

        action
    }
}
