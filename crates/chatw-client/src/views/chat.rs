use std::collections::{HashMap, HashSet};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2,
};
use uuid::Uuid;
use chatw_core::{FriendItem, PendingRequestItem, UserSearchResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarTab {
    Chats,
    Friends,
}

#[derive(Clone, Debug)]
pub struct DecryptedMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub is_me: bool,
    pub text: String,
    pub time_str: String,
}

pub struct ChatView {
    pub current_user_id: Uuid,
    pub current_username: String,
    pub friends: Vec<FriendItem>,
    pub pending_requests: Vec<PendingRequestItem>,
    pub selected_friend_id: Option<Uuid>,
    pub cached_keys: HashMap<Uuid, ([u8; 65], [u8; 32], [u8; 65])>,
    pub messages: HashMap<Uuid, Vec<DecryptedMessage>>,
    pub typing_users: HashSet<Uuid>,
    pub message_input: String,
    pub add_friend_input: String,
    pub search_results: Vec<UserSearchResult>,
    pub status_message: Option<String>,

    pub sidebar_tab: SidebarTab,
    pub is_settings_open: bool,
    pub is_partner_info_open: bool,
    pub is_emoji_picker_open: bool,
    pub server_url_input: String,
}

pub enum ChatAction {
    SelectFriend(Uuid),
    SendMessage(String),
    SendFriendRequest(String),
    AcceptFriendRequest(Uuid),
    RejectFriendRequest(Uuid),
    TypingStarted,
    TypingStopped,
    RefreshData,
    Logout,
    ChangeServerUrl(String),
}

// ==========================================
// PURE RUST VECTOR SVG ICONS MATCHING WEB
// ==========================================

pub fn draw_svg_send_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);

    let poly = [
        pt(22.0, 2.0),
        pt(15.0, 22.0),
        pt(11.0, 13.0),
        pt(2.0, 9.0),
    ];
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);
    painter.line_segment([poly[0], poly[1]], stroke);
    painter.line_segment([poly[1], poly[2]], stroke);
    painter.line_segment([poly[2], poly[3]], stroke);
    painter.line_segment([poly[3], poly[0]], stroke);
    painter.line_segment([pt(22.0, 2.0), pt(11.0, 13.0)], stroke);
}

pub fn draw_svg_menu_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);
    painter.line_segment([pt(3.0, 6.0), pt(21.0, 6.0)], stroke);
    painter.line_segment([pt(3.0, 12.0), pt(21.0, 12.0)], stroke);
    painter.line_segment([pt(3.0, 18.0), pt(21.0, 18.0)], stroke);
}

pub fn draw_svg_close_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.8_f32, color);
    painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
}

pub fn draw_svg_back_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);
    painter.line_segment([pt(19.0, 12.0), pt(5.0, 12.0)], stroke);
    painter.line_segment([pt(12.0, 19.0), pt(5.0, 12.0)], stroke);
    painter.line_segment([pt(5.0, 12.0), pt(12.0, 5.0)], stroke);
}

pub fn draw_svg_chat_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);
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

pub fn draw_svg_friends_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);
    // Primary person
    painter.circle_stroke(pt(9.0, 7.0), 3.5 * s.x, stroke);
    painter.line_segment([pt(4.0, 20.0), pt(4.0, 16.0)], stroke);
    painter.line_segment([pt(4.0, 16.0), pt(14.0, 16.0)], stroke);
    painter.line_segment([pt(14.0, 16.0), pt(14.0, 20.0)], stroke);
    // Secondary person
    painter.circle_stroke(pt(17.0, 8.0), 2.5 * s.x, stroke);
    painter.line_segment([pt(16.0, 15.0), pt(20.0, 15.0)], stroke);
    painter.line_segment([pt(20.0, 15.0), pt(20.0, 20.0)], stroke);
}

pub fn draw_svg_settings_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width() * 0.40;
    let stroke = Stroke::new(1.8_f32, color);
    painter.circle_stroke(center, r * 0.45, stroke);
    for i in 0..8 {
        let angle = (i as f32) * std::f32::consts::PI / 4.0;
        let p1 = center + vec2(angle.cos() * (r * 0.55), angle.sin() * (r * 0.55));
        let p2 = center + vec2(angle.cos() * r, angle.sin() * r);
        painter.line_segment([p1, p2], stroke);
    }
}

pub fn draw_svg_logout_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((1.8 * s.x).max(1.2), color);

    // Door outline (left rounded)
    painter.line_segment([pt(9.0, 4.0), pt(5.0, 4.0)], stroke);
    painter.line_segment([pt(5.0, 4.0), pt(5.0, 20.0)], stroke);
    painter.line_segment([pt(5.0, 20.0), pt(9.0, 20.0)], stroke);

    // Exit arrow (well inside, from x=9 to x=18)
    painter.line_segment([pt(9.0, 12.0), pt(18.0, 12.0)], stroke);
    painter.line_segment([pt(14.5, 8.5), pt(18.0, 12.0)], stroke);
    painter.line_segment([pt(18.0, 12.0), pt(14.5, 15.5)], stroke);
}

pub fn draw_svg_emoji_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width() * 0.42;
    let stroke = Stroke::new(1.8_f32, color);
    painter.circle_stroke(center, r, stroke);
    // Eyes
    painter.circle_filled(center + vec2(-r * 0.35, -r * 0.2), 1.5, color);
    painter.circle_filled(center + vec2(r * 0.35, -r * 0.2), 1.5, color);
    // Smile
    let p_left = center + vec2(-r * 0.4, r * 0.18);
    let p_mid = center + vec2(0.0, r * 0.44);
    let p_right = center + vec2(r * 0.4, r * 0.18);
    painter.line_segment([p_left, p_mid], stroke);
    painter.line_segment([p_mid, p_right], stroke);
}

// ==========================================
// CHAT VIEW IMPLEMENTATION
// ==========================================

impl ChatView {
    pub fn new(user_id: Uuid, username: String) -> Self {
        Self {
            current_user_id: user_id,
            current_username: username,
            friends: Vec::new(),
            pending_requests: Vec::new(),
            selected_friend_id: None,
            cached_keys: HashMap::new(),
            messages: HashMap::new(),
            typing_users: HashSet::new(),
            message_input: String::new(),
            add_friend_input: String::new(),
            search_results: Vec::new(),
            status_message: None,

            sidebar_tab: SidebarTab::Chats,
            is_settings_open: false,
            is_partner_info_open: true,
            is_emoji_picker_open: false,
            server_url_input: "http://127.0.0.1:8082".to_string(),
        }
    }

    /// Master render method leveraging standard egui SidePanel & CentralPanel show_inside
    pub fn show(&mut self, ui: &mut Ui) -> Option<ChatAction> {
        let mut triggered_action = None;
        let active_friend = self.friends.iter().find(|f| Some(f.id) == self.selected_friend_id).cloned();
        let show_right_sidebar = self.is_partner_info_open
            && active_friend.is_some()
            && !self.is_settings_open
            && ui.available_width() >= 880.0;

        // 1. LEFT SIDEBAR PANEL (320px, #F7F8FA background, 1px right border #E5E7EB)
        egui::SidePanel::left("chat_sidebar_panel")
            .exact_width(320.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(247, 248, 250))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235))),
            )
            .show_inside(ui, |ui| {
                self.render_sidebar_content(ui, &mut triggered_action);
            });

        // 2. RIGHT PARTNER INFO SIDEBAR (380px, #FFFFFF background, 1px left border #E5E7EB)
        if show_right_sidebar {
            if let Some(ref friend) = active_friend {
                egui::SidePanel::right("partner_info_panel")
                    .exact_width(380.0)
                    .resizable(false)
                    .frame(
                        egui::Frame::NONE
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235))),
                    )
                    .show_inside(ui, |ui| {
                        self.render_partner_info_sidebar(ui, friend);
                    });
            }
        }

        // 3. CENTER AREA (Fills 100% remaining space, #FFFFFF background)
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
            .show_inside(ui, |ui| {
                if self.is_settings_open {
                    self.render_settings_view(ui, &mut triggered_action);
                } else {
                    self.render_chat_area(ui, &mut triggered_action, active_friend.as_ref());
                }
            });

        triggered_action
    }

    // ==========================================
    // SIDEBAR CONTENT RENDERING (320px)
    // ==========================================
    pub fn render_sidebar_content(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::from_rgb(247, 248, 250));

        // Header (58px) matching web .sidebar-header
        let header_height = 58.0;
        let (header_rect, _) = ui.allocate_exact_size(Vec2::new(total_rect.width(), header_height), Sense::hover());
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        // 1. User profile on the left (.app-brand)
        // 4 action buttons: 4 * 28px + 3 * 3px = 121px + 10px margin = 131px
        let brand_w = (header_rect.width() - 136.0).max(80.0);
        let brand_rect = Rect::from_min_size(
            pos2(header_rect.min.x + 12.0, header_rect.min.y),
            Vec2::new(brand_w, header_height),
        );
        let mut brand_ui = ui.new_child(UiBuilder::new().max_rect(brand_rect).layout(Layout::left_to_right(Align::Center)));

        render_avatar(&mut brand_ui, &self.current_username, 28.0);
        brand_ui.add_space(8.0);

        let display_name: String = if self.current_username.chars().count() > 10 {
            self.current_username.chars().take(9).collect::<String>() + "..."
        } else {
            self.current_username.clone()
        };
        brand_ui.label(
            RichText::new(display_name)
                .size(14.0)
                .strong()
                .color(Color32::from_rgb(23, 23, 23)),
        );

        // 2. 4 Action Buttons on the right (.sidebar-actions)
        let actions_w = 124.0;
        let actions_rect = Rect::from_min_size(
            pos2(header_rect.right() - actions_w - 8.0, header_rect.min.y),
            Vec2::new(actions_w, header_height),
        );
        let mut actions_ui = ui.new_child(UiBuilder::new().max_rect(actions_rect).layout(Layout::left_to_right(Align::Center)));

        let btn_sz = Vec2::new(28.0, 28.0);

        // Button 1: Chats Tab
        let (ch_rect, ch_resp) = actions_ui.allocate_exact_size(btn_sz, Sense::click());
        let is_ch_active = self.sidebar_tab == SidebarTab::Chats && !self.is_settings_open;
        if is_ch_active {
            actions_ui.painter().rect_filled(ch_rect, CornerRadius::same(6), Color32::from_rgb(239, 246, 255)); // Blue-50
        } else if ch_resp.hovered() {
            actions_ui.painter().rect_filled(ch_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let ch_color = if is_ch_active { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) };
        draw_svg_chat_icon(actions_ui.painter(), ch_rect.shrink(5.5), ch_color);
        if ch_resp.clicked() {
            self.sidebar_tab = SidebarTab::Chats;
            self.is_settings_open = false;
        }

        actions_ui.add_space(3.0);

        // Button 2: Friends Tab (with notification badge)
        let (fr_rect, fr_resp) = actions_ui.allocate_exact_size(btn_sz, Sense::click());
        let is_fr_active = self.sidebar_tab == SidebarTab::Friends && !self.is_settings_open;
        if is_fr_active {
            actions_ui.painter().rect_filled(fr_rect, CornerRadius::same(6), Color32::from_rgb(239, 246, 255));
        } else if fr_resp.hovered() {
            actions_ui.painter().rect_filled(fr_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let fr_color = if is_fr_active { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) };
        draw_svg_friends_icon(actions_ui.painter(), fr_rect.shrink(5.5), fr_color);

        if !self.pending_requests.is_empty() {
            let badge_pos = pos2(fr_rect.right() - 2.0, fr_rect.top() + 2.0);
            actions_ui.painter().circle_filled(badge_pos, 6.5, Color32::from_rgb(37, 99, 235));
            actions_ui.painter().text(
                badge_pos,
                Align2::CENTER_CENTER,
                &format!("{}", self.pending_requests.len().min(99)),
                egui::FontId::proportional(8.0),
                Color32::WHITE,
            );
        }
        if fr_resp.clicked() {
            self.sidebar_tab = SidebarTab::Friends;
            self.is_settings_open = false;
        }

        actions_ui.add_space(3.0);

        // Button 3: Settings
        let (set_rect, set_resp) = actions_ui.allocate_exact_size(btn_sz, Sense::click());
        let is_set_active = self.is_settings_open;
        if is_set_active {
            actions_ui.painter().rect_filled(set_rect, CornerRadius::same(6), Color32::from_rgb(239, 246, 255));
        } else if set_resp.hovered() {
            actions_ui.painter().rect_filled(set_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let set_color = if is_set_active { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) };
        draw_svg_settings_icon(actions_ui.painter(), set_rect.shrink(5.5), set_color);
        if set_resp.clicked() {
            self.is_settings_open = !self.is_settings_open;
        }

        actions_ui.add_space(3.0);

        // Button 4: Logout (with red danger hover)
        let (logout_rect, logout_resp) = actions_ui.allocate_exact_size(btn_sz, Sense::click());
        let logout_bg = if logout_resp.hovered() { Color32::from_rgb(252, 232, 230) } else { Color32::TRANSPARENT };
        actions_ui.painter().rect_filled(logout_rect, CornerRadius::same(6), logout_bg);
        let logout_color = if logout_resp.hovered() { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(115, 115, 115) };
        draw_svg_logout_icon(actions_ui.painter(), logout_rect.shrink(5.5), logout_color);
        if logout_resp.clicked() {
            *action = Some(ChatAction::Logout);
        }

        // 3. Tab Contents
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.sidebar_tab {
                    SidebarTab::Chats => {
                        self.render_chats_tab(ui, action);
                    }
                    SidebarTab::Friends => {
                        self.render_friends_tab(ui, action);
                    }
                }
            });
    }

    // ==========================================
    // TAB 1: CHATS LIST
    // ==========================================
    fn render_chats_tab(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        if self.friends.is_empty() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Chưa có bạn bè nào")
                        .size(14.0)
                        .color(Color32::from_rgb(115, 115, 115)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Bấm vào biểu tượng kết bạn ở trên")
                        .size(12.0)
                        .color(Color32::from_rgb(163, 163, 163)),
                );
            });
            return;
        }

        ui.add_space(8.0);
        let item_width = ui.available_width();

        for friend in &self.friends {
            let is_selected = Some(friend.id) == self.selected_friend_id;
            let (item_rect, resp) = ui.allocate_exact_size(Vec2::new(item_width, 64.0), Sense::click());

            let bg = if is_selected {
                Color32::from_rgb(229, 231, 235) // #E5E7EB
            } else if resp.hovered() {
                Color32::from_rgb(236, 238, 242) // #ECEEF2
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(item_rect, CornerRadius::same(8), bg);

            let mut row_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(item_rect.shrink2(vec2(12.0, 8.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );

            render_avatar(&mut row_ui, &friend.username, 40.0);
            row_ui.add_space(10.0);

            let last_msg = self.messages.get(&friend.id).and_then(|m| m.last());

            row_ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&friend.username)
                            .size(14.0)
                            .strong()
                            .color(Color32::from_rgb(23, 23, 23)),
                    );

                    if let Some(msg) = last_msg {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(&msg.time_str)
                                    .size(11.0)
                                    .color(Color32::from_rgb(115, 115, 115)),
                            );
                        });
                    }
                });

                ui.add_space(2.0);

                let preview_text = if let Some(msg) = last_msg {
                    if msg.text.chars().count() > 24 {
                        format!("{}...", msg.text.chars().take(22).collect::<String>())
                    } else {
                        msg.text.clone()
                    }
                } else {
                    "Chưa có tin nhắn".to_string()
                };

                ui.label(
                    RichText::new(preview_text)
                        .size(12.5)
                        .color(Color32::from_rgb(115, 115, 115)),
                );
            });

            if resp.clicked() {
                *action = Some(ChatAction::SelectFriend(friend.id));
                self.is_settings_open = false;
            }
        }
    }

    // ==========================================
    // TAB 2: FRIENDS & REQUESTS LIST
    // ==========================================
    fn render_friends_tab(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        ui.add_space(12.0);

        // Add friend search box
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let input_w = ui.available_width() - 68.0;

            let (inp_rect, _) = ui.allocate_exact_size(Vec2::new(input_w, 36.0), Sense::hover());
            ui.painter().rect_filled(inp_rect, CornerRadius::same(8), Color32::WHITE);
            ui.painter().rect_stroke(
                inp_rect,
                CornerRadius::same(8),
                Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                StrokeKind::Inside,
            );

            let mut inp_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(inp_rect.shrink2(vec2(8.0, 4.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let inp = inp_ui.add(
                egui::TextEdit::singleline(&mut self.add_friend_input)
                    .hint_text("Tên bạn bè...")
                    .frame(false)
                    .desired_width(input_w - 16.0),
            );

            ui.add_space(4.0);

            let (btn_rect, btn_resp) = ui.allocate_exact_size(Vec2::new(48.0, 36.0), Sense::click());
            let btn_bg = if btn_resp.hovered() { Color32::from_rgb(30, 64, 175) } else { Color32::from_rgb(37, 99, 235) };
            ui.painter().rect_filled(btn_rect, CornerRadius::same(8), btn_bg);
            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                "Thêm",
                egui::FontId::proportional(12.5),
                Color32::WHITE,
            );

            let should_send = (btn_resp.clicked()
                || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.add_friend_input.trim().is_empty();

            if should_send {
                *action = Some(ChatAction::SendFriendRequest(
                    self.add_friend_input.trim().to_string(),
                ));
                self.add_friend_input.clear();
            }
        });

        // Status banner
        if let Some(ref status) = self.status_message {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(status).size(12.0).color(Color32::from_rgb(37, 99, 235)));
            });
        }

        // Pending friend requests
        if !self.pending_requests.is_empty() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("LỜI MỜI KẾT BẠN ({})", self.pending_requests.len()))
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(115, 115, 115)),
                );
            });

            for req in &self.pending_requests {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    render_avatar(ui, &req.sender_username, 32.0);
                    ui.add_space(8.0);
                    ui.label(RichText::new(&req.sender_username).size(13.5).strong().color(Color32::from_rgb(23, 23, 23)));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(12.0);
                        if ui.button(RichText::new("✕").size(13.0).color(Color32::from_rgb(220, 38, 38))).clicked() {
                            *action = Some(ChatAction::RejectFriendRequest(req.id));
                        }
                        ui.add_space(6.0);
                        if ui.button(RichText::new("✓").size(13.0).color(Color32::from_rgb(22, 101, 52))).clicked() {
                            *action = Some(ChatAction::AcceptFriendRequest(req.id));
                        }
                    });
                });
            }
        }

        // Friends list
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new(format!("BẠN BÈ ({})", self.friends.len()))
                    .size(11.0)
                    .strong()
                    .color(Color32::from_rgb(115, 115, 115)),
            );
        });

        if self.friends.is_empty() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Chưa có bạn bè nào trong danh bạ")
                        .size(13.0)
                        .color(Color32::from_rgb(163, 163, 163)),
                );
            });
            return;
        }

        let item_width = ui.available_width();
        for friend in &self.friends {
            ui.add_space(4.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_width, 50.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
            }

            let mut row = ui.new_child(
                UiBuilder::new()
                    .max_rect(rect.shrink2(vec2(12.0, 6.0)))
                    .layout(Layout::left_to_right(Align::Center)),
            );

            render_avatar(&mut row, &friend.username, 36.0);
            row.add_space(10.0);

            // Online indicator green dot
            let dot_pos = pos2(rect.left() + 42.0, rect.top() + 36.0);
            ui.painter().circle_filled(dot_pos, 4.0, Color32::from_rgb(34, 197, 94)); // Green-500
            ui.painter().circle_stroke(dot_pos, 4.0, Stroke::new(1.5_f32, Color32::WHITE));

            row.label(
                RichText::new(&friend.username)
                    .size(14.0)
                    .color(Color32::from_rgb(23, 23, 23)),
            );

            if resp.clicked() {
                *action = Some(ChatAction::SelectFriend(friend.id));
                self.sidebar_tab = SidebarTab::Chats;
                self.is_settings_open = false;
            }
        }
    }

    // ==========================================
    // CENTER CHAT AREA RENDERING
    // ==========================================
    pub fn render_chat_area(
        &mut self,
        ui: &mut Ui,
        action: &mut Option<ChatAction>,
        active_friend: Option<&FriendItem>,
    ) {
        let total_rect = ui.available_rect_before_wrap();

        let Some(friend) = active_friend else {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.42);
                ui.label(
                    RichText::new("Chọn một cuộc trò chuyện để bắt đầu")
                        .size(14.0)
                        .color(Color32::from_rgb(115, 115, 115)),
                );
            });
            return;
        };

        let header_height = 54.0;
        let input_bar_height = 60.0;

        // 1. TOP HEADER (54px)
        let header_rect = Rect::from_min_size(total_rect.min, Vec2::new(total_rect.width(), header_height));
        ui.painter().rect_filled(header_rect, 0.0, Color32::WHITE);
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let mut header_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(header_rect.shrink2(vec2(16.0, 8.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );

        render_avatar(&mut header_ui, &friend.username, 32.0);
        header_ui.add_space(10.0);

        header_ui.vertical(|ui| {
            ui.label(
                RichText::new(&friend.username)
                    .size(15.0)
                    .strong()
                    .color(Color32::from_rgb(23, 23, 23)),
            );
            ui.horizontal(|ui| {
                let dot_center = pos2(ui.cursor().min.x + 3.0, ui.cursor().min.y + 6.0);
                ui.painter().circle_filled(dot_center, 3.5, Color32::from_rgb(34, 197, 94));
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Trực tuyến")
                        .size(11.0)
                        .color(Color32::from_rgb(34, 197, 94)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("•  🔒 Mã hóa đầu cuối")
                        .size(11.0)
                        .color(Color32::from_rgb(115, 115, 115)),
                );
            });
        });

        // Toggle info button on header right
        header_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (info_rect, info_resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
            if info_resp.hovered() {
                ui.painter().rect_filled(info_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
            }
            let info_color = if self.is_partner_info_open {
                Color32::from_rgb(37, 99, 235)
            } else {
                Color32::from_rgb(115, 115, 115)
            };
            draw_svg_menu_icon(ui.painter(), info_rect.shrink(7.0), info_color);
            if info_resp.clicked() {
                self.is_partner_info_open = !self.is_partner_info_open;
            }
        });

        // 2. BOTTOM INPUT BAR (60px)
        let input_bar_rect = Rect::from_min_size(
            pos2(total_rect.min.x, total_rect.max.y - input_bar_height),
            Vec2::new(total_rect.width(), input_bar_height),
        );
        ui.painter().rect_filled(input_bar_rect, 0.0, Color32::WHITE);
        ui.painter().line_segment(
            [input_bar_rect.left_top(), input_bar_rect.right_top()],
            Stroke::new(1.0_f32, Color32::from_rgb(241, 243, 245)),
        );

        let mut input_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(input_bar_rect.shrink2(vec2(16.0, 10.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );

        // Emoji Button
        let (emoji_rect, emoji_resp) = input_ui.allocate_exact_size(Vec2::new(36.0, 36.0), Sense::click());
        if emoji_resp.hovered() {
            input_ui.painter().rect_filled(emoji_rect, CornerRadius::same(8), Color32::from_rgb(236, 238, 242));
        }
        draw_svg_emoji_icon(input_ui.painter(), emoji_rect.shrink(8.0), Color32::from_rgb(115, 115, 115));
        if emoji_resp.clicked() {
            self.is_emoji_picker_open = !self.is_emoji_picker_open;
        }

        input_ui.add_space(8.0);

        // Message text input
        let available_input_w = input_ui.available_width() - 48.0;
        let (text_box_rect, _) = input_ui.allocate_exact_size(Vec2::new(available_input_w, 40.0), Sense::hover());
        input_ui.painter().rect_filled(text_box_rect, CornerRadius::same(8), Color32::from_rgb(247, 248, 250));

        let mut text_child = input_ui.new_child(
            UiBuilder::new()
                .max_rect(text_box_rect.shrink2(vec2(12.0, 4.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let msg_edit = egui::TextEdit::singleline(&mut self.message_input)
            .hint_text("Nhập tin nhắn...")
            .frame(false)
            .desired_width(available_input_w - 24.0);
        let msg_resp = text_child.add(msg_edit);

        input_ui.add_space(8.0);

        // Send Button
        let (send_rect, send_resp) = input_ui.allocate_exact_size(Vec2::new(38.0, 38.0), Sense::click());
        let can_send = !self.message_input.trim().is_empty();
        let send_bg = if can_send {
            if send_resp.hovered() { Color32::from_rgb(30, 64, 175) } else { Color32::from_rgb(37, 99, 235) }
        } else {
            Color32::from_rgb(236, 238, 242)
        };
        input_ui.painter().rect_filled(send_rect, CornerRadius::same(8), send_bg);
        let icon_color = if can_send { Color32::WHITE } else { Color32::from_rgb(163, 163, 163) };
        draw_svg_send_icon(input_ui.painter(), send_rect.shrink(10.0), icon_color);

        let should_send = (send_resp.clicked()
            || (msg_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
            && can_send;

        if should_send {
            *action = Some(ChatAction::SendMessage(self.message_input.trim().to_string()));
            self.message_input.clear();
        }

        // Emoji Picker Popup
        if self.is_emoji_picker_open {
            let picker_w = 260.0;
            let picker_h = 180.0;
            let picker_rect = Rect::from_min_size(
                pos2(input_bar_rect.min.x + 16.0, input_bar_rect.min.y - picker_h - 8.0),
                Vec2::new(picker_w, picker_h),
            );

            ui.painter().rect_filled(picker_rect, CornerRadius::same(10), Color32::from_rgb(247, 248, 250));
            ui.painter().rect_stroke(
                picker_rect,
                CornerRadius::same(10),
                Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                StrokeKind::Inside,
            );

            let mut picker_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(picker_rect.shrink2(vec2(10.0, 10.0)))
                    .layout(Layout::top_down(Align::Min)),
            );

            let emojis = [
                "😀", "😃", "😄", "😁", "😆", "😅",
                "🤣", "😂", "🙂", "🙃", "😉", "😊",
                "😇", "🥰", "😍", "🤩", "😘", "😗",
                "👍", "👋", "👏", "🙌", "❤️", "🔥",
            ];

            egui::Grid::new("emoji_grid").spacing(vec2(6.0, 6.0)).show(&mut picker_ui, |ui| {
                for (idx, &e) in emojis.iter().enumerate() {
                    if ui.button(RichText::new(e).size(18.0)).clicked() {
                        self.message_input.push_str(e);
                        self.is_emoji_picker_open = false;
                    }
                    if (idx + 1) % 6 == 0 {
                        ui.end_row();
                    }
                }
            });
        }

        // 3. MIDDLE MESSAGES AREA
        let msgs_rect = Rect::from_min_max(
            pos2(total_rect.min.x, total_rect.min.y + header_height),
            pos2(total_rect.max.x, total_rect.max.y - input_bar_height),
        );

        let mut msgs_ui = ui.new_child(UiBuilder::new().max_rect(msgs_rect).layout(Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(&mut msgs_ui, |ui| {
                ui.add_space(14.0);

                let dummy = Vec::new();
                let msg_list = self.messages.get(&friend.id).unwrap_or(&dummy);

                if msg_list.is_empty() {
                    ui.add_space(msgs_rect.height() * 0.35);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Chưa có tin nhắn nào trong cuộc trò chuyện")
                                .size(13.5)
                                .color(Color32::from_rgb(163, 163, 163)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Gửi tin nhắn đầu tiên được mã hóa E2EE an toàn")
                                .size(12.0)
                                .color(Color32::from_rgb(180, 180, 180)),
                        );
                    });
                } else {
                    for msg in msg_list {
                        render_message_bubble(ui, msg, &friend.username);
                    }
                }

                if self.typing_users.contains(&friend.id) {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label(
                            RichText::new("Đang nhập...")
                                .size(12.0)
                                .italics()
                                .color(Color32::from_rgb(115, 115, 115)),
                        );
                    });
                }

                ui.add_space(12.0);
            });
    }

    // ==========================================
    // SETTINGS VIEW (Replaces center chat area matching web)
    // ==========================================
    pub fn render_settings_view(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::WHITE);

        let header_height = 54.0;
        let header_rect = Rect::from_min_size(total_rect.min, Vec2::new(total_rect.width(), header_height));
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let mut header_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(header_rect.shrink2(vec2(16.0, 8.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );

        // Back button
        let (back_rect, back_resp) = header_ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
        if back_resp.hovered() {
            header_ui.painter().rect_filled(back_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        draw_svg_back_icon(header_ui.painter(), back_rect.shrink(6.0), Color32::from_rgb(23, 23, 23));
        if back_resp.clicked() {
            self.is_settings_open = false;
        }

        header_ui.add_space(10.0);
        header_ui.label(
            RichText::new("Cài đặt")
                .size(16.0)
                .strong()
                .color(Color32::from_rgb(23, 23, 23)),
        );

        // Content Area (ScrollArea)
        let content_rect = Rect::from_min_max(
            pos2(total_rect.min.x, total_rect.min.y + header_height),
            total_rect.max,
        );

        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut content_ui, |ui| {
                ui.add_space(20.0);

                let max_w = 560.0_f32.min(ui.available_width() - 40.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(max_w);

                        // Profile Card (.settings-profile-card)
                        let (card_rect, _) = ui.allocate_exact_size(Vec2::new(max_w, 76.0), Sense::hover());
                        ui.painter().rect_filled(card_rect, CornerRadius::same(10), Color32::from_rgb(247, 248, 250));
                        ui.painter().rect_stroke(
                            card_rect,
                            CornerRadius::same(10),
                            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                            StrokeKind::Inside,
                        );

                        let mut card_ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(card_rect.shrink2(vec2(16.0, 12.0)))
                                .layout(Layout::left_to_right(Align::Center)),
                        );

                        render_avatar(&mut card_ui, &self.current_username, 48.0);
                        card_ui.add_space(14.0);
                        card_ui.vertical(|ui| {
                            ui.label(RichText::new(&self.current_username).size(16.0).strong().color(Color32::from_rgb(23, 23, 23)));
                            ui.label(RichText::new("Tài khoản cá nhân").size(13.0).color(Color32::from_rgb(115, 115, 115)));
                        });

                        ui.add_space(20.0);

                        // Group: Tài khoản
                        ui.label(RichText::new("Tài khoản").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.add_space(6.0);
                        render_settings_row(ui, max_w, "Tên người dùng", &self.current_username, false);
                        render_settings_row(ui, max_w, "Mã định danh", &self.current_user_id.to_string(), true);

                        ui.add_space(18.0);

                        // Group: Mã hóa đầu cuối
                        ui.label(RichText::new("Mã hóa đầu cuối").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.add_space(6.0);
                        render_settings_row(ui, max_w, "Giao thức", "ECDH P-256 / AES-256-GCM", false);
                        render_settings_row(ui, max_w, "Chữ ký số", "ECDSA P-256 / SHA-256", false);
                        render_settings_tag_row(ui, max_w, "Trạng thái khóa", "Đã kích hoạt trên thiết bị");

                        if let Some((_, my_fp, _)) = self.cached_keys.get(&self.current_user_id) {
                            let fp_b64 = URL_SAFE_NO_PAD.encode(my_fp);
                            render_settings_row(ui, max_w, "Vân tay khóa của bạn", &fp_b64, true);
                        }

                        ui.add_space(18.0);

                        // Group: Máy chủ
                        ui.label(RichText::new("Máy chủ & Kết nối").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.add_space(6.0);

                        let (srv_rect, _) = ui.allocate_exact_size(Vec2::new(max_w, 46.0), Sense::hover());
                        ui.painter().rect_filled(srv_rect, CornerRadius::same(8), Color32::from_rgb(247, 248, 250));
                        ui.painter().rect_stroke(
                            srv_rect,
                            CornerRadius::same(8),
                            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                            StrokeKind::Inside,
                        );

                        let mut srv_ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(srv_rect.shrink2(vec2(14.0, 6.0)))
                                .layout(Layout::left_to_right(Align::Center)),
                        );

                        srv_ui.label(RichText::new("URL Máy chủ:").size(13.0).color(Color32::from_rgb(115, 115, 115)));
                        srv_ui.add_space(8.0);
                        let inp = srv_ui.add(egui::TextEdit::singleline(&mut self.server_url_input).desired_width(max_w - 200.0));
                        srv_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button(RichText::new("Lưu").size(12.0)).clicked()
                                || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                *action = Some(ChatAction::ChangeServerUrl(self.server_url_input.trim().to_string()));
                            }
                        });

                        ui.add_space(20.0);

                        // Group: Phiên đăng nhập
                        ui.label(RichText::new("Phiên đăng nhập").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.add_space(8.0);

                        let (logout_btn_rect, logout_btn_resp) = ui.allocate_exact_size(Vec2::new(120.0, 36.0), Sense::click());
                        let logout_bg = if logout_btn_resp.hovered() { Color32::from_rgb(254, 202, 202) } else { Color32::from_rgb(252, 232, 230) };
                        ui.painter().rect_filled(logout_btn_rect, CornerRadius::same(8), logout_bg);
                        ui.painter().text(
                            logout_btn_rect.center(),
                            Align2::CENTER_CENTER,
                            "Đăng xuất",
                            egui::FontId::proportional(13.5),
                            Color32::from_rgb(220, 38, 38),
                        );

                        if logout_btn_resp.clicked() {
                            *action = Some(ChatAction::Logout);
                        }

                        ui.add_space(28.0);
                    });
                });
            });
    }

    // ==========================================
    // RIGHT PARTNER INFO SIDEBAR (380px)
    // ==========================================
    pub fn render_partner_info_sidebar(&mut self, ui: &mut Ui, friend: &FriendItem) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::WHITE);

        let header_height = 54.0;
        let header_rect = Rect::from_min_size(total_rect.min, Vec2::new(total_rect.width(), header_height));
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let mut header_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(header_rect.shrink2(vec2(16.0, 8.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );

        header_ui.label(
            RichText::new("Thông tin")
                .size(15.0)
                .strong()
                .color(Color32::from_rgb(23, 23, 23)),
        );

        header_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (close_rect, close_resp) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click());
            if close_resp.hovered() {
                ui.painter().rect_filled(close_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
            }
            draw_svg_close_icon(ui.painter(), close_rect.shrink(7.0), Color32::from_rgb(115, 115, 115));
            if close_resp.clicked() {
                self.is_partner_info_open = false;
            }
        });

        // Content Area (ScrollArea)
        let content_rect = Rect::from_min_max(
            pos2(total_rect.min.x, total_rect.min.y + header_height),
            total_rect.max,
        );

        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut content_ui, |ui| {
                ui.add_space(20.0);

                let card_w = ui.available_width() - 32.0;
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(card_w);

                        // Avatar card (.partner-info-card)
                        ui.vertical_centered(|ui| {
                            render_avatar(ui, &friend.username, 64.0);
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(&friend.username)
                                    .size(17.0)
                                    .strong()
                                    .color(Color32::from_rgb(23, 23, 23)),
                            );
                            ui.label(
                                RichText::new("Tài khoản bạn bè")
                                    .size(13.0)
                                    .color(Color32::from_rgb(115, 115, 115)),
                            );
                        });

                        ui.add_space(24.0);

                        // Group: Tài khoản
                        ui.label(RichText::new("Tài khoản").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.add_space(6.0);
                        render_settings_row(ui, card_w, "Tên người dùng", &friend.username, false);
                        render_settings_row(ui, card_w, "Mã định danh", &friend.id.to_string(), true);

                        ui.add_space(20.0);

                        // Group: Mã hóa đầu cuối
                        ui.label(RichText::new("Mã hóa đầu cuối").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.add_space(6.0);
                        render_settings_row(ui, card_w, "Giao thức", "ECDH P-256 / AES-256-GCM", false);
                        render_settings_row(ui, card_w, "Chữ ký số", "ECDSA P-256 / SHA-256", false);

                        let has_key = self.cached_keys.contains_key(&friend.id);
                        render_settings_tag_row(
                            ui,
                            card_w,
                            "Trạng thái khóa",
                            if has_key { "Đã kích hoạt trên thiết bị" } else { "Đang đồng bộ..." },
                        );

                        if let Some((_, fp, _)) = self.cached_keys.get(&friend.id) {
                            let fp_b64 = URL_SAFE_NO_PAD.encode(fp);
                            render_settings_row(ui, card_w, "Vân tay khóa", &fp_b64, true);
                        }

                        ui.add_space(24.0);
                    });
                });
            });
    }
}

// ==========================================
// MESSAGE BUBBLE & HELPER ROW DRAWING
// ==========================================

fn render_message_bubble(ui: &mut Ui, msg: &DecryptedMessage, friend_name: &str) {
    let max_bubble_width = (ui.available_width() * 0.65).max(180.0);

    ui.horizontal(|ui| {
        if msg.is_me {
            // Sent Message: Align Right, Blue bubble #2563EB, White text, corners (14, 14, 14, 2)
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(37, 99, 235)) // #2563EB
                        .corner_radius(CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 14,
                            se: 2,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(max_bubble_width);
                            ui.label(RichText::new(&msg.text).size(14.0).color(Color32::WHITE));
                        });

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.add_space(2.0);
                        ui.label(RichText::new(&msg.time_str).size(11.0).color(Color32::from_rgb(115, 115, 115)));
                    });
                });
            });
        } else {
            // Received Message: Align Left, 28px Avatar on left, Gray bubble #F1F3F5, corners (14, 14, 2, 14)
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.add_space(20.0);

                // 28px Avatar matching web
                render_avatar(ui, friend_name, 28.0);
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(241, 243, 245)) // #F1F3F5
                        .corner_radius(CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 2,
                            se: 14,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(max_bubble_width);
                            ui.label(RichText::new(&msg.text).size(14.0).color(Color32::from_rgb(23, 23, 23)));
                        });

                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.add_space(2.0);
                        ui.label(RichText::new(&msg.time_str).size(11.0).color(Color32::from_rgb(115, 115, 115)));
                    });
                });
            });
        }
    });

    ui.add_space(6.0);
}

fn render_settings_row(ui: &mut Ui, width: f32, label: &str, value: &str, is_mono: bool) {
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, 42.0), Sense::hover());
    ui.painter().rect_filled(row_rect, CornerRadius::same(8), Color32::from_rgb(247, 248, 250));
    ui.painter().rect_stroke(
        row_rect,
        CornerRadius::same(8),
        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        StrokeKind::Inside,
    );

    let mut row_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(row_rect.shrink2(vec2(14.0, 4.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );

    row_ui.label(RichText::new(label).size(13.0).color(Color32::from_rgb(115, 115, 115)));
    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let text = if is_mono {
            RichText::new(value).size(11.5).monospace().color(Color32::from_rgb(23, 23, 23))
        } else {
            RichText::new(value).size(13.0).strong().color(Color32::from_rgb(23, 23, 23))
        };
        ui.label(text);
    });
}

fn render_settings_tag_row(ui: &mut Ui, width: f32, label: &str, tag_text: &str) {
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, 42.0), Sense::hover());
    ui.painter().rect_filled(row_rect, CornerRadius::same(8), Color32::from_rgb(247, 248, 250));
    ui.painter().rect_stroke(
        row_rect,
        CornerRadius::same(8),
        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        StrokeKind::Inside,
    );

    let mut row_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(row_rect.shrink2(vec2(14.0, 4.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );

    row_ui.label(RichText::new(label).size(13.0).color(Color32::from_rgb(115, 115, 115)));
    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let (tag_rect, _) = ui.allocate_exact_size(Vec2::new(150.0, 24.0), Sense::hover());
        ui.painter().rect_filled(tag_rect, CornerRadius::same(4), Color32::from_rgb(230, 244, 234));
        ui.painter().text(
            tag_rect.center(),
            Align2::CENTER_CENTER,
            tag_text,
            egui::FontId::proportional(11.5),
            Color32::from_rgb(19, 115, 51),
        );
    });
}

pub fn render_avatar(ui: &mut Ui, name: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();

    let bg_color = match name.bytes().next().unwrap_or(0) % 6 {
        0 => Color32::from_rgb(37, 99, 235),  // Blue
        1 => Color32::from_rgb(13, 148, 136), // Teal
        2 => Color32::from_rgb(217, 119, 6),  // Amber
        3 => Color32::from_rgb(220, 38, 38),  // Red
        4 => Color32::from_rgb(147, 51, 234), // Purple
        _ => Color32::from_rgb(5, 150, 105),  // Emerald
    };

    ui.painter().circle_filled(rect.center(), size / 2.0, bg_color);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        egui::FontId::proportional(size * 0.44),
        Color32::WHITE,
    );
}
