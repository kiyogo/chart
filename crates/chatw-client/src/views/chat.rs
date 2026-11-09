use std::collections::{HashMap, HashSet};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2,
};
use uuid::Uuid;
use chatw_core::{FriendItem, PendingRequestItem, UserSearchResult};
use crate::titlebar::{
    draw_smooth_polyline, draw_svg_logo_icon, font_mono, font_regular, font_semibold,
    push_arc_points,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarTab {
    Chats,
    Friends,
}

#[derive(Clone, Debug)]
pub struct DecryptedMessage {
    pub id: Uuid,
    #[allow(dead_code)]
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
    pub unread_counts: HashMap<Uuid, usize>,
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

// ==========================================
// EXACT WEB SVG ICONS (24x24 viewBox, stroke-width 2, round caps/joins)
// ==========================================

pub fn draw_svg_chat_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    draw_svg_logo_icon(painter, rect, color);
}

/// Web `<FriendsIcon />`:
/// `<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />`
/// `<circle cx="9" cy="7" r="4" />`
/// `<path d="M23 21v-2a4 4 0 0 0-3-3.87" />`
/// `<path d="M16 3.13a4 4 0 0 1 0 7.75" />`
pub fn draw_svg_friends_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);
    let pi = std::f32::consts::PI;

    // 1. Primary person head: circle(9, 7, r=4)
    painter.circle_stroke(pt(9.0, 7.0), 4.0 * s.x, stroke);

    // 2. Primary person body with smooth radius-4 shoulder arcs
    let mut body1 = Vec::with_capacity(20);
    body1.push(pt(1.0, 21.0));
    push_arc_points(&mut body1, pt(5.0, 19.0), vec2(4.0 * s.x, 4.0 * s.y), pi, pi * 1.5, 6);
    push_arc_points(&mut body1, pt(13.0, 19.0), vec2(4.0 * s.x, 4.0 * s.y), pi * 1.5, pi * 2.0, 6);
    body1.push(pt(17.0, 21.0));
    draw_smooth_polyline(painter, &body1, stroke, false);

    // 3. Secondary person head arc: center(15, 7), r=4, angle -75° to +75°
    let mut head2 = Vec::with_capacity(12);
    push_arc_points(&mut head2, pt(15.0, 7.0), vec2(4.0 * s.x, 4.0 * s.y), -pi * 0.42, pi * 0.42, 10);
    draw_smooth_polyline(painter, &head2, stroke, false);

    // 4. Secondary person shoulder arc: center(19, 19), r=4, angle -75° to 0° then down to (23, 21)
    let mut body2 = Vec::with_capacity(10);
    push_arc_points(&mut body2, pt(19.0, 19.0), vec2(4.0 * s.x, 4.0 * s.y), -pi * 0.42, 0.0, 6);
    body2.push(pt(23.0, 21.0));
    draw_smooth_polyline(painter, &body2, stroke, false);
}

/// Web `<SettingsIcon />`: Feather 8-tooth gear contour + inner circle(12,12, r=3)
pub fn draw_svg_settings_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let center = rect.center();
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    // Inner circle r=3.0 matching web SVG <circle cx="12" cy="12" r="3" />
    painter.circle_stroke(center, 3.0 * s.x, stroke);

    // Continuous 8-tooth gear contour matching Feather / Lucide settings icon
    let r_inner = 7.2 * s.x;
    let r_outer = 10.0 * s.x;
    let mut gear_pts = Vec::with_capacity(32);
    let num_teeth = 8;
    for i in 0..num_teeth {
        let base = (i as f32) * std::f32::consts::TAU / (num_teeth as f32);
        let step = std::f32::consts::TAU / (num_teeth as f32);
        let a0 = base - step * 0.18;
        let a1 = base - step * 0.10;
        let a2 = base + step * 0.10;
        let a3 = base + step * 0.18;
        gear_pts.push(center + vec2(a0.cos() * r_inner, a0.sin() * r_inner));
        gear_pts.push(center + vec2(a1.cos() * r_outer, a1.sin() * r_outer));
        gear_pts.push(center + vec2(a2.cos() * r_outer, a2.sin() * r_outer));
        gear_pts.push(center + vec2(a3.cos() * r_inner, a3.sin() * r_inner));
    }
    draw_smooth_polyline(painter, &gear_pts, stroke, true);
}

/// Web `<LogoutIcon />`:
/// `<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />`
/// `<polyline points="16 17 21 12 16 7" />`
/// `<line x1="21" y1="12" x2="9" y2="12" />`
pub fn draw_svg_logout_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let r2 = vec2(2.0 * s.x, 2.0 * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);
    let pi = std::f32::consts::PI;

    // 1. Door frame with smooth radius-2 corners
    let mut door = Vec::with_capacity(18);
    door.push(pt(9.0, 21.0));
    push_arc_points(&mut door, pt(5.0, 19.0), r2, pi * 0.5, pi, 5);
    push_arc_points(&mut door, pt(5.0, 5.0), r2, pi, pi * 1.5, 5);
    door.push(pt(9.0, 3.0));
    draw_smooth_polyline(painter, &door, stroke, false);

    // 2. Horizontal shaft (9,12 -> 21,12)
    draw_smooth_polyline(painter, &[pt(9.0, 12.0), pt(21.0, 12.0)], stroke, false);

    // 3. Arrow head (16,7 -> 21,12 -> 16,17)
    draw_smooth_polyline(painter, &[pt(16.0, 7.0), pt(21.0, 12.0), pt(16.0, 17.0)], stroke, false);
}

/// Web `<EmojiIcon />`
pub fn draw_svg_emoji_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    painter.circle_stroke(pt(12.0, 12.0), 10.0 * s.x, stroke);
    painter.circle_filled(pt(9.0, 9.0), (1.3 * s.x).max(1.2), color);
    painter.circle_filled(pt(15.0, 9.0), (1.3 * s.x).max(1.2), color);

    let mut smile = Vec::with_capacity(12);
    push_arc_points(
        &mut smile,
        pt(12.0, 12.5),
        vec2(4.2 * s.x, 3.5 * s.y),
        std::f32::consts::PI * 0.15,
        std::f32::consts::PI * 0.85,
        10,
    );
    draw_smooth_polyline(painter, &smile, stroke, false);
}

/// Web `<SendIcon />`
pub fn draw_svg_send_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    let poly = [
        pt(22.0, 2.0),
        pt(15.0, 22.0),
        pt(11.0, 13.0),
        pt(2.0, 9.0),
    ];
    draw_smooth_polyline(painter, &poly, stroke, true);
    draw_smooth_polyline(painter, &[pt(22.0, 2.0), pt(11.0, 13.0)], stroke, false);
}

/// Web `<BackIcon />`
pub fn draw_svg_back_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    draw_smooth_polyline(painter, &[pt(19.0, 12.0), pt(5.0, 12.0)], stroke, false);
    draw_smooth_polyline(painter, &[pt(12.0, 19.0), pt(5.0, 12.0), pt(12.0, 5.0)], stroke, false);
}

/// Web `<MenuIcon />`
pub fn draw_svg_menu_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    draw_smooth_polyline(painter, &[pt(3.0, 6.0), pt(21.0, 6.0)], stroke, false);
    draw_smooth_polyline(painter, &[pt(3.0, 12.0), pt(21.0, 12.0)], stroke, false);
    draw_smooth_polyline(painter, &[pt(3.0, 18.0), pt(21.0, 18.0)], stroke, false);
}

/// Web `<CloseIcon />`
pub fn draw_svg_close_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = rect.size() / 24.0;
    let pt = |x: f32, y: f32| pos2(rect.min.x + x * s.x, rect.min.y + y * s.y);
    let stroke = Stroke::new((2.0 * s.x).max(1.35), color);

    draw_smooth_polyline(painter, &[pt(18.0, 6.0), pt(6.0, 18.0)], stroke, false);
    draw_smooth_polyline(painter, &[pt(6.0, 6.0), pt(18.0, 18.0)], stroke, false);
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
            unread_counts: HashMap::new(),
            server_url_input: "http://127.0.0.1:8082".to_string(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Option<ChatAction> {
        let mut triggered_action = None;
        let active_friend = self.friends.iter().find(|f| Some(f.id) == self.selected_friend_id).cloned();

        let avail = ui.available_rect_before_wrap();
        if avail.width() < 100.0 || avail.height() < 100.0 {
            return None;
        }

        let total_w = avail.width();
        let total_h = avail.height();

        let sidebar_w = 320.0_f32;
        let show_right_sidebar = self.is_partner_info_open
            && active_friend.is_some()
            && !self.is_settings_open
            && total_w >= 1024.0;
        let partner_w = if show_right_sidebar { 380.0_f32 } else { 0.0_f32 };
        let center_w = (total_w - sidebar_w - partner_w).max(200.0);

        let sidebar_rect = Rect::from_min_size(avail.min, vec2(sidebar_w, total_h));
        let center_rect = Rect::from_min_size(pos2(sidebar_rect.right(), avail.top()), vec2(center_w, total_h));
        let partner_rect = if show_right_sidebar {
            Rect::from_min_size(pos2(center_rect.right(), avail.top()), vec2(partner_w, total_h))
        } else {
            Rect::ZERO
        };

        // 1. RENDER LEFT SIDEBAR (320px, #F7F8FA background)
        let mut sidebar_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(sidebar_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        self.render_sidebar_content(&mut sidebar_ui, &mut triggered_action);

        // 1px solid #E5E7EB border between sidebar and center view
        ui.painter().line_segment(
            [sidebar_rect.right_top(), sidebar_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        // 2. RENDER CENTER MAIN VIEW
        let mut center_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(center_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        if self.is_settings_open {
            self.render_settings_view(&mut center_ui, &mut triggered_action);
        } else {
            self.render_chat_area(&mut center_ui, &mut triggered_action, active_friend.as_ref());
        }

        // 3. RENDER RIGHT PARTNER INFO SIDEBAR (if shown)
        if show_right_sidebar {
            if let Some(ref friend) = active_friend {
                ui.painter().line_segment(
                    [partner_rect.left_top(), partner_rect.left_bottom()],
                    Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
                );
                let mut partner_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(partner_rect)
                        .layout(Layout::top_down(Align::Min)),
                );
                self.render_partner_info_sidebar(&mut partner_ui, friend);
            }
        }

        triggered_action
    }

    // ==========================================
    // SIDEBAR CONTENT RENDERING (320px)
    // ==========================================
    pub fn render_sidebar_content(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::from_rgb(247, 248, 250));

        // .sidebar-header: padding 16px, height 68px (36px buttons + 32px padding), NO bottom border
        let header_height = 68.0;
        let (header_rect, _) = ui.allocate_exact_size(Vec2::new(total_rect.width(), header_height), Sense::hover());

        // Exact coordinates for 4 action buttons matching web CSS (.sidebar-actions { gap: 6px; }, .icon-btn { width: 36px; height: 36px; }):
        // Total width = 4 * 36 + 3 * 6 = 162px. Right padding = 16px.
        // Logout right edge is at 304px, exactly 16px away from 320px sidebar border.
        let btn_size = vec2(36.0, 36.0);
        let btn_y = header_rect.center().y - 18.0;
        let right_edge = header_rect.right() - 16.0;
        let gap = 6.0;

        let logout_rect = Rect::from_min_size(pos2(right_edge - 36.0, btn_y), btn_size);
        let set_rect = Rect::from_min_size(pos2(logout_rect.left() - gap - 36.0, btn_y), btn_size);
        let fr_rect = Rect::from_min_size(pos2(set_rect.left() - gap - 36.0, btn_y), btn_size);
        let ch_rect = Rect::from_min_size(pos2(fr_rect.left() - gap - 36.0, btn_y), btn_size);

        // 1. Left brand (.app-brand: Avatar 28px + 8px gap + 16px semibold username)
        let avatar_rect = Rect::from_center_size(
            pos2(header_rect.left() + 16.0 + 14.0, header_rect.center().y),
            vec2(28.0, 28.0),
        );
        paint_avatar_in_rect(ui.painter(), avatar_rect, &self.current_username);

        let max_name_w = (ch_rect.left() - avatar_rect.right() - 14.0).max(40.0);
        let name_galley = ui.painter().layout_no_wrap(
            self.current_username.clone(),
            font_regular(15.0),
            Color32::from_rgb(23, 23, 23),
        );
        let name_pos = pos2(
            avatar_rect.right() + 8.0,
            header_rect.center().y - name_galley.size().y * 0.5,
        );
        let name_clip = Rect::from_min_size(
            pos2(name_pos.x, header_rect.top()),
            vec2(max_name_w, header_height),
        );
        ui.painter().with_clip_rect(name_clip).galley(
            name_pos,
            name_galley,
            Color32::from_rgb(23, 23, 23),
        );

        // Button 1: Chats Tab (<LogoIcon size={18} />)
        let ch_resp = ui.interact(ch_rect, ui.id().with("sb_btn_chats"), Sense::click());
        let is_ch_active = self.sidebar_tab == SidebarTab::Chats && !self.is_settings_open;
        if is_ch_active || ch_resp.hovered() {
            ui.painter().rect_filled(ch_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242)); // #ECEEF2
        }
        let ch_color = if is_ch_active {
            Color32::from_rgb(37, 99, 235) // #2563EB
        } else if ch_resp.hovered() {
            Color32::from_rgb(23, 23, 23)
        } else {
            Color32::from_rgb(115, 115, 115) // #737373
        };
        draw_svg_chat_icon(ui.painter(), Rect::from_center_size(ch_rect.center(), vec2(18.0, 18.0)), ch_color);

        let total_unread: usize = self.unread_counts.values().sum();
        if total_unread > 0 {
            let badge_center = pos2(ch_rect.right() - 8.5, ch_rect.top() + 8.5);
            let border_col = if is_ch_active || ch_resp.hovered() {
                Color32::from_rgb(236, 238, 242)
            } else {
                Color32::from_rgb(247, 248, 250)
            };
            ui.painter().circle_filled(badge_center, 8.5, border_col);
            ui.painter().circle_filled(badge_center, 6.5, Color32::from_rgb(37, 99, 235));
            let badge_text = if total_unread > 99 { "99+".to_string() } else { format!("{}", total_unread) };
            ui.painter().text(
                badge_center,
                Align2::CENTER_CENTER,
                &badge_text,
                font_semibold(9.5),
                Color32::WHITE,
            );
        }

        if ch_resp.clicked() {
            self.sidebar_tab = SidebarTab::Chats;
            self.is_settings_open = false;
        }

        // Button 2: Friends Tab (<FriendsIcon /> 18x18 + .tab-badge)
        let fr_resp = ui.interact(fr_rect, ui.id().with("sb_btn_friends"), Sense::click());
        let is_fr_active = self.sidebar_tab == SidebarTab::Friends && !self.is_settings_open;
        if is_fr_active || fr_resp.hovered() {
            ui.painter().rect_filled(fr_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let fr_color = if is_fr_active {
            Color32::from_rgb(37, 99, 235)
        } else if fr_resp.hovered() {
            Color32::from_rgb(23, 23, 23)
        } else {
            Color32::from_rgb(115, 115, 115)
        };
        draw_svg_friends_icon(ui.painter(), Rect::from_center_size(fr_rect.center(), vec2(18.0, 18.0)), fr_color);

        if !self.pending_requests.is_empty() {
            let badge_center = pos2(fr_rect.right() - 8.5, fr_rect.top() + 8.5);
            let border_col = if is_fr_active || fr_resp.hovered() {
                Color32::from_rgb(236, 238, 242)
            } else {
                Color32::from_rgb(247, 248, 250)
            };
            ui.painter().circle_filled(badge_center, 8.5, border_col);
            ui.painter().circle_filled(badge_center, 6.5, Color32::from_rgb(37, 99, 235));
            let badge_text = if self.pending_requests.len() > 99 { "99+".to_string() } else { format!("{}", self.pending_requests.len()) };
            ui.painter().text(
                badge_center,
                Align2::CENTER_CENTER,
                &badge_text,
                font_semibold(9.5),
                Color32::WHITE,
            );
        }
        if fr_resp.clicked() {
            self.sidebar_tab = SidebarTab::Friends;
            self.is_settings_open = false;
        }

        // Button 3: Settings (<SettingsIcon /> 18x18)
        let set_resp = ui.interact(set_rect, ui.id().with("sb_btn_settings"), Sense::click());
        let is_set_active = self.is_settings_open;
        if is_set_active || set_resp.hovered() {
            ui.painter().rect_filled(set_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let set_color = if is_set_active {
            Color32::from_rgb(37, 99, 235)
        } else if set_resp.hovered() {
            Color32::from_rgb(23, 23, 23)
        } else {
            Color32::from_rgb(115, 115, 115)
        };
        draw_svg_settings_icon(ui.painter(), Rect::from_center_size(set_rect.center(), vec2(18.0, 18.0)), set_color);
        if set_resp.clicked() {
            let next_open = !self.is_settings_open;
            self.is_settings_open = next_open;
            if next_open {
                self.selected_friend_id = None;
            }
        }

        // Button 4: Logout (.icon-btn.btn-danger: color ALWAYS #DC2626, hover bg #FCE8E6)
        let logout_resp = ui.interact(logout_rect, ui.id().with("sb_btn_logout"), Sense::click());
        if logout_resp.hovered() {
            ui.painter().rect_filled(logout_rect, CornerRadius::same(6), Color32::from_rgb(252, 232, 230)); // #FCE8E6
        }
        draw_svg_logout_icon(
            ui.painter(),
            Rect::from_center_size(logout_rect.center(), vec2(18.0, 18.0)),
            Color32::from_rgb(220, 38, 38), // #DC2626
        );
        if logout_resp.clicked() {
            *action = Some(ChatAction::Logout);
        }

        // 2. Tab Body
        match self.sidebar_tab {
            SidebarTab::Chats => {
                self.render_chats_tab(ui, action);
            }
            SidebarTab::Friends => {
                self.render_friends_tab(ui, action);
            }
        }
    }

    // ==========================================
    // TAB 1: CHATS LIST (Matches Chat.tsx lines 875-910)
    // ==========================================
    fn render_chats_tab(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        if self.friends.is_empty() {
            let avail = ui.available_rect_before_wrap();
            ui.painter().text(
                avail.center(),
                Align2::CENTER_CENTER,
                "Chưa có bạn bè nào",
                font_regular(14.0),
                Color32::from_rgb(115, 115, 115),
            );
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let row_w = ui.available_width() - 16.0;
                for friend in &self.friends {
                    let is_selected = Some(friend.id) == self.selected_friend_id && !self.is_settings_open;
                    let unread = self.unread_counts.get(&friend.id).copied().unwrap_or(0);
                    let (outer_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::hover());
                    let item_rect = Rect::from_center_size(outer_rect.center(), vec2(row_w, 54.0));
                    let resp = ui.interact(item_rect, ui.id().with(("chat_item", friend.id)), Sense::click());

                    let bg = if is_selected {
                        Color32::from_rgb(230, 237, 254) // #E6EDFE (.chat-item.selected)
                    } else if resp.hovered() {
                        Color32::from_rgb(236, 238, 242) // #ECEEF2 (.chat-item:hover)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(item_rect, CornerRadius::same(8), bg);

                    // Avatar 38px
                    let av_rect = Rect::from_center_size(
                        pos2(item_rect.left() + 10.0 + 19.0, item_rect.center().y),
                        vec2(38.0, 38.0),
                    );
                    paint_avatar_in_rect(ui.painter(), av_rect, &friend.username);

                    let text_left = av_rect.right() + 12.0;
                    ui.painter().text(
                        pos2(text_left, item_rect.center().y - 9.0),
                        Align2::LEFT_CENTER,
                        &friend.username,
                        font_regular(14.0),
                        Color32::from_rgb(23, 23, 23),
                    );

                    // Unread badge pill on item header matching web .chat-item-badge
                    if unread > 0 {
                        let badge_text = if unread > 99 { "99+".to_string() } else { format!("{}", unread) };
                        let galley = ui.painter().layout_no_wrap(badge_text, font_semibold(11.0), Color32::WHITE);
                        let badge_w = (galley.size().x + 10.0).max(18.0);
                        let badge_rect = Rect::from_center_size(
                            pos2(item_rect.right() - 12.0 - badge_w * 0.5, item_rect.center().y - 9.0),
                            vec2(badge_w, 18.0),
                        );
                        ui.painter().rect_filled(badge_rect, CornerRadius::same(9), Color32::from_rgb(37, 99, 235));
                        ui.painter().galley(
                            pos2(badge_rect.center().x - galley.size().x * 0.5, badge_rect.center().y - galley.size().y * 0.5),
                            galley,
                            Color32::WHITE,
                        );
                    }

                    let preview_text = if let Some(msg) = self.messages.get(&friend.id).and_then(|m| m.last()) {
                        msg.text.chars().take(28).collect::<String>()
                    } else {
                        "Bắt đầu trò chuyện".to_string()
                    };

                    let preview_col = if unread > 0 {
                        Color32::from_rgb(23, 23, 23)
                    } else {
                        Color32::from_rgb(115, 115, 115)
                    };
                    ui.painter().text(
                        pos2(text_left, item_rect.center().y + 10.0),
                        Align2::LEFT_CENTER,
                        &preview_text,
                        font_regular(12.0),
                        preview_col,
                    );

                    if resp.clicked() {
                        self.unread_counts.remove(&friend.id);
                        *action = Some(ChatAction::SelectFriend(friend.id));
                        self.is_settings_open = false;
                    }
                }
            });
    }

    // ==========================================
    // TAB 2: FRIENDS & REQUESTS (Matches Chat.tsx lines 773-872)
    // ==========================================
    fn render_friends_tab(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        // .add-friend-container: padding 0 16px 12px
        let total_w = ui.available_width();
        let (form_outer, _) = ui.allocate_exact_size(Vec2::new(total_w, 48.0), Sense::hover());

        let btn_w = 68.0;
        let gap = 8.0;
        let inp_w = (total_w - 32.0 - btn_w - gap).max(100.0);

        let inp_rect = Rect::from_min_size(
            pos2(form_outer.left() + 16.0, form_outer.top()),
            vec2(inp_w, 38.0),
        );
        let btn_rect = Rect::from_min_size(
            pos2(inp_rect.right() + gap, form_outer.top() + 4.0),
            vec2(btn_w, 30.0),
        );

        // Input background (#FFFFFF inside #F7F8FA sidebar for clear contrast)
        ui.painter().rect_filled(inp_rect, CornerRadius::same(8), Color32::WHITE);
        let mut inp_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(inp_rect.shrink2(vec2(14.0, 4.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let inp = inp_ui.add(
            egui::TextEdit::singleline(&mut self.add_friend_input)
                .font(font_regular(14.0))
                .hint_text(
                    RichText::new("Tên tài khoản")
                        .font(font_regular(14.0))
                        .color(Color32::from_rgb(115, 115, 115)),
                )
                .frame(false)
                .desired_width(inp_w - 28.0),
        );

        let border_col = if inp.has_focus() {
            Color32::from_rgb(37, 99, 235) // #2563EB
        } else {
            Color32::from_rgb(229, 231, 235) // #E5E7EB
        };
        ui.painter().rect_stroke(
            inp_rect,
            CornerRadius::same(8),
            Stroke::new(1.0_f32, border_col),
            StrokeKind::Inside,
        );

        // Button "Kết bạn" (.btn-small-primary)
        let has_input = !self.add_friend_input.trim().is_empty();
        let btn_resp = ui.interact(
            btn_rect,
            ui.id().with("add_friend_submit_btn"),
            if has_input { Sense::click() } else { Sense::hover() },
        );
        let btn_bg = if !has_input {
            Color32::from_rgb(147, 180, 248) // disabled opacity 0.5 of #2563EB
        } else if btn_resp.hovered() {
            Color32::from_rgb(29, 78, 216)
        } else {
            Color32::from_rgb(37, 99, 235)
        };
        ui.painter().rect_filled(btn_rect, CornerRadius::same(6), btn_bg);
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            "Kết bạn",
            font_regular(12.0),
            Color32::WHITE,
        );

        let should_send = (btn_resp.clicked()
            || (inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
            && has_input;

        if should_send {
            *action = Some(ChatAction::SendFriendRequest(
                self.add_friend_input.trim().to_string(),
            ));
            self.add_friend_input.clear();
        }

        // Status badge (.status-badge)
        if let Some(ref status) = self.status_message {
            let is_err = status.starts_with("Lỗi") || status.starts_with("Không");
            let (bg, fg) = if is_err {
                (Color32::from_rgb(252, 232, 230), Color32::from_rgb(197, 34, 31))
            } else {
                (Color32::from_rgb(230, 244, 234), Color32::from_rgb(19, 115, 51))
            };
            let (st_outer, _) = ui.allocate_exact_size(Vec2::new(total_w, 34.0), Sense::hover());
            let st_rect = Rect::from_min_size(pos2(st_outer.left() + 16.0, st_outer.top()), vec2(total_w - 32.0, 28.0));
            ui.painter().rect_filled(st_rect, CornerRadius::same(6), bg);
            ui.painter().text(st_rect.center(), Align2::CENTER_CENTER, status, font_regular(12.0), fg);
        }

        // Scrollable list of pending requests (or empty state "Không có lời mời nào")
        if self.pending_requests.is_empty() {
            let avail = ui.available_rect_before_wrap();
            ui.painter().text(
                avail.center(),
                Align2::CENTER_CENTER,
                "Không có lời mời nào",
                font_regular(14.0),
                Color32::from_rgb(115, 115, 115),
            );
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let card_w = ui.available_width() - 16.0;
                for req in &self.pending_requests {
                    let (outer, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::hover());
                    let card_rect = Rect::from_center_size(outer.center(), vec2(card_w, 52.0));
                    ui.painter().rect_filled(card_rect, CornerRadius::same(8), Color32::WHITE);

                    let av_rect = Rect::from_center_size(
                        pos2(card_rect.left() + 12.0 + 16.0, card_rect.center().y),
                        vec2(32.0, 32.0),
                    );
                    paint_avatar_in_rect(ui.painter(), av_rect, &req.sender_username);

                    ui.painter().text(
                        pos2(av_rect.right() + 10.0, card_rect.center().y),
                        Align2::LEFT_CENTER,
                        &req.sender_username,
                        font_regular(14.0),
                        Color32::from_rgb(23, 23, 23),
                    );

                    // "Chấp nhận" button (.btn-small-primary)
                    let acc_rect = Rect::from_center_size(
                        pos2(card_rect.right() - 12.0 - 40.0, card_rect.center().y),
                        vec2(80.0, 28.0),
                    );
                    let acc_resp = ui.interact(acc_rect, ui.id().with(("accept_req", req.id)), Sense::click());
                    let acc_bg = if acc_resp.hovered() {
                        Color32::from_rgb(29, 78, 216)
                    } else {
                        Color32::from_rgb(37, 99, 235)
                    };
                    ui.painter().rect_filled(acc_rect, CornerRadius::same(6), acc_bg);
                    ui.painter().text(
                        acc_rect.center(),
                        Align2::CENTER_CENTER,
                        "Chấp nhận",
                        font_regular(12.0),
                        Color32::WHITE,
                    );
                    if acc_resp.clicked() {
                        *action = Some(ChatAction::AcceptFriendRequest(req.id));
                    }
                }
            });
    }

    // ==========================================
    // CENTER CHAT AREA (Matches Chat.tsx lines 1014-1114)
    // ==========================================
    pub fn render_chat_area(
        &mut self,
        ui: &mut Ui,
        action: &mut Option<ChatAction>,
        active_friend: Option<&FriendItem>,
    ) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::WHITE);

        let Some(friend) = active_friend else {
            ui.painter().text(
                total_rect.center(),
                Align2::CENTER_CENTER,
                "Chọn một cuộc trò chuyện để bắt đầu",
                font_regular(14.0),
                Color32::from_rgb(115, 115, 115),
            );
            return;
        };

        // 1. TOP HEADER (.chat-header: height 54px, border-bottom 1px solid #E5E7EB)
        let header_height = 54.0;
        let input_bar_height = 60.0;
        let header_rect = Rect::from_min_size(total_rect.min, Vec2::new(total_rect.width(), header_height));
        ui.painter().rect_filled(header_rect, 0.0, Color32::WHITE);
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let av_rect = Rect::from_center_size(
            pos2(header_rect.left() + 20.0 + 17.0, header_rect.center().y),
            vec2(34.0, 34.0),
        );
        paint_avatar_in_rect(ui.painter(), av_rect, &friend.username);

        ui.painter().text(
            pos2(av_rect.right() + 12.0, header_rect.center().y),
            Align2::LEFT_CENTER,
            &friend.username,
            font_regular(15.0),
            Color32::from_rgb(23, 23, 23),
        );

        // Right MenuIcon toggle button (36x36)
        let menu_rect = Rect::from_center_size(
            pos2(header_rect.right() - 20.0 - 18.0, header_rect.center().y),
            vec2(36.0, 36.0),
        );
        let menu_resp = ui.interact(menu_rect, ui.id().with("chat_header_menu_btn"), Sense::click());
        if self.is_partner_info_open || menu_resp.hovered() {
            ui.painter().rect_filled(menu_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let menu_color = if self.is_partner_info_open {
            Color32::from_rgb(37, 99, 235)
        } else if menu_resp.hovered() {
            Color32::from_rgb(23, 23, 23)
        } else {
            Color32::from_rgb(115, 115, 115)
        };
        draw_svg_menu_icon(
            ui.painter(),
            Rect::from_center_size(menu_rect.center(), vec2(20.0, 20.0)),
            menu_color,
        );
        if menu_resp.clicked() {
            self.is_partner_info_open = !self.is_partner_info_open;
        }

        // 2. BOTTOM INPUT BAR (height 60px, border-top 1px solid #E5E7EB)
        let input_bar_rect = Rect::from_min_size(
            pos2(total_rect.min.x, total_rect.max.y - input_bar_height),
            Vec2::new(total_rect.width(), input_bar_height),
        );
        ui.painter().rect_filled(input_bar_rect, 0.0, Color32::WHITE);
        ui.painter().line_segment(
            [input_bar_rect.left_top(), input_bar_rect.right_top()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let emoji_btn_rect = Rect::from_center_size(
            pos2(input_bar_rect.left() + 20.0 + 18.0, input_bar_rect.center().y),
            vec2(36.0, 36.0),
        );
        let emoji_resp = ui.interact(emoji_btn_rect, ui.id().with("chat_emoji_btn"), Sense::click());
        if emoji_resp.hovered() {
            ui.painter().rect_filled(emoji_btn_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        draw_svg_emoji_icon(
            ui.painter(),
            Rect::from_center_size(emoji_btn_rect.center(), vec2(20.0, 20.0)),
            Color32::from_rgb(115, 115, 115),
        );
        if emoji_resp.clicked() {
            self.is_emoji_picker_open = !self.is_emoji_picker_open;
        }

        let send_btn_rect = Rect::from_center_size(
            pos2(input_bar_rect.right() - 20.0 - 20.0, input_bar_rect.center().y),
            vec2(40.0, 40.0),
        );

        let text_box_rect = Rect::from_min_max(
            pos2(emoji_btn_rect.right() + 10.0, input_bar_rect.center().y - 20.0),
            pos2(send_btn_rect.left() - 10.0, input_bar_rect.center().y + 20.0),
        );
        ui.painter().rect_filled(text_box_rect, CornerRadius::same(8), Color32::from_rgb(247, 248, 250));

        let mut text_child = ui.new_child(
            UiBuilder::new()
                .max_rect(text_box_rect.shrink2(vec2(16.0, 4.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let msg_edit = egui::TextEdit::singleline(&mut self.message_input)
            .font(font_regular(14.0))
            .hint_text(
                RichText::new("Nhập tin nhắn")
                    .font(font_regular(14.0))
                    .color(Color32::from_rgb(115, 115, 115)),
            )
            .frame(false)
            .desired_width(text_box_rect.width() - 32.0);
        let msg_resp = text_child.add(msg_edit);

        let can_send = !self.message_input.trim().is_empty();
        let send_resp = ui.interact(
            send_btn_rect,
            ui.id().with("chat_send_btn"),
            if can_send { Sense::click() } else { Sense::hover() },
        );
        let send_bg = if can_send {
            if send_resp.hovered() {
                Color32::from_rgb(29, 78, 216)
            } else {
                Color32::from_rgb(37, 99, 235)
            }
        } else {
            Color32::from_rgb(236, 238, 242)
        };
        ui.painter().rect_filled(send_btn_rect, CornerRadius::same(8), send_bg);
        let send_icon_col = if can_send {
            Color32::WHITE
        } else {
            Color32::from_rgb(115, 115, 115)
        };
        draw_svg_send_icon(
            ui.painter(),
            Rect::from_center_size(send_btn_rect.center(), vec2(18.0, 18.0)),
            send_icon_col,
        );

        let should_send = (send_resp.clicked()
            || (msg_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
            && can_send;

        if should_send {
            *action = Some(ChatAction::SendMessage(self.message_input.trim().to_string()));
            self.message_input.clear();
            self.is_emoji_picker_open = false;
        }

        // 3. MIDDLE MESSAGES AREA
        let msgs_rect = Rect::from_min_max(
            pos2(total_rect.min.x, total_rect.min.y + header_height),
            pos2(total_rect.max.x, total_rect.max.y - input_bar_height),
        );

        let dummy = Vec::new();
        let msg_list = self.messages.get(&friend.id).unwrap_or(&dummy);

        if msg_list.is_empty() {
            ui.painter().text(
                msgs_rect.center(),
                Align2::CENTER_CENTER,
                "Chưa có tin nhắn",
                font_regular(14.0),
                Color32::from_rgb(115, 115, 115),
            );
        } else {
            let mut msgs_ui = ui.new_child(UiBuilder::new().max_rect(msgs_rect).layout(Layout::top_down(Align::Min)));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(&mut msgs_ui, |ui| {
                    ui.add_space(16.0);
                    for msg in msg_list {
                        render_message_bubble(ui, msg, &friend.username);
                    }
                    if self.typing_users.contains(&friend.id) {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                RichText::new("Đang nhập")
                                    .font(font_regular(12.0))
                                    .italics()
                                    .color(Color32::from_rgb(115, 115, 115)),
                            );
                        });
                    }
                    ui.add_space(12.0);
                });
        }

        // Emoji Picker Popup (drawn on top of messages)
        if self.is_emoji_picker_open {
            let picker_w = 250.0;
            let picker_h = 176.0;
            let picker_rect = Rect::from_min_size(
                pos2(input_bar_rect.left() + 20.0, input_bar_rect.top() - picker_h - 8.0),
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
    }

    // ==========================================
    // SETTINGS VIEW (Matches Chat.tsx lines 926-1010 & main.css lines 843-996)
    // ==========================================
    pub fn render_settings_view(&mut self, ui: &mut Ui, action: &mut Option<ChatAction>) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::WHITE);

        // .chat-header: height 54px, border-bottom 1px solid #E5E7EB
        let header_height = 54.0;
        let header_rect = Rect::from_min_size(total_rect.min, Vec2::new(total_rect.width(), header_height));
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        let back_rect = Rect::from_center_size(
            pos2(header_rect.left() + 20.0 + 18.0, header_rect.center().y),
            vec2(36.0, 36.0),
        );
        let back_resp = ui.interact(back_rect, ui.id().with("settings_back_btn"), Sense::click());
        if back_resp.hovered() {
            ui.painter().rect_filled(back_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let back_col = if back_resp.hovered() {
            Color32::from_rgb(23, 23, 23)
        } else {
            Color32::from_rgb(115, 115, 115)
        };
        draw_svg_back_icon(
            ui.painter(),
            Rect::from_center_size(back_rect.center(), vec2(20.0, 20.0)),
            back_col,
        );
        if back_resp.clicked() {
            self.is_settings_open = false;
        }

        ui.painter().text(
            pos2(back_rect.right() + 12.0, header_rect.center().y),
            Align2::LEFT_CENTER,
            "Cài đặt",
            font_semibold(15.0),
            Color32::from_rgb(23, 23, 23),
        );

        // .settings-content: padding 24px, max-width 600px, gap 24px
        let content_rect = Rect::from_min_max(
            pos2(total_rect.min.x, total_rect.min.y + header_height),
            total_rect.max,
        );

        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut content_ui, |ui| {
                ui.add_space(24.0);

                // max-width: 600px with 24px padding -> content width = 552px max
                let max_w = 552.0_f32.min(ui.available_width() - 48.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(max_w);
                        ui.spacing_mut().item_spacing.y = 0.0;

                        // 1. Profile Card (.settings-profile-card: #F7F8FA bg, 10px radius, padding 16px 20px, NO border)
                        let (card_rect, _) = ui.allocate_exact_size(Vec2::new(max_w, 88.0), Sense::hover());
                        ui.painter().rect_filled(card_rect, CornerRadius::same(10), Color32::from_rgb(247, 248, 250));

                        // Avatar 56px
                        let av_rect = Rect::from_center_size(
                            pos2(card_rect.left() + 20.0 + 28.0, card_rect.center().y),
                            vec2(56.0, 56.0),
                        );
                        paint_avatar_in_rect(ui.painter(), av_rect, &self.current_username);

                        let info_x = av_rect.right() + 16.0;
                        ui.painter().text(
                            pos2(info_x, card_rect.center().y - 10.0),
                            Align2::LEFT_CENTER,
                            &self.current_username,
                            font_regular(16.0),
                            Color32::from_rgb(23, 23, 23),
                        );
                        ui.painter().text(
                            pos2(info_x, card_rect.center().y + 11.0),
                            Align2::LEFT_CENTER,
                            "Tài khoản cá nhân",
                            font_regular(13.0),
                            Color32::from_rgb(115, 115, 115),
                        );

                        // "Đổi ảnh đại diện" button (.btn-avatar-upload: #FFFFFF bg, #2563EB text, 6px radius)
                        let upload_btn_w = 128.0;
                        let upload_btn_rect = Rect::from_center_size(
                            pos2(card_rect.right() - 20.0 - upload_btn_w * 0.5, card_rect.center().y),
                            vec2(upload_btn_w, 34.0),
                        );
                        let upload_resp = ui.interact(upload_btn_rect, ui.id().with("btn_avatar_upload"), Sense::click());
                        let upload_bg = if upload_resp.hovered() {
                            Color32::from_rgb(236, 238, 242)
                        } else {
                            Color32::WHITE
                        };
                        ui.painter().rect_filled(upload_btn_rect, CornerRadius::same(6), upload_bg);
                        ui.painter().text(
                            upload_btn_rect.center(),
                            Align2::CENTER_CENTER,
                            "Đổi ảnh đại diện",
                            font_regular(13.0),
                            Color32::from_rgb(37, 99, 235),
                        );

                        ui.add_space(24.0);

                        // 2. Group: Tài khoản (.settings-group: gap 12px)
                        ui.label(
                            RichText::new("Tài khoản")
                                .font(font_regular(14.0))
                                .color(Color32::from_rgb(23, 23, 23)),
                        );
                        ui.add_space(12.0);
                        render_settings_row_flat(ui, max_w, "Tên người dùng", &self.current_username, false);
                        ui.add_space(12.0);
                        render_settings_column_row_flat(ui, max_w, "Mã định danh", &self.current_user_id.to_string(), false);

                        ui.add_space(24.0);

                        // 3. Group: Mã hóa đầu cuối
                        ui.label(
                            RichText::new("Mã hóa đầu cuối")
                                .font(font_regular(14.0))
                                .color(Color32::from_rgb(23, 23, 23)),
                        );
                        ui.add_space(12.0);
                        render_settings_row_flat(ui, max_w, "Giao thức", "ECDH P-256 / AES-256-GCM", false);
                        ui.add_space(12.0);
                        render_settings_row_flat(ui, max_w, "Chữ ký số", "ECDSA P-256 / SHA-256", false);
                        ui.add_space(12.0);
                        render_settings_tag_row_flat(ui, max_w, "Trạng thái khóa", "Đã kích hoạt trên thiết bị", false);

                        if let Some((_, my_fp, _)) = self.cached_keys.get(&self.current_user_id) {
                            let fp_b64 = URL_SAFE_NO_PAD.encode(my_fp);
                            ui.add_space(12.0);
                            render_settings_column_row_flat(ui, max_w, "Vân tay khóa", &fp_b64, false);
                        }

                        ui.add_space(24.0);

                        // 4. Group: Phiên đăng nhập
                        ui.label(
                            RichText::new("Phiên đăng nhập")
                                .font(font_regular(14.0))
                                .color(Color32::from_rgb(23, 23, 23)),
                        );
                        ui.add_space(12.0);

                        // .btn-danger-outline: padding 10px 16px, #FCE8E6 bg, #DC2626 text, 8px radius
                        let (logout_btn_rect, logout_btn_resp) = ui.allocate_exact_size(Vec2::new(104.0, 40.0), Sense::click());
                        let logout_bg = if logout_btn_resp.hovered() {
                            Color32::from_rgb(250, 215, 212)
                        } else {
                            Color32::from_rgb(252, 232, 230) // #FCE8E6
                        };
                        ui.painter().rect_filled(logout_btn_rect, CornerRadius::same(8), logout_bg);
                        ui.painter().text(
                            logout_btn_rect.center(),
                            Align2::CENTER_CENTER,
                            "Đăng xuất",
                            font_regular(14.0),
                            Color32::from_rgb(220, 38, 38), // #DC2626
                        );

                        if logout_btn_resp.clicked() {
                            *action = Some(ChatAction::Logout);
                        }

                        ui.add_space(32.0);
                    });
                });
            });
    }

    // ==========================================
    // RIGHT PARTNER INFO SIDEBAR (Matches Chat.tsx lines 1118-1170)
    // ==========================================
    pub fn render_partner_info_sidebar(&mut self, ui: &mut Ui, friend: &FriendItem) {
        let total_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(total_rect, 0.0, Color32::WHITE);

        // .partner-info-header: height 54px, border-bottom 1px solid #E5E7EB
        let header_height = 54.0;
        let header_rect = Rect::from_min_size(total_rect.min, Vec2::new(total_rect.width(), header_height));
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(229, 231, 235)),
        );

        ui.painter().text(
            pos2(header_rect.left() + 20.0, header_rect.center().y),
            Align2::LEFT_CENTER,
            "Thông tin",
            font_regular(15.0),
            Color32::from_rgb(23, 23, 23),
        );

        let close_rect = Rect::from_center_size(
            pos2(header_rect.right() - 20.0 - 18.0, header_rect.center().y),
            vec2(36.0, 36.0),
        );
        let close_resp = ui.interact(close_rect, ui.id().with("partner_info_close_btn"), Sense::click());
        if close_resp.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(6), Color32::from_rgb(236, 238, 242));
        }
        let close_col = if close_resp.hovered() {
            Color32::from_rgb(23, 23, 23)
        } else {
            Color32::from_rgb(115, 115, 115)
        };
        draw_svg_close_icon(
            ui.painter(),
            Rect::from_center_size(close_rect.center(), vec2(18.0, 18.0)),
            close_col,
        );
        if close_resp.clicked() {
            self.is_partner_info_open = false;
        }

        // .partner-info-content: padding 0 20px 24px, gap 16px
        let content_rect = Rect::from_min_max(
            pos2(total_rect.min.x, total_rect.min.y + header_height),
            total_rect.max,
        );

        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut content_ui, |ui| {
                let card_w = ui.available_width() - 40.0;
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(card_w);
                        ui.spacing_mut().item_spacing.y = 0.0;

                        // .partner-info-card: padding 24px 20px, centered Avatar(64) + name(17px semibold) + sub(13px)
                        let (pcard_rect, _) = ui.allocate_exact_size(Vec2::new(card_w, 156.0), Sense::hover());
                        let av_rect = Rect::from_center_size(
                            pos2(pcard_rect.center().x, pcard_rect.top() + 24.0 + 32.0),
                            vec2(64.0, 64.0),
                        );
                        paint_avatar_in_rect(ui.painter(), av_rect, &friend.username);

                        ui.painter().text(
                            pos2(pcard_rect.center().x, av_rect.bottom() + 18.0),
                            Align2::CENTER_CENTER,
                            &friend.username,
                            font_regular(16.0),
                            Color32::from_rgb(23, 23, 23),
                        );
                        ui.painter().text(
                            pos2(pcard_rect.center().x, av_rect.bottom() + 40.0),
                            Align2::CENTER_CENTER,
                            "Tài khoản bạn bè",
                            font_regular(13.0),
                            Color32::from_rgb(115, 115, 115),
                        );

                        ui.add_space(16.0);

                        // Group: Tài khoản (inside partner-info-sidebar, rows have #FFFFFF bg and 1px #ECEEF2 border)
                        ui.label(
                            RichText::new("Tài khoản")
                                .font(font_regular(14.0))
                                .color(Color32::from_rgb(23, 23, 23)),
                        );
                        ui.add_space(12.0);
                        render_settings_row_flat(ui, card_w, "Tên người dùng", &friend.username, true);
                        ui.add_space(12.0);
                        render_settings_column_row_flat(ui, card_w, "Mã định danh", &friend.id.to_string(), true);

                        ui.add_space(16.0);

                        // Group: Mã hóa đầu cuối
                        ui.label(
                            RichText::new("Mã hóa đầu cuối")
                                .font(font_regular(14.0))
                                .color(Color32::from_rgb(23, 23, 23)),
                        );
                        ui.add_space(12.0);
                        render_settings_row_flat(ui, card_w, "Giao thức", "ECDH P-256 / AES-256-GCM", true);
                        ui.add_space(12.0);
                        render_settings_row_flat(ui, card_w, "Chữ ký số", "ECDSA P-256 / SHA-256", true);
                        ui.add_space(12.0);

                        let has_key = self.cached_keys.contains_key(&friend.id);
                        render_settings_tag_row_flat(
                            ui,
                            card_w,
                            "Trạng thái khóa",
                            if has_key { "Đã kích hoạt trên thiết bị" } else { "Đang tải" },
                            true,
                        );

                        if let Some((_, fp, _)) = self.cached_keys.get(&friend.id) {
                            let fp_b64 = URL_SAFE_NO_PAD.encode(fp);
                            ui.add_space(12.0);
                            render_settings_column_row_flat(ui, card_w, "Vân tay khóa", &fp_b64, true);
                        }

                        ui.add_space(24.0);
                    });
                });
            });
    }
}

// ==========================================
// MESSAGE BUBBLE & SETTINGS ROW HELPERS
// ==========================================

fn render_message_bubble(ui: &mut Ui, msg: &DecryptedMessage, friend_name: &str) {
    let max_bubble_width = (ui.available_width() * 0.65).max(180.0);

    ui.horizontal(|ui| {
        if msg.is_me {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(37, 99, 235)) // #2563EB
                        .corner_radius(CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 14,
                            se: 4,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(max_bubble_width);
                            ui.label(
                                RichText::new(&msg.text)
                                    .font(font_regular(14.0))
                                    .color(Color32::WHITE),
                            );
                        });

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&msg.time_str)
                                .font(font_regular(11.0))
                                .color(Color32::from_rgb(115, 115, 115)),
                        );
                    });
                });
            });
        } else {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.add_space(20.0);
                render_avatar(ui, friend_name, 28.0);
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(241, 243, 245)) // #F1F3F5
                        .corner_radius(CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 4,
                            se: 14,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(max_bubble_width);
                            ui.label(
                                RichText::new(&msg.text)
                                    .font(font_regular(14.0))
                                    .color(Color32::from_rgb(23, 23, 23)),
                            );
                        });

                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&msg.time_str)
                                .font(font_regular(11.0))
                                .color(Color32::from_rgb(115, 115, 115)),
                        );
                    });
                });
            });
        }
    });

    ui.add_space(8.0);
}

/// Horizontal `.settings-row`:
/// - In Settings (`bordered == false`): `#F7F8FA` background, NO border, `8px` radius, `padding: 12px 16px`
/// - In Partner Info (`bordered == true`): `#FFFFFF` background, `1px solid #ECEEF2` border
fn render_settings_row_flat(ui: &mut Ui, width: f32, label: &str, value: &str, bordered: bool) {
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, 45.0), Sense::hover());
    let bg = if bordered {
        Color32::WHITE
    } else {
        Color32::from_rgb(247, 248, 250) // #F7F8FA
    };
    ui.painter().rect_filled(row_rect, CornerRadius::same(8), bg);
    if bordered {
        ui.painter().rect_stroke(
            row_rect,
            CornerRadius::same(8),
            Stroke::new(1.0_f32, Color32::from_rgb(236, 238, 242)),
            StrokeKind::Inside,
        );
    }

    ui.painter().text(
        pos2(row_rect.left() + 16.0, row_rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font_regular(14.0),
        Color32::from_rgb(115, 115, 115),
    );
    ui.painter().text(
        pos2(row_rect.right() - 16.0, row_rect.center().y),
        Align2::RIGHT_CENTER,
        value,
        font_regular(14.0),
        Color32::from_rgb(23, 23, 23),
    );
}

/// Vertical 2-line `.settings-row.column-layout` for UUID (`Mã định danh`) and Fingerprint (`Vân tay khóa`)
fn render_settings_column_row_flat(ui: &mut Ui, width: f32, label: &str, value: &str, bordered: bool) {
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, 64.0), Sense::hover());
    let bg = if bordered {
        Color32::WHITE
    } else {
        Color32::from_rgb(247, 248, 250) // #F7F8FA
    };
    ui.painter().rect_filled(row_rect, CornerRadius::same(8), bg);
    if bordered {
        ui.painter().rect_stroke(
            row_rect,
            CornerRadius::same(8),
            Stroke::new(1.0_f32, Color32::from_rgb(236, 238, 242)),
            StrokeKind::Inside,
        );
    }

    ui.painter().text(
        pos2(row_rect.left() + 16.0, row_rect.top() + 21.0),
        Align2::LEFT_CENTER,
        label,
        font_regular(14.0),
        Color32::from_rgb(115, 115, 115),
    );
    ui.painter().text(
        pos2(row_rect.left() + 16.0, row_rect.top() + 44.0),
        Align2::LEFT_CENTER,
        value,
        font_mono(12.0),
        Color32::from_rgb(23, 23, 23),
    );
}

/// Horizontal `.settings-row` with `.tag-active` green badge
fn render_settings_tag_row_flat(ui: &mut Ui, width: f32, label: &str, tag_text: &str, bordered: bool) {
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, 45.0), Sense::hover());
    let bg = if bordered {
        Color32::WHITE
    } else {
        Color32::from_rgb(247, 248, 250) // #F7F8FA
    };
    ui.painter().rect_filled(row_rect, CornerRadius::same(8), bg);
    if bordered {
        ui.painter().rect_stroke(
            row_rect,
            CornerRadius::same(8),
            Stroke::new(1.0_f32, Color32::from_rgb(236, 238, 242)),
            StrokeKind::Inside,
        );
    }

    ui.painter().text(
        pos2(row_rect.left() + 16.0, row_rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font_regular(14.0),
        Color32::from_rgb(115, 115, 115),
    );

    let galley = ui.painter().layout_no_wrap(
        tag_text.to_string(),
        font_regular(12.0),
        Color32::from_rgb(19, 115, 51),
    );
    let tag_w = galley.size().x + 20.0;
    let tag_rect = Rect::from_center_size(
        pos2(row_rect.right() - 16.0 - tag_w * 0.5, row_rect.center().y),
        vec2(tag_w, 26.0),
    );
    ui.painter().rect_filled(tag_rect, CornerRadius::same(6), Color32::from_rgb(230, 244, 234)); // #E6F4EA
    ui.painter().galley(
        pos2(tag_rect.center().x - galley.size().x * 0.5, tag_rect.center().y - galley.size().y * 0.5),
        galley,
        Color32::from_rgb(19, 115, 51), // #137333
    );
}

/// Exact Web `Avatar.tsx` color hash, 8-color palette, and font size rules
pub fn paint_avatar_in_rect(painter: &egui::Painter, rect: Rect, name: &str) {
    let trimmed = name.trim();
    let initial = trimmed
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

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
    for ch in name.encode_utf16() {
        hash = (hash.wrapping_shl(5)).wrapping_sub(hash).wrapping_add(ch as i32);
    }
    let idx = (hash.unsigned_abs() as usize) % palette.len();
    let bg_color = palette[idx];

    let size = rect.width();
    let font_sz = if size <= 28.0 {
        11.0
    } else if size <= 36.0 {
        14.0
    } else if size <= 42.0 {
        16.0
    } else {
        24.0
    };

    painter.circle_filled(rect.center(), size * 0.5, bg_color);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        font_semibold(font_sz),
        Color32::WHITE,
    );
}

pub fn render_avatar(ui: &mut Ui, name: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    paint_avatar_in_rect(ui.painter(), rect, name);
}
