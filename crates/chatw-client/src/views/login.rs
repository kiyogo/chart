use crate::titlebar::{
    draw_smooth_polyline, draw_svg_logo_icon, font_regular, push_arc_points, snap_pos, snap_xy,
};
use egui::{
    Align, Align2, Color32, CornerRadius, Layout, Pos2, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2, pos2, vec2,
};

pub struct LoginView {
    pub username_input: String,
    pub server_url_input: String,
    #[allow(dead_code)]
    pub show_server_config: bool,
    pub is_loading: bool,
    pub is_google_pending: bool,
    pub error_msg: Option<String>,
}

impl Default for LoginView {
    fn default() -> Self {
        Self {
            username_input: String::new(),
            server_url_input: "http://127.0.0.1:8085".to_string(),
            show_server_config: false,
            is_loading: false,
            is_google_pending: false,
            error_msg: None,
        }
    }
}

#[allow(dead_code)]
pub enum LoginAction {
    GoogleLogin,
    DevLogin(String),
    ChangeServer(String),
    CancelGoogleLogin,
}

/// Draws smooth, high-precision Google 4-color "G" logo matching web `<GoogleIcon />`
fn draw_svg_google_g_icon(painter: &egui::Painter, center: Pos2, radius: f32) {
    let stroke_w = (radius * 0.44).max(2.6);
    let r = vec2(radius * 0.78, radius * 0.78);
    let pi = std::f32::consts::PI;

    // 1. Red top arc (220° to 318°)
    let mut red_pts = Vec::with_capacity(16);
    push_arc_points(&mut red_pts, center, r, pi * 1.22, pi * 1.77, 14);
    draw_smooth_polyline(
        painter,
        &red_pts,
        Stroke::new(stroke_w, Color32::from_rgb(234, 67, 53)),
        false,
    );

    // 2. Yellow left arc (140° to 220°)
    let mut yellow_pts = Vec::with_capacity(16);
    push_arc_points(&mut yellow_pts, center, r, pi * 0.78, pi * 1.22, 12);
    draw_smooth_polyline(
        painter,
        &yellow_pts,
        Stroke::new(stroke_w, Color32::from_rgb(251, 188, 5)),
        false,
    );

    // 3. Green bottom arc (42° to 140°)
    let mut green_pts = Vec::with_capacity(16);
    push_arc_points(&mut green_pts, center, r, pi * 0.23, pi * 0.78, 14);
    draw_smooth_polyline(
        painter,
        &green_pts,
        Stroke::new(stroke_w, Color32::from_rgb(52, 168, 83)),
        false,
    );

    // 4. Blue right arc (-8° to 42°) & horizontal crossbar
    let mut blue_pts = Vec::with_capacity(16);
    blue_pts.push(center);
    blue_pts.push(pos2(center.x + r.x, center.y));
    push_arc_points(&mut blue_pts, center, r, 0.0, pi * 0.23, 8);
    draw_smooth_polyline(
        painter,
        &blue_pts,
        Stroke::new(stroke_w, Color32::from_rgb(66, 133, 244)),
        false,
    );
}

impl LoginView {
    pub fn show(&mut self, ui: &mut Ui) -> Option<LoginAction> {
        let mut action = None;

        // Container: White background (#FFFFFF)
        let avail = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(avail, 0.0, Color32::from_rgb(255, 255, 255));

        // Vertically & horizontally centered .auth-box (max-width: 320px)
        let card_width = 320.0;
        let top_pad = ((avail.height() - 330.0) * 0.5).max(32.0);

        ui.vertical_centered(|ui| {
            ui.add_space(top_pad);
            ui.set_max_width(card_width);

            // 1. Logo: 44x44 outline chat bubble in #2563EB
            let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), Sense::hover());
            draw_svg_logo_icon(ui.painter(), logo_rect, Color32::from_rgb(37, 99, 235));

            ui.add_space(12.0);

            // 2. Title: chatw (18px, font-weight 500, color #171717)
            ui.label(
                RichText::new("chatw")
                    .font(font_regular(18.0))
                    .color(Color32::from_rgb(23, 23, 23)),
            );

            ui.add_space(2.0);

            // 3. Subtitle: Ứng dụng nhắn tin bảo mật E2EE (12.0px, color #737373)
            ui.label(
                RichText::new("Ứng dụng nhắn tin bảo mật E2EE")
                    .font(font_regular(12.0))
                    .color(Color32::from_rgb(75, 85, 99)),
            );

            ui.add_space(16.0);

            let ppp = ui.pixels_per_point();

            // 4. Google Login Button (.auth-btn-primary)
            if self.is_google_pending {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(247, 248, 250))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Đang xác thực với Google")
                                    .font(font_regular(14.0))
                                    .color(Color32::from_rgb(23, 23, 23)),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .button(
                                        RichText::new("Hủy")
                                            .font(font_regular(13.0))
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

                // Center [GoogleIcon (18px) + 10px gap + "Tiếp tục với Google"] together matching flex justify-content: center
                let label_text = "Tiếp tục với Google";
                let galley = ui.painter().layout_no_wrap(
                    label_text.to_string(),
                    font_regular(14.0),
                    Color32::WHITE,
                );
                let icon_size = 18.0;
                let gap = 10.0;
                let total_content_w = icon_size + gap + galley.size().x;
                let start_x = btn_rect.center().x - total_content_w * 0.5;

                let icon_center = snap_xy(start_x + icon_size * 0.5, btn_rect.center().y, ppp);
                draw_svg_google_g_icon(ui.painter(), icon_center, 9.0);

                ui.painter().galley(
                    snap_xy(
                        start_x + icon_size + gap,
                        btn_rect.center().y - galley.size().y * 0.5,
                        ppp,
                    ),
                    galley,
                    Color32::WHITE,
                );

                if btn_resp.clicked() && !self.is_loading {
                    action = Some(LoginAction::GoogleLogin);
                }
            }

            ui.add_space(16.0);

            // 5. Divider: "chế độ phát triển cục bộ" (.auth-divider)
            let (div_rect, _) = ui.allocate_exact_size(Vec2::new(card_width, 20.0), Sense::hover());
            let div_galley = ui.painter().layout_no_wrap(
                "chế độ phát triển cục bộ".to_string(),
                font_regular(12.0),
                Color32::from_rgb(75, 85, 99),
            );
            let text_half = div_galley.size().x * 0.5 + 10.0;
            let line_y = div_rect.center().y;
            let line_stroke = Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235));

            ui.painter().line_segment(
                [
                    pos2(div_rect.left(), line_y),
                    pos2(div_rect.center().x - text_half, line_y),
                ],
                line_stroke,
            );
            ui.painter().line_segment(
                [
                    pos2(div_rect.center().x + text_half, line_y),
                    pos2(div_rect.right(), line_y),
                ],
                line_stroke,
            );
            ui.painter().galley(
                snap_xy(
                    div_rect.center().x - div_galley.size().x * 0.5,
                    line_y - div_galley.size().y * 0.5,
                    ppp,
                ),
                div_galley,
                Color32::from_rgb(75, 85, 99),
            );

            ui.add_space(12.0);

            // 6. Dev input matching web .auth-input (1px solid #E5E7EB, focus #2563EB, 8px radius, #FFFFFF bg)
            let (input_box_rect, _) =
                ui.allocate_exact_size(Vec2::new(card_width, 40.0), Sense::hover());
            ui.painter()
                .rect_filled(input_box_rect, CornerRadius::same(8), Color32::WHITE);

            let mut input_child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(input_box_rect.shrink2(vec2(14.0, 4.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let input_edit = egui::TextEdit::singleline(&mut self.username_input)
                .font(font_regular(14.0))
                .hint_text(
                    RichText::new("Tên tài khoản thử (ví dụ: alice)")
                        .font(font_regular(14.0))
                        .color(Color32::from_rgb(75, 85, 99)),
                )
                .frame(false)
                .desired_width(card_width - 28.0);
            let input_resp = input_child.add(input_edit);

            let border_color = if input_resp.has_focus() {
                Color32::from_rgb(37, 99, 235) // #2563EB on focus
            } else {
                Color32::from_rgb(229, 231, 235) // #E5E7EB
            };
            ui.painter().rect_stroke(
                input_box_rect,
                CornerRadius::same(8),
                Stroke::new(1.0_f32, border_color),
                StrokeKind::Inside,
            );

            ui.add_space(10.0);

            // 7. Dev login button matching web .auth-btn-secondary
            let (dev_rect, dev_resp) = ui.allocate_exact_size(
                Vec2::new(card_width, 40.0),
                if self.is_loading {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            );

            let dev_bg = if dev_resp.hovered() {
                Color32::from_rgb(236, 238, 242) // #ECEEF2
            } else {
                Color32::from_rgb(247, 248, 250) // #F7F8FA
            };

            ui.painter()
                .rect_filled(dev_rect, CornerRadius::same(8), dev_bg);
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
                snap_pos(dev_rect.center(), ppp),
                Align2::CENTER_CENTER,
                dev_text,
                font_regular(14.0),
                Color32::from_rgb(23, 23, 23), // #171717
            );

            let should_dev_login = (dev_resp.clicked()
                || (input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.username_input.trim().is_empty();

            if should_dev_login && !self.is_loading {
                action = Some(LoginAction::DevLogin(
                    self.username_input.trim().to_string(),
                ));
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
                        ui.set_max_width(card_width);
                        ui.label(
                            RichText::new(err)
                                .font(font_regular(13.0))
                                .color(Color32::from_rgb(220, 38, 38)),
                        );
                    });
            }

            // 9. Server configuration switcher
            ui.add_space(12.0);
            if self.show_server_config {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(249, 250, 251))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(card_width);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Cài đặt máy chủ:")
                                    .font(font_regular(12.0))
                                    .color(Color32::from_rgb(75, 85, 99)),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .button(
                                        RichText::new("✕")
                                            .font(font_regular(11.0))
                                            .color(Color32::from_rgb(156, 163, 175)),
                                    )
                                    .clicked()
                                {
                                    self.show_server_config = false;
                                }
                            });
                        });
                        ui.add_space(4.0);
                        let srv_edit = egui::TextEdit::singleline(&mut self.server_url_input)
                            .font(font_regular(12.0))
                            .desired_width(card_width - 24.0);
                        ui.add(srv_edit);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    RichText::new("Cục bộ (8085)")
                                        .font(font_regular(12.0))
                                        .color(Color32::from_rgb(37, 99, 235)),
                                )
                                .clicked()
                            {
                                self.server_url_input = "http://127.0.0.1:8085".to_string();
                                action =
                                    Some(LoginAction::ChangeServer(self.server_url_input.clone()));
                            }
                            if ui
                                .button(
                                    RichText::new("Cloud (kiyo.vn)")
                                        .font(font_regular(12.0))
                                        .color(Color32::from_rgb(75, 85, 99)),
                                )
                                .clicked()
                            {
                                self.server_url_input = "https://api.chat.kiyo.vn".to_string();
                                action =
                                    Some(LoginAction::ChangeServer(self.server_url_input.clone()));
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .button(
                                        RichText::new("Áp dụng")
                                            .font(font_regular(12.0))
                                            .color(Color32::from_rgb(37, 99, 235)),
                                    )
                                    .clicked()
                                {
                                    action = Some(LoginAction::ChangeServer(
                                        self.server_url_input.trim().to_string(),
                                    ));
                                }
                            });
                        });
                    });
            } else {
                let srv_btn = ui.button(
                    RichText::new(format!("⚙ {}", self.server_url_input))
                        .font(font_regular(12.0))
                        .color(Color32::from_rgb(156, 163, 175)),
                );
                if srv_btn.clicked() {
                    self.show_server_config = true;
                }
            }
        });

        action
    }
}
