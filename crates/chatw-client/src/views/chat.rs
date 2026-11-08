use std::collections::{HashMap, HashSet};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use egui::{Align, Align2, Color32, CornerRadius, Layout, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use uuid::Uuid;
use chatw_core::{FriendItem, PendingRequestItem, UserSearchResult};

const EMOJIS: &[&str] = &[
    "😀", "😂", "🥹", "❤️", "👍", "🎉",
    "😭", "😍", "🔥", "😊", "🥳", "🙏",
    "👏", "✨", "🚀", "💯", "😎", "🙌",
    "🤝", "🤩", "😴", "🤔", "✌️", "💪",
];

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DecryptedMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub is_me: bool,
    pub text: String,
    pub time_str: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarTab {
    Chats,
    Friends,
}

pub struct ChatView {
    pub current_user_id: Uuid,
    pub current_username: String,
    pub friends: Vec<FriendItem>,
    pub pending_requests: Vec<PendingRequestItem>,
    pub selected_friend_id: Option<Uuid>,
    pub cached_keys: HashMap<Uuid, ([u8; 65], [u8; 32], [u8; 65])>, // (ecdh_pub, fingerprint, ecdsa_pub)
    pub messages: HashMap<Uuid, Vec<DecryptedMessage>>,
    pub typing_users: HashSet<Uuid>,
    pub message_input: String,
    pub add_friend_input: String,
    pub search_results: Vec<UserSearchResult>,
    pub status_message: Option<String>,

    // Web UI navigation states
    pub sidebar_tab: SidebarTab,
    pub is_settings_open: bool,
    pub is_partner_info_open: bool,
    pub is_emoji_picker_open: bool,
    pub server_url_input: String,
}

#[allow(dead_code)]
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

pub fn avatar_bg_color(name: &str) -> Color32 {
    let palette = [
        Color32::from_rgb(37, 99, 235),  // #2563EB Blue
        Color32::from_rgb(124, 58, 237), // #7C3AED Violet
        Color32::from_rgb(5, 150, 105),  // #059669 Emerald
        Color32::from_rgb(217, 119, 6),  // #D97706 Amber
        Color32::from_rgb(225, 29, 72),  // #E11D48 Rose
        Color32::from_rgb(8, 145, 178),  // #0891B2 Cyan
        Color32::from_rgb(79, 70, 229),  // #4F46E5 Indigo
        Color32::from_rgb(13, 148, 136), // #0D9488 Teal
    ];
    let mut hash: i32 = 0;
    for b in name.as_bytes() {
        hash = ((hash << 5).wrapping_sub(hash)).wrapping_add(*b as i32);
    }
    let idx = (hash.abs() as usize) % palette.len();
    palette[idx]
}

pub fn render_avatar(ui: &mut Ui, name: &str, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    let bg = avatar_bg_color(name);
    ui.painter().circle_filled(rect.center(), size / 2.0, bg);
    let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();
    let font_size = if size <= 28.0 {
        11.0
    } else if size <= 36.0 {
        13.5
    } else if size <= 44.0 {
        16.0
    } else {
        24.0
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        &initial,
        egui::FontId::proportional(font_size),
        Color32::WHITE,
    );
    resp
}

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

    pub fn show(&mut self, ui: &mut Ui) -> Option<ChatAction> {
        let mut triggered_action = None;

        let total_rect = ui.available_rect_before_wrap();
        let sidebar_width = 300.0_f32.min(total_rect.width() * 0.45);
        let border_stroke = Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)); // #E5E7EB

        let active_friend = self.friends.iter().find(|f| Some(f.id) == self.selected_friend_id).cloned();
        let show_right_sidebar = self.is_partner_info_open && active_friend.is_some() && !self.is_settings_open;
        let right_sidebar_width = if show_right_sidebar { 300.0_f32.min(total_rect.width() * 0.35) } else { 0.0 };

        let chat_width = (total_rect.width() - sidebar_width - right_sidebar_width - 2.0).max(100.0);

        ui.horizontal(|ui| {
            // 1. LEFT SIDEBAR (Width 300px, background #F7F8FA)
            ui.allocate_ui_with_layout(
                Vec2::new(sidebar_width, total_rect.height()),
                Layout::top_down(Align::Min),
                |ui| {
                    self.render_sidebar(ui, &mut triggered_action);
                },
            );

            // Left Separator line segment (#E5E7EB)
            let left_line_x = total_rect.min.x + sidebar_width;
            ui.painter().line_segment(
                [egui::pos2(left_line_x, total_rect.min.y), egui::pos2(left_line_x, total_rect.max.y)],
                border_stroke,
            );

            // 2. CENTER AREA (Chat conversation or Settings View)
            ui.allocate_ui_with_layout(
                Vec2::new(chat_width, total_rect.height()),
                Layout::top_down(Align::Min),
                |ui| {
                    if self.is_settings_open {
                        self.render_settings_view(ui, &mut triggered_action);
                    } else {
                        self.render_chat_area(ui, &mut triggered_action, active_friend.as_ref());
                    }
                },
            );

            // 3. RIGHT PARTNER INFO SIDEBAR (Width 300px, background #FFFFFF)
            if show_right_sidebar {
                let right_line_x = total_rect.max.x - right_sidebar_width;
                ui.painter().line_segment(
                    [egui::pos2(right_line_x, total_rect.min.y), egui::pos2(right_line_x, total_rect.max.y)],
                    border_stroke,
                );

                ui.allocate_ui_with_layout(
                    Vec2::new(right_sidebar_width, total_rect.height()),
                    Layout::top_down(Align::Min),
                    |ui| {
                        if let Some(ref friend) = active_friend {
                            self.render_partner_info_sidebar(ui, friend);
                        }
                    },
                );
            }
        });

        triggered_action
    }

    // ==========================================
    // SIDEBAR RENDERING (Left 300px)
    // ==========================================
    fn render_sidebar(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        // Sidebar background: #F7F8FA
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(247, 248, 250));

        // 1. Sidebar Header (height 58px)
        let header_height = 58.0;
        let (header_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), header_height), Sense::hover());
        ui.painter().rect_filled(header_rect, 0.0, Color32::from_rgb(247, 248, 250));
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)), // #E5E7EB
        );

        let mut header_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(header_rect.shrink2(egui::vec2(12.0, 8.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );

        // User Avatar + Username
        render_avatar(&mut header_ui, &self.current_username, 34.0);
        header_ui.add_space(8.0);
        header_ui.label(
            RichText::new(&self.current_username)
                .size(14.0)
                .strong()
                .color(Color32::from_rgb(23, 23, 23)),
        );

        // Header Actions (right aligned: Chats, Friends, Settings, Logout)
        header_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Logout button
            let (logout_rect, logout_resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
            let logout_bg = if logout_resp.hovered() {
                Color32::from_rgb(252, 232, 230) // #FCE8E6
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(logout_rect, CornerRadius::same(6), logout_bg);
            ui.painter().text(
                logout_rect.center(),
                Align2::CENTER_CENTER,
                "🚪",
                egui::FontId::proportional(14.0),
                if logout_resp.hovered() { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(115, 115, 115) },
            );
            if logout_resp.clicked() {
                *action = Some(ChatAction::Logout);
            }

            ui.add_space(4.0);

            // Settings button
            let (set_rect, set_resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
            let is_set_active = self.is_settings_open;
            let set_bg = if is_set_active || set_resp.hovered() {
                Color32::from_rgb(236, 238, 242) // #ECEEF2
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(set_rect, CornerRadius::same(6), set_bg);
            ui.painter().text(
                set_rect.center(),
                Align2::CENTER_CENTER,
                "⚙",
                egui::FontId::proportional(15.0),
                if is_set_active { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) },
            );
            if set_resp.clicked() {
                self.is_settings_open = !self.is_settings_open;
            }

            ui.add_space(4.0);

            // Friends tab button
            let (fr_rect, fr_resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
            let is_fr_active = self.sidebar_tab == SidebarTab::Friends && !self.is_settings_open;
            let fr_bg = if is_fr_active || fr_resp.hovered() {
                Color32::from_rgb(236, 238, 242)
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(fr_rect, CornerRadius::same(6), fr_bg);
            ui.painter().text(
                fr_rect.center(),
                Align2::CENTER_CENTER,
                "👥",
                egui::FontId::proportional(14.0),
                if is_fr_active { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) },
            );
            // Badge for pending requests
            if !self.pending_requests.is_empty() {
                let badge_pos = egui::pos2(fr_rect.right() - 2.0, fr_rect.top() + 2.0);
                ui.painter().circle_filled(badge_pos, 7.0, Color32::from_rgb(37, 99, 235));
                ui.painter().text(
                    badge_pos,
                    Align2::CENTER_CENTER,
                    &format!("{}", self.pending_requests.len().min(99)),
                    egui::FontId::proportional(8.5),
                    Color32::WHITE,
                );
            }
            if fr_resp.clicked() {
                self.sidebar_tab = SidebarTab::Friends;
                self.is_settings_open = false;
            }

            ui.add_space(4.0);

            // Chats tab button
            let (ch_rect, ch_resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
            let is_ch_active = self.sidebar_tab == SidebarTab::Chats && !self.is_settings_open;
            let ch_bg = if is_ch_active || ch_resp.hovered() {
                Color32::from_rgb(236, 238, 242)
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(ch_rect, CornerRadius::same(6), ch_bg);
            ui.painter().text(
                ch_rect.center(),
                Align2::CENTER_CENTER,
                "💬",
                egui::FontId::proportional(14.0),
                if is_ch_active { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) },
            );
            if ch_resp.clicked() {
                self.sidebar_tab = SidebarTab::Chats;
                self.is_settings_open = false;
            }
        });

        // 2. Tab Navigation Bar (Text tabs)
        ui.add_space(6.0);
        let tab_bar_width = ui.available_width() - 16.0;
        let (tab_bar_rect, _) = ui.allocate_exact_size(Vec2::new(tab_bar_width, 32.0), Sense::hover());
        ui.painter().rect_filled(tab_bar_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));

        let tab_half_w = tab_bar_width / 2.0;

        // "Đoạn chat" Tab button
        let chats_rect = egui::Rect::from_min_size(tab_bar_rect.min, Vec2::new(tab_half_w, 32.0));
        let chats_resp = ui.interact(chats_rect, ui.id().with("tab_chats"), Sense::click());
        if self.sidebar_tab == SidebarTab::Chats {
            ui.painter().rect_filled(chats_rect.shrink(2.0), CornerRadius::same(5), Color32::WHITE);
        }
        ui.painter().text(
            chats_rect.center(),
            Align2::CENTER_CENTER,
            "Đoạn chat",
            egui::FontId::proportional(12.5),
            if self.sidebar_tab == SidebarTab::Chats { Color32::from_rgb(23, 23, 23) } else { Color32::from_rgb(115, 115, 115) },
        );
        if chats_resp.clicked() {
            self.sidebar_tab = SidebarTab::Chats;
            self.is_settings_open = false;
        }

        // "Bạn bè" Tab button
        let friends_rect = egui::Rect::from_min_size(egui::pos2(tab_bar_rect.min.x + tab_half_w, tab_bar_rect.min.y), Vec2::new(tab_half_w, 32.0));
        let friends_resp = ui.interact(friends_rect, ui.id().with("tab_friends"), Sense::click());
        if self.sidebar_tab == SidebarTab::Friends {
            ui.painter().rect_filled(friends_rect.shrink(2.0), CornerRadius::same(5), Color32::WHITE);
        }
        let friends_label = if !self.pending_requests.is_empty() {
            format!("Bạn bè ({})", self.pending_requests.len())
        } else {
            "Bạn bè".to_string()
        };
        ui.painter().text(
            friends_rect.center(),
            Align2::CENTER_CENTER,
            &friends_label,
            egui::FontId::proportional(12.5),
            if self.sidebar_tab == SidebarTab::Friends { Color32::from_rgb(23, 23, 23) } else { Color32::from_rgb(115, 115, 115) },
        );
        if friends_resp.clicked() {
            self.sidebar_tab = SidebarTab::Friends;
            self.is_settings_open = false;
        }

        ui.add_space(8.0);

        // 3. Tab Contents
        match self.sidebar_tab {
            SidebarTab::Chats => {
                self.render_sidebar_chats_tab(ui, action);
            }
            SidebarTab::Friends => {
                self.render_sidebar_friends_tab(ui, action);
            }
        }
    }

    fn render_sidebar_chats_tab(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        // Container background matching web .sidebar-scrollable { background-color: var(--bg-primary); }
        let scroll_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(scroll_rect, 0.0, Color32::from_rgb(255, 255, 255));

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(6.0);

            if self.friends.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("Chưa có bạn bè nào").size(13.0).color(Color32::from_rgb(115, 115, 115)));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Chuyển sang tab Bạn bè để kết bạn").size(11.5).color(Color32::from_rgb(161, 161, 170)));
                });
            } else {
                for friend in &self.friends {
                    let is_selected = self.selected_friend_id == Some(friend.id) && !self.is_settings_open;
                    let item_width = ui.available_width() - 12.0;
                    let (item_rect, item_resp) = ui.allocate_exact_size(Vec2::new(item_width, 56.0), Sense::click());

                    let bg = if is_selected {
                        Color32::from_rgb(230, 237, 254) // #E6EDFE
                    } else if item_resp.hovered() {
                        Color32::from_rgb(236, 238, 242) // #ECEEF2
                    } else {
                        Color32::TRANSPARENT
                    };

                    ui.painter().rect_filled(item_rect, CornerRadius::same(8), bg);

                    // Avatar (36px)
                    let av_center = egui::pos2(item_rect.left() + 26.0, item_rect.center().y);
                    let av_bg = avatar_bg_color(&friend.username);
                    ui.painter().circle_filled(av_center, 17.0, av_bg);
                    let initial = friend.username.chars().next().unwrap_or('F').to_uppercase().to_string();
                    ui.painter().text(
                        av_center,
                        Align2::CENTER_CENTER,
                        &initial,
                        egui::FontId::proportional(14.0),
                        Color32::WHITE,
                    );

                    // Name & Preview
                    let text_left = item_rect.left() + 52.0;
                    ui.painter().text(
                        egui::pos2(text_left, item_rect.top() + 18.0),
                        Align2::LEFT_CENTER,
                        &friend.username,
                        egui::FontId::proportional(13.5),
                        Color32::from_rgb(23, 23, 23), // #171717
                    );

                    // Preview text & time
                    let last_msg = self.messages.get(&friend.id).and_then(|msgs| msgs.last());
                    let is_typing = self.typing_users.contains(&friend.id);

                    if is_typing {
                        ui.painter().text(
                            egui::pos2(text_left, item_rect.top() + 38.0),
                            Align2::LEFT_CENTER,
                            "✍ Đang soạn tin...",
                            egui::FontId::proportional(11.5),
                            Color32::from_rgb(37, 99, 235), // Blue
                        );
                    } else if let Some(m) = last_msg {
                        let preview: String = if m.text.chars().count() > 24 {
                            m.text.chars().take(22).collect::<String>() + "..."
                        } else {
                            m.text.clone()
                        };
                        ui.painter().text(
                            egui::pos2(text_left, item_rect.top() + 38.0),
                            Align2::LEFT_CENTER,
                            &preview,
                            egui::FontId::proportional(11.5),
                            Color32::from_rgb(115, 115, 115), // #737373
                        );

                        // Timestamp on the right
                        ui.painter().text(
                            egui::pos2(item_rect.right() - 10.0, item_rect.top() + 18.0),
                            Align2::RIGHT_CENTER,
                            &m.time_str,
                            egui::FontId::proportional(10.5),
                            Color32::from_rgb(115, 115, 115),
                        );
                    } else {
                        ui.painter().text(
                            egui::pos2(text_left, item_rect.top() + 38.0),
                            Align2::LEFT_CENTER,
                            "Chưa có tin nhắn",
                            egui::FontId::proportional(11.5),
                            Color32::from_rgb(161, 161, 170),
                        );
                    }

                    if item_resp.clicked() {
                        *action = Some(ChatAction::SelectFriend(friend.id));
                        self.is_settings_open = false;
                    }
                }
            }
        });
    }

    fn render_sidebar_friends_tab(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // Add friend form matching web .add-friend-container
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                let input_w = ui.available_width() - 76.0;
                let inp = ui.add(
                    egui::TextEdit::singleline(&mut self.add_friend_input)
                        .hint_text("Tên tài khoản")
                        .desired_width(input_w),
                );
                let (btn_rect, btn_resp) = ui.allocate_exact_size(Vec2::new(56.0, 32.0), Sense::click());
                let btn_bg = if btn_resp.hovered() { Color32::from_rgb(30, 64, 175) } else { Color32::from_rgb(37, 99, 235) };
                ui.painter().rect_filled(btn_rect, CornerRadius::same(6), btn_bg);
                ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "Gửi", egui::FontId::proportional(12.5), Color32::WHITE);

                let should_add = (btn_resp.clicked() || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                    && !self.add_friend_input.trim().is_empty();

                if should_add {
                    let target = self.add_friend_input.trim().to_string();
                    *action = Some(ChatAction::SendFriendRequest(target));
                    self.add_friend_input.clear();
                }
            });

            // Status message
            if let Some(ref msg) = self.status_message {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new(msg).size(11.5).color(Color32::from_rgb(37, 99, 235)));
                });
            }

            ui.add_space(12.0);

            // Pending requests section
            if !self.pending_requests.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("Lời mời kết bạn ({})", self.pending_requests.len()))
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(217, 119, 6)), // Amber
                    );
                });
                ui.add_space(4.0);

                for req in &self.pending_requests {
                    let card_w = ui.available_width() - 20.0;
                    let (card_rect, _) = ui.allocate_exact_size(Vec2::new(card_w, 46.0), Sense::hover());
                    ui.painter().rect_filled(card_rect, CornerRadius::same(6), Color32::WHITE);
                    ui.painter().rect_stroke(
                        card_rect,
                        CornerRadius::same(6),
                        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                        StrokeKind::Inside,
                    );

                    let mut card_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(card_rect.shrink2(egui::vec2(8.0, 4.0)))
                            .layout(Layout::left_to_right(Align::Center)),
                    );

                    render_avatar(&mut card_ui, &req.sender_username, 28.0);
                    card_ui.add_space(6.0);
                    card_ui.label(
                        RichText::new(&req.sender_username)
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(23, 23, 23)),
                    );

                    card_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Reject button
                        let (rej_rect, rej_resp) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::click());
                        ui.painter().rect_filled(rej_rect, CornerRadius::same(4), Color32::from_rgb(252, 232, 230));
                        ui.painter().text(rej_rect.center(), Align2::CENTER_CENTER, "✕", egui::FontId::proportional(11.0), Color32::from_rgb(220, 38, 38));
                        if rej_resp.clicked() {
                            *action = Some(ChatAction::RejectFriendRequest(req.id));
                        }

                        ui.add_space(4.0);

                        // Accept button
                        let (acc_rect, acc_resp) = ui.allocate_exact_size(Vec2::new(56.0, 26.0), Sense::click());
                        ui.painter().rect_filled(acc_rect, CornerRadius::same(4), Color32::from_rgb(37, 99, 235));
                        ui.painter().text(acc_rect.center(), Align2::CENTER_CENTER, "Đồng ý", egui::FontId::proportional(11.0), Color32::WHITE);
                        if acc_resp.clicked() {
                            *action = Some(ChatAction::AcceptFriendRequest(req.id));
                        }
                    });
                }
                ui.add_space(10.0);
            }

            // Confirmed friends list header
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("Danh sách bạn bè ({})", self.friends.len()))
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(115, 115, 115)),
                );
            });
            ui.add_space(4.0);

            if self.friends.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Chưa có bạn bè nào").size(12.0).color(Color32::from_rgb(161, 161, 170)));
                });
            } else {
                for friend in &self.friends {
                    let row_w = ui.available_width() - 20.0;
                    let (row_rect, row_resp) = ui.allocate_exact_size(Vec2::new(row_w, 42.0), Sense::click());
                    let row_bg = if row_resp.hovered() { Color32::from_rgb(236, 238, 242) } else { Color32::WHITE };
                    ui.painter().rect_filled(row_rect, CornerRadius::same(6), row_bg);

                    let mut row_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(row_rect.shrink2(egui::vec2(8.0, 4.0)))
                            .layout(Layout::left_to_right(Align::Center)),
                    );

                    render_avatar(&mut row_ui, &friend.username, 28.0);
                    row_ui.add_space(6.0);
                    row_ui.label(RichText::new(&friend.username).size(13.0).color(Color32::from_rgb(23, 23, 23)));

                    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (chat_rect, chat_resp) = ui.allocate_exact_size(Vec2::new(60.0, 24.0), Sense::click());
                        ui.painter().rect_filled(chat_rect, CornerRadius::same(4), Color32::from_rgb(236, 238, 242));
                        ui.painter().text(chat_rect.center(), Align2::CENTER_CENTER, "Nhắn tin", egui::FontId::proportional(11.0), Color32::from_rgb(37, 99, 235));
                        if chat_resp.clicked() || row_resp.clicked() {
                            *action = Some(ChatAction::SelectFriend(friend.id));
                            self.sidebar_tab = SidebarTab::Chats;
                            self.is_settings_open = false;
                        }
                    });
                }
            }
        });
    }

    // ==========================================
    // CHAT AREA RENDERING (Center Area)
    // ==========================================
    fn render_chat_area(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>, active_friend: Option<&FriendItem>) {
        // Chat area background: #FFFFFF
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(255, 255, 255));

        match active_friend {
            None => {
                // Empty state matching web .empty-chat
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.32);
                    let (lock_rect, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), Sense::hover());
                    ui.painter().circle_filled(lock_rect.center(), 24.0, Color32::from_rgb(241, 243, 245));
                    ui.painter().text(lock_rect.center(), Align2::CENTER_CENTER, "🔒", egui::FontId::proportional(22.0), Color32::from_rgb(37, 99, 235));

                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("Tin nhắn bảo mật E2EE")
                            .size(17.0)
                            .strong()
                            .color(Color32::from_rgb(23, 23, 23)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Chọn một đoạn chat để bắt đầu nhắn tin an toàn.")
                            .size(13.0)
                            .color(Color32::from_rgb(115, 115, 115)),
                    );
                });
            }
            Some(friend) => {
                // 1. Chat Header (height 54px)
                let header_height = 54.0;
                let (header_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), header_height), Sense::hover());
                ui.painter().rect_filled(header_rect, 0.0, Color32::from_rgb(255, 255, 255));
                ui.painter().line_segment(
                    [header_rect.left_bottom(), header_rect.right_bottom()],
                    Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                );

                let mut header_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(header_rect.shrink2(egui::vec2(16.0, 8.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );

                render_avatar(&mut header_ui, &friend.username, 34.0);
                header_ui.add_space(10.0);

                header_ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&friend.username)
                            .size(14.5)
                            .strong()
                            .color(Color32::from_rgb(23, 23, 23)),
                    );
                    ui.horizontal(|ui| {
                        let (tag_rect, _) = ui.allocate_exact_size(Vec2::new(104.0, 16.0), Sense::hover());
                        ui.painter().rect_filled(tag_rect, CornerRadius::same(4), Color32::from_rgb(230, 244, 234)); // #E6F4EA
                        ui.painter().text(
                            tag_rect.center(),
                            Align2::CENTER_CENTER,
                            "🔒 Mã hóa đầu cuối",
                            egui::FontId::proportional(10.0),
                            Color32::from_rgb(19, 115, 51), // #137333
                        );
                    });
                });

                header_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (info_rect, info_resp) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
                    let info_bg = if self.is_partner_info_open || info_resp.hovered() {
                        Color32::from_rgb(236, 238, 242)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(info_rect, CornerRadius::same(6), info_bg);
                    ui.painter().text(
                        info_rect.center(),
                        Align2::CENTER_CENTER,
                        "ℹ",
                        egui::FontId::proportional(15.0),
                        if self.is_partner_info_open { Color32::from_rgb(37, 99, 235) } else { Color32::from_rgb(115, 115, 115) },
                    );
                    if info_resp.clicked() {
                        self.is_partner_info_open = !self.is_partner_info_open;
                    }
                });

                // 2. Messages List Area
                let input_bar_height = 60.0;
                let msgs_height = (ui.available_height() - input_bar_height).max(50.0);

                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), msgs_height),
                    Layout::top_down(Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.add_space(12.0);

                                let empty_vec = Vec::new();
                                let msg_list = self.messages.get(&friend.id).unwrap_or(&empty_vec);

                                if msg_list.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(60.0);
                                        ui.label(
                                            RichText::new("Chưa có tin nhắn")
                                                .size(13.0)
                                                .color(Color32::from_rgb(161, 161, 170)),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new("Gửi lời chào đầu tiên được mã hóa an toàn!")
                                                .size(11.5)
                                                .color(Color32::from_rgb(115, 115, 115)),
                                        );
                                    });
                                } else {
                                    for m in msg_list {
                                        render_message_bubble(ui, m);
                                    }
                                }

                                if self.typing_users.contains(&friend.id) {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(20.0);
                                        ui.label(
                                            RichText::new(format!("{} đang nhập...", friend.username))
                                                .size(11.5)
                                                .italics()
                                                .color(Color32::from_rgb(115, 115, 115)),
                                        );
                                    });
                                }

                                ui.add_space(10.0);
                            });
                    },
                );

                // 3. Emoji Picker Popup (if open)
                if self.is_emoji_picker_open {
                    let picker_w = 260.0;
                    let picker_h = 170.0;
                    let picker_x = ui.cursor().left() + 16.0;
                    let picker_y = ui.cursor().top() - picker_h - 10.0;
                    let picker_rect = egui::Rect::from_min_size(egui::pos2(picker_x, picker_y), Vec2::new(picker_w, picker_h));

                    // Frame
                    ui.painter().rect_filled(picker_rect, CornerRadius::same(10), Color32::from_rgb(247, 248, 250));
                    ui.painter().rect_stroke(
                        picker_rect,
                        CornerRadius::same(10),
                        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                        StrokeKind::Inside,
                    );

                    let mut picker_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(picker_rect.shrink(8.0))
                            .layout(Layout::top_down(Align::Min)),
                    );

                    egui::Grid::new("emoji_grid")
                        .spacing([4.0, 4.0])
                        .show(&mut picker_ui, |ui| {
                            for (i, emoji) in EMOJIS.iter().enumerate() {
                                let (e_rect, e_resp) = ui.allocate_exact_size(Vec2::new(36.0, 34.0), Sense::click());
                                if e_resp.hovered() {
                                    ui.painter().rect_filled(e_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
                                }
                                ui.painter().text(e_rect.center(), Align2::CENTER_CENTER, *emoji, egui::FontId::proportional(18.0), Color32::WHITE);
                                if e_resp.clicked() {
                                    self.message_input.push_str(emoji);
                                    self.is_emoji_picker_open = false;
                                }
                                if (i + 1) % 6 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                }

                // 4. Input Bar (height 60px matching web .chat-input-bar)
                let input_bar_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 60.0));
                ui.painter().rect_filled(input_bar_rect, 0.0, Color32::from_rgb(255, 255, 255));
                ui.painter().line_segment(
                    [input_bar_rect.left_top(), input_bar_rect.right_top()],
                    Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                );

                let mut bar_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(input_bar_rect.shrink2(egui::vec2(16.0, 10.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );

                // Emoji button
                let (em_rect, em_resp) = bar_ui.allocate_exact_size(Vec2::new(36.0, 36.0), Sense::click());
                let em_bg = if self.is_emoji_picker_open || em_resp.hovered() {
                    Color32::from_rgb(236, 238, 242)
                } else {
                    Color32::TRANSPARENT
                };
                bar_ui.painter().rect_filled(em_rect, CornerRadius::same(6), em_bg);
                bar_ui.painter().text(em_rect.center(), Align2::CENTER_CENTER, "🙂", egui::FontId::proportional(18.0), Color32::from_rgb(115, 115, 115));
                if em_resp.clicked() {
                    self.is_emoji_picker_open = !self.is_emoji_picker_open;
                }

                bar_ui.add_space(8.0);

                // Text Edit
                let text_edit_w = bar_ui.available_width() - 48.0;
                let text_edit = egui::TextEdit::singleline(&mut self.message_input)
                    .hint_text("Nhập tin nhắn...")
                    .desired_width(text_edit_w);
                let edit_resp = bar_ui.add(text_edit);

                if edit_resp.changed() {
                    if !self.message_input.is_empty() {
                        *action = Some(ChatAction::TypingStarted);
                    } else {
                        *action = Some(ChatAction::TypingStopped);
                    }
                }

                bar_ui.add_space(8.0);

                // Send Button
                let can_send = !self.message_input.trim().is_empty();
                let (send_rect, send_resp) = bar_ui.allocate_exact_size(
                    Vec2::new(38.0, 38.0),
                    if can_send { Sense::click() } else { Sense::hover() },
                );

                let send_bg = if !can_send {
                    Color32::from_rgb(236, 238, 242) // #ECEEF2
                } else if send_resp.hovered() {
                    Color32::from_rgb(30, 64, 175)
                } else {
                    Color32::from_rgb(37, 99, 235) // #2563EB
                };

                bar_ui.painter().rect_filled(send_rect, CornerRadius::same(8), send_bg);
                bar_ui.painter().text(
                    send_rect.center(),
                    Align2::CENTER_CENTER,
                    "➤",
                    egui::FontId::proportional(15.0),
                    if can_send { Color32::WHITE } else { Color32::from_rgb(161, 161, 170) },
                );

                let should_send = (send_resp.clicked()
                    || (edit_resp.lost_focus() && bar_ui.input(|i| i.key_pressed(egui::Key::Enter))))
                    && can_send;

                if should_send {
                    let text = self.message_input.trim().to_string();
                    self.message_input.clear();
                    self.is_emoji_picker_open = false;
                    *action = Some(ChatAction::SendMessage(text));
                }
            }
        }
    }

    // ==========================================
    // PARTNER INFO SIDEBAR (Right 300px)
    // ==========================================
    fn render_partner_info_sidebar(&mut self, ui: &mut Ui, friend: &FriendItem) {
        // Background: #FFFFFF
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(255, 255, 255));

        // 1. Header (height 54px)
        let header_height = 54.0;
        let (header_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), header_height), Sense::hover());
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let mut header_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(header_rect.shrink2(egui::vec2(16.0, 8.0)))
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
            ui.painter().text(close_rect.center(), Align2::CENTER_CENTER, "✕", egui::FontId::proportional(13.0), Color32::from_rgb(115, 115, 115));
            if close_resp.clicked() {
                self.is_partner_info_open = false;
            }
        });

        // 2. Content
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(20.0);

            // Partner Profile Card (Centered 64px avatar)
            ui.vertical_centered(|ui| {
                render_avatar(ui, &friend.username, 64.0);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(&friend.username)
                        .size(16.0)
                        .strong()
                        .color(Color32::from_rgb(23, 23, 23)),
                );
                ui.label(
                    RichText::new("Tài khoản bạn bè")
                        .size(12.5)
                        .color(Color32::from_rgb(115, 115, 115)),
                );
            });

            ui.add_space(20.0);

            // Group: Tài khoản
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(RichText::new("Tài khoản").size(13.5).strong().color(Color32::from_rgb(23, 23, 23)));
            });
            ui.add_space(6.0);

            render_info_row(ui, "Tên người dùng", &friend.username, false);
            render_info_row(ui, "Mã định danh", &friend.id.to_string(), true);

            ui.add_space(16.0);

            // Group: Mã hóa đầu cuối
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(RichText::new("Mã hóa đầu cuối").size(13.5).strong().color(Color32::from_rgb(23, 23, 23)));
            });
            ui.add_space(6.0);

            render_info_row(ui, "Giao thức", "ECDH P-256 / AES-256-GCM", false);
            render_info_row(ui, "Chữ ký số", "ECDSA P-256 / SHA-256", false);

            let has_keys = self.cached_keys.contains_key(&friend.id);
            render_status_tag_row(ui, "Trạng thái khóa", if has_keys { "Đã kích hoạt trên thiết bị" } else { "Đang tải" });

            if let Some((_, fp, _)) = self.cached_keys.get(&friend.id) {
                let fp_b64 = URL_SAFE_NO_PAD.encode(fp);
                render_info_row(ui, "Vân tay khóa", &fp_b64, true);
            }
        });
    }

    // ==========================================
    // SETTINGS VIEW (Center Area Overlay)
    // ==========================================
    fn render_settings_view(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(255, 255, 255));

        // 1. Settings Header (height 54px)
        let header_height = 54.0;
        let (header_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), header_height), Sense::hover());
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let mut header_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(header_rect.shrink2(egui::vec2(16.0, 8.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );

        // Back button
        let (back_rect, back_resp) = header_ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click());
        if back_resp.hovered() {
            header_ui.painter().rect_filled(back_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        header_ui.painter().text(back_rect.center(), Align2::CENTER_CENTER, "←", egui::FontId::proportional(16.0), Color32::from_rgb(23, 23, 23));
        if back_resp.clicked() {
            self.is_settings_open = false;
        }

        header_ui.add_space(8.0);
        header_ui.label(
            RichText::new("Cài đặt")
                .size(15.5)
                .strong()
                .color(Color32::from_rgb(23, 23, 23)),
        );

        // 2. Settings Content matching web .settings-content
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(20.0);

            let max_w = 540.0_f32.min(ui.available_width() - 32.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.allocate_ui_with_layout(Vec2::new(max_w, ui.available_height()), Layout::top_down(Align::Min), |ui| {
                    // Profile Card
                    let card_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(max_w, 80.0));
                    ui.painter().rect_filled(card_rect, CornerRadius::same(10), Color32::from_rgb(247, 248, 250)); // #F7F8FA
                    ui.painter().rect_stroke(
                        card_rect,
                        CornerRadius::same(10),
                        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                        StrokeKind::Inside,
                    );

                    let mut card_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(card_rect.shrink2(egui::vec2(16.0, 12.0)))
                            .layout(Layout::left_to_right(Align::Center)),
                    );

                    render_avatar(&mut card_ui, &self.current_username, 54.0);
                    card_ui.add_space(14.0);
                    card_ui.vertical(|ui| {
                        ui.label(RichText::new(&self.current_username).size(16.0).strong().color(Color32::from_rgb(23, 23, 23)));
                        ui.label(RichText::new("Tài khoản cá nhân").size(12.5).color(Color32::from_rgb(115, 115, 115)));
                    });

                    ui.add_space(92.0);

                    // Group: Tài khoản
                    ui.label(RichText::new("Tài khoản").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                    ui.add_space(6.0);
                    render_settings_row(ui, max_w, "Tên người dùng", &self.current_username, false);
                    render_settings_row(ui, max_w, "Mã định danh", &self.current_user_id.to_string(), true);

                    ui.add_space(16.0);

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

                    ui.add_space(16.0);

                    // Group: Máy chủ
                    ui.label(RichText::new("Máy chủ & Kết nối").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                    ui.add_space(6.0);

                    let (srv_rect, _) = ui.allocate_exact_size(Vec2::new(max_w, 48.0), Sense::hover());
                    ui.painter().rect_filled(srv_rect, CornerRadius::same(8), Color32::from_rgb(247, 248, 250));
                    ui.painter().rect_stroke(
                        srv_rect,
                        CornerRadius::same(8),
                        Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                        StrokeKind::Inside,
                    );

                    let mut srv_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(srv_rect.shrink2(egui::vec2(12.0, 6.0)))
                            .layout(Layout::left_to_right(Align::Center)),
                    );

                    srv_ui.label(RichText::new("URL Máy chủ:").size(13.0).color(Color32::from_rgb(115, 115, 115)));
                    srv_ui.add_space(8.0);
                    let inp = srv_ui.add(egui::TextEdit::singleline(&mut self.server_url_input).desired_width(max_w - 200.0));
                    srv_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(RichText::new("Lưu").size(12.0)).clicked() || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                            *action = Some(ChatAction::ChangeServerUrl(self.server_url_input.trim().to_string()));
                        }
                    });

                    ui.add_space(16.0);

                    // Group: Phiên đăng nhập
                    ui.label(RichText::new("Phiên đăng nhập").size(14.0).strong().color(Color32::from_rgb(23, 23, 23)));
                    ui.add_space(8.0);

                    let (logout_btn_rect, logout_btn_resp) = ui.allocate_exact_size(Vec2::new(120.0, 38.0), Sense::click());
                    let logout_bg = if logout_btn_resp.hovered() { Color32::from_rgb(254, 202, 202) } else { Color32::from_rgb(252, 232, 230) };
                    ui.painter().rect_filled(logout_btn_rect, CornerRadius::same(8), logout_bg);
                    ui.painter().text(
                        logout_btn_rect.center(),
                        Align2::CENTER_CENTER,
                        "Đăng xuất",
                        egui::FontId::proportional(13.5),
                        Color32::from_rgb(220, 38, 38), // Red #DC2626
                    );

                    if logout_btn_resp.clicked() {
                        *action = Some(ChatAction::Logout);
                    }
                });
            });
        });
    }
}

// ==========================================
// HELPER DRAW FUNCTIONS
// ==========================================

fn render_message_bubble(ui: &mut Ui, msg: &DecryptedMessage) {
    let max_bubble_width = (ui.available_width() * 0.65).max(180.0);

    ui.horizontal(|ui| {
        if msg.is_me {
            // Sent: Align Right, Blue bubble #2563EB, White text, Corner: 14, 14, 14, 2
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(37, 99, 235)) // #2563EB
                        .corner_radius(CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 14,
                            se: 2,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 9))
                        .show(ui, |ui| {
                            ui.set_max_width(max_bubble_width);
                            ui.label(RichText::new(&msg.text).size(13.5).color(Color32::WHITE));
                        });

                    // Timestamp below bubble
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.label(RichText::new(&msg.time_str).size(10.5).color(Color32::from_rgb(115, 115, 115)));
                    });
                });
            });
        } else {
            // Received: Align Left, Light Gray bubble #F1F3F5, Dark text #171717, Corner: 14, 14, 2, 14
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(241, 243, 245)) // #F1F3F5
                        .corner_radius(CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 2,
                            se: 14,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 9))
                        .show(ui, |ui| {
                            ui.set_max_width(max_bubble_width);
                            ui.label(RichText::new(&msg.text).size(13.5).color(Color32::from_rgb(23, 23, 23)));
                        });

                    // Timestamp below bubble
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.label(RichText::new(&msg.time_str).size(10.5).color(Color32::from_rgb(115, 115, 115)));
                    });
                });
            });
        }
    });

    ui.add_space(4.0);
}

fn render_info_row(ui: &mut Ui, label: &str, value: &str, is_mono: bool) {
    let row_w = ui.available_width() - 32.0;
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(row_w, 40.0), Sense::hover());
    ui.painter().rect_filled(row_rect, CornerRadius::same(6), Color32::WHITE);
    ui.painter().rect_stroke(
        row_rect,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, Color32::from_rgb(236, 238, 242)),
        StrokeKind::Inside,
    );

    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row_rect.shrink2(egui::vec2(12.0, 4.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );

    row_ui.label(RichText::new(label).size(12.5).color(Color32::from_rgb(115, 115, 115)));
    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let text = if is_mono {
            RichText::new(value).size(11.0).monospace().color(Color32::from_rgb(23, 23, 23))
        } else {
            RichText::new(value).size(12.5).strong().color(Color32::from_rgb(23, 23, 23))
        };
        ui.label(text);
    });
}

fn render_status_tag_row(ui: &mut Ui, label: &str, tag_text: &str) {
    let row_w = ui.available_width() - 32.0;
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(row_w, 40.0), Sense::hover());
    ui.painter().rect_filled(row_rect, CornerRadius::same(6), Color32::WHITE);
    ui.painter().rect_stroke(
        row_rect,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, Color32::from_rgb(236, 238, 242)),
        StrokeKind::Inside,
    );

    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row_rect.shrink2(egui::vec2(12.0, 4.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );

    row_ui.label(RichText::new(label).size(12.5).color(Color32::from_rgb(115, 115, 115)));
    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let (tag_rect, _) = ui.allocate_exact_size(Vec2::new(145.0, 22.0), Sense::hover());
        ui.painter().rect_filled(tag_rect, CornerRadius::same(4), Color32::from_rgb(230, 244, 234));
        ui.painter().text(
            tag_rect.center(),
            Align2::CENTER_CENTER,
            tag_text,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(19, 115, 51),
        );
    });
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
        egui::UiBuilder::new()
            .max_rect(row_rect.shrink2(egui::vec2(14.0, 4.0)))
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
    ui.add_space(4.0);
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
        egui::UiBuilder::new()
            .max_rect(row_rect.shrink2(egui::vec2(14.0, 4.0)))
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
    ui.add_space(4.0);
}
