use crate::titlebar::{draw_svg_logo_icon, font_medium, font_regular, snap_pos};
use egui::{Align, Align2, Color32, CornerRadius, Layout, RichText, Sense, Ui, Vec2, vec2};

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
        let avail = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(avail, 0.0, Color32::from_rgb(255, 255, 255));

        let card_width = 320.0;
        let top_pad = ((avail.height() - 250.0) * 0.5).max(32.0);

        ui.vertical_centered(|ui| {
            ui.add_space(top_pad);
            ui.set_max_width(card_width);

            // 1. Logo: 44x44 outline chat bubble in #2563EB matching web LogoIcon
            let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), Sense::hover());
            draw_svg_logo_icon(ui.painter(), logo_rect, Color32::from_rgb(37, 99, 235));

            ui.add_space(16.0);

            // 2. Title: Đặt tên của bạn (18px, font-weight 500, color #171717)
            ui.label(
                RichText::new("Đặt tên của bạn")
                    .font(font_medium(18.0))
                    .color(Color32::from_rgb(23, 23, 23)),
            );

            ui.add_space(20.0);

            // 3. Input matching web input (#F7F8FA background, 8px radius, padding 10px 14px)
            let (inp_rect, _) = ui.allocate_exact_size(Vec2::new(card_width, 40.0), Sense::hover());
            ui.painter().rect_filled(
                inp_rect,
                CornerRadius::same(8),
                Color32::from_rgb(247, 248, 250),
            );

            let mut inp_child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inp_rect.shrink2(vec2(14.0, 4.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let input_edit = egui::TextEdit::singleline(&mut self.name_input)
                .font(font_regular(14.0))
                .hint_text(
                    RichText::new("Nhập tên người dùng")
                        .font(font_regular(14.0))
                        .color(Color32::from_rgb(115, 115, 115)),
                )
                .frame(false)
                .desired_width(card_width - 28.0);
            let input_resp = inp_child.add(input_edit);

            // 4. Error message if any (.error-text)
            if let Some(ref err) = self.error_msg {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(err)
                        .font(font_regular(13.0))
                        .color(Color32::from_rgb(220, 38, 38)), // #DC2626
                );
            }

            ui.add_space(16.0);

            // 5. Submit Button matching web .auth-btn-primary
            let (btn_rect, btn_resp) = ui.allocate_exact_size(
                Vec2::new(card_width, 44.0),
                if self.is_loading {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            );

            let btn_bg = if btn_resp.is_pointer_button_down_on() {
                Color32::from_rgb(29, 78, 216) // #1D4ED8
            } else if btn_resp.hovered() {
                Color32::from_rgb(30, 64, 175)
            } else {
                Color32::from_rgb(37, 99, 235) // #2563EB
            };

            ui.painter()
                .rect_filled(btn_rect, CornerRadius::same(8), btn_bg);

            let btn_text = if self.is_loading {
                "Đang lưu"
            } else {
                "Tiếp tục"
            };
            ui.painter().text(
                snap_pos(btn_rect.center(), ui.pixels_per_point()),
                Align2::CENTER_CENTER,
                btn_text,
                font_regular(14.0),
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
