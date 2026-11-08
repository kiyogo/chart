use std::collections::{HashMap, HashSet};
use egui::{Align, Align2, Color32, CornerRadius, Layout, RichText, Sense, Stroke, Ui, Vec2};
use uuid::Uuid;
use chatw_core::{FriendItem, PendingRequestItem, UserSearchResult};

#[allow(dead_code)]
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
    pub cached_keys: HashMap<Uuid, ([u8; 65], [u8; 32], [u8; 65])>, // (ecdh_pub, fingerprint, ecdsa_pub)
    pub messages: HashMap<Uuid, Vec<DecryptedMessage>>,
    pub typing_users: HashSet<Uuid>,
    pub message_input: String,
    pub add_friend_input: String,
    pub search_results: Vec<UserSearchResult>,
    pub show_add_friend_modal: bool,
    pub status_message: Option<String>,
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
            show_add_friend_modal: false,
            status_message: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Option<ChatAction> {
        let mut triggered_action = None;

        let total_rect = ui.available_rect_before_wrap();
        let sidebar_width = 280.0_f32.min(total_rect.width() * 0.4);
        let chat_width = (total_rect.width() - sidebar_width - 1.0).max(100.0);

        ui.horizontal(|ui| {
            // LEFT SIDEBAR
            ui.allocate_ui_with_layout(
                Vec2::new(sidebar_width, total_rect.height()),
                Layout::top_down(Align::Min),
                |ui| {
                    self.render_sidebar(ui, &mut triggered_action);
                },
            );

            // SEPARATOR LINE
            let line_x = total_rect.min.x + sidebar_width;
            ui.painter().line_segment(
                [
                    egui::pos2(line_x, total_rect.min.y),
                    egui::pos2(line_x, total_rect.max.y),
                ],
                Stroke::new(1.0_f32, Color32::from_rgb(39, 39, 42)),
            );

            // RIGHT CHAT AREA
            ui.allocate_ui_with_layout(
                Vec2::new(chat_width, total_rect.height()),
                Layout::top_down(Align::Min),
                |ui| {
                    self.render_chat_area(ui, &mut triggered_action);
                },
            );
        });

        triggered_action
    }

    fn render_sidebar(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        // Background of sidebar #18181B
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(24, 24, 27));

        ui.add_space(8.0);

        // Header: User Info & Logout
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            let (av_rect, _) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::hover());
            ui.painter().circle_filled(av_rect.center(), 16.0, Color32::from_rgb(37, 99, 235));
            let initial = self.current_username.chars().next().unwrap_or('U').to_uppercase().to_string();
            ui.painter().text(
                av_rect.center(),
                Align2::CENTER_CENTER,
                &initial,
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&self.current_username).strong().size(13.0).color(Color32::WHITE));
                ui.label(RichText::new("Trực tuyến").size(10.0).color(Color32::from_rgb(34, 197, 94)));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(10.0);
                if ui.button(RichText::new("🚪").size(14.0)).on_hover_text("Đăng xuất").clicked() {
                    *action = Some(ChatAction::Logout);
                }
                if ui.button(RichText::new("➕").size(14.0)).on_hover_text("Thêm bạn bè").clicked() {
                    self.show_add_friend_modal = !self.show_add_friend_modal;
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();

        // Add Friend Popup / Section
        if self.show_add_friend_modal {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("Thêm bạn:").size(12.0).color(Color32::from_rgb(161, 161, 170)));
                let inp = ui.add(
                    egui::TextEdit::singleline(&mut self.add_friend_input)
                        .hint_text("Tên tài khoản")
                        .desired_width(120.0),
                );
                if (ui.button("Gửi").clicked()
                    || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                    && !self.add_friend_input.trim().is_empty()
                {
                    *action = Some(ChatAction::SendFriendRequest(self.add_friend_input.trim().to_string()));
                    self.add_friend_input.clear();
                    self.show_add_friend_modal = false;
                }
            });
            ui.add_space(6.0);
            ui.separator();
        }

        // Pending Requests
        if !self.pending_requests.is_empty() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(format!("Lời mời kết bạn ({})", self.pending_requests.len())).size(11.0).color(Color32::from_rgb(245, 158, 11)));
            });

            for req in &self.pending_requests {
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new(&req.sender_username).size(12.0).color(Color32::WHITE));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui.button(RichText::new("✕").size(10.0).color(Color32::from_rgb(239, 68, 68))).clicked() {
                            *action = Some(ChatAction::RejectFriendRequest(req.id));
                        }
                        if ui.button(RichText::new("✓").size(10.0).color(Color32::from_rgb(34, 197, 94))).clicked() {
                            *action = Some(ChatAction::AcceptFriendRequest(req.id));
                        }
                    });
                });
            }
            ui.add_space(4.0);
            ui.separator();
        }

        // Friends List
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new("BẠN BÈ").size(11.0).strong().color(Color32::from_rgb(113, 113, 122)));
        });

        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.friends.is_empty() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Chưa có bạn bè nào").size(12.0).color(Color32::from_rgb(113, 113, 122)));
                    ui.label(RichText::new("Bấm ➕ để kết bạn").size(11.0).color(Color32::from_rgb(82, 82, 91)));
                });
            } else {
                for friend in &self.friends {
                    let is_selected = self.selected_friend_id == Some(friend.id);
                    let row_width = ui.available_width() - 8.0;
                    let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(row_width, 44.0), Sense::click());

                    let bg = if is_selected {
                        Color32::from_rgb(39, 39, 42)
                    } else if resp.hovered() {
                        Color32::from_rgb(31, 31, 35)
                    } else {
                        Color32::TRANSPARENT
                    };

                    ui.painter().rect_filled(row_rect, 6.0, bg);

                    // Avatar
                    let av_center = egui::pos2(row_rect.left() + 22.0, row_rect.center().y);
                    ui.painter().circle_filled(av_center, 14.0, Color32::from_rgb(14, 165, 233));
                    let initial = friend.username.chars().next().unwrap_or('F').to_uppercase().to_string();
                    ui.painter().text(
                        av_center,
                        Align2::CENTER_CENTER,
                        &initial,
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );

                    // Name
                    ui.painter().text(
                        egui::pos2(row_rect.left() + 46.0, row_rect.center().y - 2.0),
                        Align2::LEFT_CENTER,
                        &friend.username,
                        egui::FontId::proportional(13.0),
                        if is_selected { Color32::WHITE } else { Color32::from_rgb(212, 212, 216) },
                    );

                    // Typing badge
                    if self.typing_users.contains(&friend.id) {
                        ui.painter().text(
                            egui::pos2(row_rect.right() - 10.0, row_rect.center().y),
                            Align2::RIGHT_CENTER,
                            "✍",
                            egui::FontId::proportional(11.0),
                            Color32::from_rgb(59, 130, 246),
                        );
                    }

                    if resp.clicked() {
                        *action = Some(ChatAction::SelectFriend(friend.id));
                    }
                }
            }
        });
    }

    fn render_chat_area(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        // Background of chat area #09090B
        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(9, 9, 11));

        let selected_friend = self.friends.iter().find(|f| Some(f.id) == self.selected_friend_id).cloned();

        match selected_friend {
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.35);
                    ui.label(RichText::new("🔒").size(40.0));
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Tin nhắn bảo mật mã hóa đầu cuối (E2EE)")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.label(
                        RichText::new("Chọn một người bạn ở danh sách bên trái để bắt đầu cuộc trò chuyện riêng tư.")
                            .size(13.0)
                            .color(Color32::from_rgb(113, 113, 122)),
                    );
                });
            }
            Some(friend) => {
                // Top Chat Header
                let header_height = 48.0;
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), header_height),
                    Layout::left_to_right(Align::Center),
                    |header_ui| {
                        let header_rect = header_ui.max_rect();
                        header_ui.painter().rect_filled(header_rect, 0.0, Color32::from_rgb(18, 18, 21));
                        header_ui.painter().line_segment(
                            [header_rect.left_bottom(), header_rect.right_bottom()],
                            Stroke::new(1.0_f32, Color32::from_rgb(39, 39, 42)),
                        );

                        header_ui.add_space(16.0);

                        let (av_rect, _) = header_ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                        header_ui.painter().circle_filled(av_rect.center(), 15.0, Color32::from_rgb(14, 165, 233));
                        let initial = friend.username.chars().next().unwrap_or('F').to_uppercase().to_string();
                        header_ui.painter().text(
                            av_rect.center(),
                            Align2::CENTER_CENTER,
                            &initial,
                            egui::FontId::proportional(13.0),
                            Color32::WHITE,
                        );

                        header_ui.add_space(10.0);
                        header_ui.vertical(|ui| {
                            ui.label(RichText::new(&friend.username).strong().size(14.0).color(Color32::WHITE));
                            ui.label(
                                RichText::new("🔒 E2EE P-256 + AES-GCM (Bảo mật tuyệt đối)")
                                    .size(10.0)
                                    .color(Color32::from_rgb(34, 197, 94)),
                            );
                        });
                    },
                );

                // Messages View
                let input_bar_height = 56.0;
                let messages_height = (ui.available_height() - input_bar_height).max(50.0);

                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), messages_height),
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
                                        ui.add_space(40.0);
                                        ui.label(
                                            RichText::new("Chưa có tin nhắn nào trong cuộc trò chuyện này.")
                                                .size(12.0)
                                                .color(Color32::from_rgb(82, 82, 91)),
                                        );
                                        ui.label(
                                            RichText::new("Gửi lời chào đầu tiên an toàn bằng mã hóa E2EE!")
                                                .size(11.0)
                                                .color(Color32::from_rgb(113, 113, 122)),
                                        );
                                    });
                                } else {
                                    for m in msg_list {
                                        render_message_bubble(ui, m);
                                    }
                                }

                                // Typing indicator
                                if self.typing_users.contains(&friend.id) {
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(16.0);
                                        ui.label(
                                            RichText::new(format!("{} đang soạn tin nhắn", friend.username))
                                                .size(11.0)
                                                .italics()
                                                .color(Color32::from_rgb(96, 165, 250)),
                                        );
                                    });
                                }

                                ui.add_space(10.0);
                            });
                    },
                );

                // Bottom Input Bar
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_space(12.0);

                    // Quick Emojis
                    for emoji in &["😀", "👍", "❤️", "🎉", "🔥"] {
                        if ui.button(RichText::new(*emoji).size(15.0)).clicked() {
                            self.message_input.push_str(emoji);
                        }
                    }

                    ui.add_space(6.0);

                    let text_edit = egui::TextEdit::singleline(&mut self.message_input)
                        .hint_text("Nhập tin nhắn bảo mật E2EE")
                        .desired_width((ui.available_width() - 80.0).max(100.0));
                    let edit_resp = ui.add(text_edit);

                    if edit_resp.changed() {
                        if !self.message_input.is_empty() {
                            *action = Some(ChatAction::TypingStarted);
                        } else {
                            *action = Some(ChatAction::TypingStopped);
                        }
                    }

                    let send_btn = egui::Button::new(RichText::new("Gửi 📤").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(37, 99, 235))
                        .min_size(Vec2::new(60.0, 32.0));

                    let should_send = (ui.add(send_btn).clicked()
                        || (edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                        && !self.message_input.trim().is_empty();

                    if should_send {
                        let text = self.message_input.trim().to_string();
                        self.message_input.clear();
                        *action = Some(ChatAction::SendMessage(text));
                    }
                });
            }
        }
    }
}

fn render_message_bubble(ui: &mut Ui, msg: &DecryptedMessage) {
    let max_bubble_width = (ui.available_width() * 0.65).max(180.0);

    ui.horizontal(|ui| {
        if msg.is_me {
            // Sent: Align Right, Blue bubble #2563EB
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_space(16.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(37, 99, 235))
                    .corner_radius(CornerRadius {
                        nw: 14,
                        ne: 14,
                        sw: 14,
                        se: 2,
                    })
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(max_bubble_width);
                        ui.label(RichText::new(&msg.text).size(13.5).color(Color32::WHITE));
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            ui.label(RichText::new(&msg.time_str).size(9.5).color(Color32::from_rgb(191, 219, 254)));
                        });
                    });
            });
        } else {
            // Received: Align Left, Dark gray bubble #27272A
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.add_space(16.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(39, 39, 42))
                    .corner_radius(CornerRadius {
                        nw: 14,
                        ne: 14,
                        sw: 2,
                        se: 14,
                    })
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(max_bubble_width);
                        ui.label(RichText::new(&msg.text).size(13.5).color(Color32::WHITE));
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            ui.label(RichText::new(&msg.time_str).size(9.5).color(Color32::from_rgb(161, 161, 170)));
                        });
                    });
            });
        }
    });

    ui.add_space(6.0);
}
