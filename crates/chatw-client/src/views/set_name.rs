use egui::{Align2, Color32, RichText, Sense, Ui, Vec2};

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

        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.2);

            let card_width = 340.0;
            ui.set_max_width(card_width);

            // Icon
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), Sense::hover());
            ui.painter().rect_filled(icon_rect, 10.0, Color32::from_rgb(16, 185, 129));
            ui.painter().text(
                icon_rect.center(),
                Align2::CENTER_CENTER,
                "👤",
                egui::FontId::proportional(22.0),
                Color32::WHITE,
            );

            ui.add_space(14.0);
            ui.label(RichText::new("Chào mừng bạn đến với chatw!").size(20.0).strong().color(Color32::WHITE));
            ui.label(
                RichText::new("Vui lòng chọn tên người dùng của bạn để bạn bè có thể tìm thấy bạn.")
                    .size(13.0)
                    .color(Color32::from_rgb(161, 161, 170)),
            );

            ui.add_space(20.0);

            let input = egui::TextEdit::singleline(&mut self.name_input)
                .hint_text("Tên người dùng (chữ cái, số, gạch dưới)")
                .min_size(Vec2::new(card_width, 38.0));
            let input_resp = ui.add(input);

            ui.add_space(12.0);

            let submit_btn = egui::Button::new(
                RichText::new(if self.is_loading { "Đang lưu" } else { "Xác nhận & Bắt đầu trò chuyện" })
                    .size(14.0)
                    .color(Color32::WHITE)
                    .strong(),
            )
            .fill(Color32::from_rgb(37, 99, 235))
            .min_size(Vec2::new(card_width, 40.0));

            if (ui.add_enabled(!self.is_loading, submit_btn).clicked()
                || (input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.name_input.trim().is_empty()
            {
                submitted_name = Some(self.name_input.trim().to_string());
            }

            if let Some(ref err) = self.error_msg {
                ui.add_space(12.0);
                ui.label(RichText::new(err).color(Color32::from_rgb(248, 113, 113)).size(12.0));
            }
        });

        submitted_name
    }
}
