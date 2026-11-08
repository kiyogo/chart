use egui::{Align2, Color32, CornerRadius, RichText, Sense, Ui, Vec2};

pub struct SetNameView {
    pub name_input: String,
    pub is_loading: bool,
    pub error_msg: Option<String>,
}

impl Default for SetNameView {
    fn default() -> Self {
        Self {
            name_input: String::new(),
            is_loading: false,
            error_msg: None,
        }
    }
}

impl SetNameView {
    pub fn show(&mut self, ui: &mut Ui) -> Option<String> {
        let mut submitted_name = None;

        // Container: White background (#FFFFFF)
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(255, 255, 255));

        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.15);

            // Card box (max-width: 320px) matching web .auth-box
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

            ui.add_space(16.0);

            // 2. Title: Đặt tên của bạn (20px, font-weight 600, color #171717)
            ui.label(
                RichText::new("Đặt tên của bạn")
                    .size(20.0)
                    .strong()
                    .color(Color32::from_rgb(23, 23, 23)),
            );

            ui.add_space(20.0);

            // 3. Input matching web .auth-input
            let input = egui::TextEdit::singleline(&mut self.name_input)
                .hint_text("Nhập tên người dùng")
                .min_size(Vec2::new(card_width, 40.0));
            let input_resp = ui.add(input);

            // 4. Error message if any
            if let Some(ref err) = self.error_msg {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(err)
                        .size(13.0)
                        .color(Color32::from_rgb(220, 38, 38)), // #DC2626
                );
            }

            ui.add_space(16.0);

            // 5. Submit Button matching web .auth-btn-primary
            let (btn_rect, btn_resp) = ui.allocate_exact_size(
                Vec2::new(card_width, 44.0),
                if self.is_loading { Sense::hover() } else { Sense::click() },
            );

            let btn_bg = if btn_resp.is_pointer_button_down_on() {
                Color32::from_rgb(29, 78, 216) // Blue 700 #1D4ED8
            } else if btn_resp.hovered() {
                Color32::from_rgb(30, 64, 175)
            } else {
                Color32::from_rgb(37, 99, 235) // Blue 600 #2563EB
            };

            ui.painter().rect_filled(btn_rect, CornerRadius::same(8), btn_bg);

            let btn_text = if self.is_loading { "Đang lưu" } else { "Tiếp tục" };
            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                btn_text,
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );

            let trimmed = self.name_input.trim();
            let should_submit = (btn_resp.clicked()
                || (input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && trimmed.len() >= 2
                && trimmed.len() <= 32;

            if should_submit && !self.is_loading {
                submitted_name = Some(trimmed.to_string());
            }
        });

        submitted_name
    }
}
