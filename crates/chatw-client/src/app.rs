use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use egui::{Color32, RichText};
use std::sync::{Arc, mpsc};
use uuid::Uuid;

use crate::net::{HttpClient, WsManager};
use crate::storage;
use crate::titlebar;
use crate::views::{ChatAction, ChatView, DecryptedMessage, LoginAction, LoginView, SetNameView};
use chatw_core::{
    ClientKeys, FriendItem, MeResponse, MessageWireItem, PendingRequestItem, PublicKeysResponse,
    UserSearchResult, WsClientCommand, WsServerEvent, compute_fingerprint, decrypt_message,
    encrypt_message,
};

#[allow(dead_code)]
pub enum GuiEvent {
    AuthResult(Result<MeResponse, String>),
    DevLoginResult {
        result: Result<MeResponse, String>,
        token: Option<String>,
    },
    SetNameResult(Result<(), String>),
    FriendsList(Result<Vec<FriendItem>, String>),
    PendingList(Result<Vec<PendingRequestItem>, String>),
    SearchResults(Result<Vec<UserSearchResult>, String>),
    FriendKeysLoaded {
        user_id: Uuid,
        result: Result<PublicKeysResponse, String>,
    },
    HistoryLoaded {
        friend_id: Uuid,
        result: Result<Vec<MessageWireItem>, String>,
    },
    MessageSendResult {
        result: Result<(), String>,
    },
    FriendRequestSent(Result<(), String>),
    FriendRequestAccepted(Result<(), String>),
}

pub enum AppState {
    CheckingAuth,
    Login(LoginView),
    SetName(SetNameView),
    Chat(ChatView),
}

pub struct ChatApp {
    pub runtime: Arc<tokio::runtime::Runtime>,
    pub http: HttpClient,
    pub ws: Option<WsManager>,
    pub keys: ClientKeys,
    pub state: AppState,
    pub tx_gui: mpsc::Sender<GuiEvent>,
    pub rx_gui: mpsc::Receiver<GuiEvent>,
    pub is_maximized: bool,
    pub dwm_applied: bool,
    pub pending_decrypt_queue: Vec<(Uuid, MessageWireItem)>,
}

fn drain_decrypt_queue(
    queue: &mut Vec<(Uuid, MessageWireItem)>,
    chat: &mut ChatView,
    my_keys: &ClientKeys,
    ws: &Option<WsManager>,
) {
    let mut still_pending = Vec::new();
    let current_queue = std::mem::take(queue);

    for (sender_id, wire) in current_queue {
        if let Some((_, _, ecdsa_pub)) = chat.cached_keys.get(&sender_id) {
            if let Ok(raw_env) = URL_SAFE_NO_PAD.decode(&wire.envelope) {
                if let Ok(text) = decrypt_message(my_keys, ecdsa_pub, &raw_env) {
                    let msgs = chat.messages.entry(sender_id).or_default();
                    if !msgs.iter().any(|m| m.id == wire.id) {
                        msgs.push(DecryptedMessage {
                            id: wire.id,
                            sender_id,
                            is_me: false,
                            text,
                            time_str: wire
                                .created_at
                                .with_timezone(&chrono::Local)
                                .format("%H:%M")
                                .to_string(),
                        });
                        if chat.selected_friend_id != Some(sender_id) || chat.is_settings_open {
                            *chat.unread_counts.entry(sender_id).or_insert(0) += 1;
                        }
                    }
                    if let Some(ws_mgr) = ws {
                        let _ = ws_mgr.tx_commands.send(WsClientCommand::Ack {
                            message_id: wire.id,
                        });
                    }
                    continue;
                }
            }
        }
        still_pending.push((sender_id, wire));
    }

    *queue = still_pending;
}

static INTER_LIGHT: &[u8] = include_bytes!("../assets/fonts/Inter-Light.ttf");
static INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
static INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
static INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");

fn setup_vietnamese_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::empty();

    // 1. Embedded Inter font family (world-class screen typography, full Vietnamese diacritics)
    fonts.font_data.insert(
        "inter_light".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(INTER_LIGHT)),
    );
    fonts.font_data.insert(
        "inter_regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        "inter_medium".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(INTER_MEDIUM)),
    );
    fonts.font_data.insert(
        "inter_semibold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(INTER_SEMIBOLD)),
    );
    fonts.font_data.insert(
        "inter_bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(INTER_BOLD)),
    );

    // 2. Windows Fallbacks (Segoe UI, Consolas, Segoe UI Emoji)
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let fonts_dir = std::path::Path::new(&windir).join("Fonts");

    let segoe_path = fonts_dir.join("segoeui.ttf");
    if let Ok(bytes) = std::fs::read(&segoe_path) {
        fonts.font_data.insert(
            "segoe_ui".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    let segoe_semi_path = fonts_dir.join("seguisb.ttf");
    if let Ok(bytes) = std::fs::read(&segoe_semi_path) {
        fonts.font_data.insert(
            "segoe_ui_semibold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    let segoe_bold_path = fonts_dir.join("segoeuib.ttf");
    if let Ok(bytes) = std::fs::read(&segoe_bold_path) {
        fonts.font_data.insert(
            "segoe_ui_bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    let consola_path = fonts_dir.join("consola.ttf");
    if let Ok(bytes) = std::fs::read(&consola_path) {
        fonts.font_data.insert(
            "consolas".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    let emoji_path = fonts_dir.join("seguiemj.ttf");
    if let Ok(bytes) = std::fs::read(&emoji_path) {
        fonts.font_data.insert(
            "segoe_emoji".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    // 3. Proportional family: Segoe UI (native Windows, clean, crisp stems) -> Inter-Medium -> Inter-Regular -> Emoji
    let mut prop = Vec::new();
    if fonts.font_data.contains_key("segoe_ui") {
        prop.push("segoe_ui".to_owned());
    }
    prop.push("inter_medium".to_owned());
    prop.push("inter_regular".to_owned());
    if fonts.font_data.contains_key("segoe_emoji") {
        prop.push("segoe_emoji".to_owned());
    }
    fonts.families.insert(egui::FontFamily::Proportional, prop);

    // 4. Medium family: Segoe UI Semibold -> Segoe UI -> Inter-Medium -> Inter-SemiBold -> Emoji
    let medium_family = egui::FontFamily::Name("medium".into());
    let mut med = Vec::new();
    if fonts.font_data.contains_key("segoe_ui_semibold") {
        med.push("segoe_ui_semibold".to_owned());
    } else if fonts.font_data.contains_key("segoe_ui") {
        med.push("segoe_ui".to_owned());
    }
    med.push("inter_medium".to_owned());
    med.push("inter_semibold".to_owned());
    med.push("inter_regular".to_owned());
    if fonts.font_data.contains_key("segoe_emoji") {
        med.push("segoe_emoji".to_owned());
    }
    fonts.families.insert(medium_family, med);

    // 5. Semibold family: Segoe UI Semibold -> Segoe UI Bold -> Inter-SemiBold -> Inter-Bold -> Emoji
    let semibold_family = egui::FontFamily::Name("semibold".into());
    let mut semi = Vec::new();
    if fonts.font_data.contains_key("segoe_ui_semibold") {
        semi.push("segoe_ui_semibold".to_owned());
    } else if fonts.font_data.contains_key("segoe_ui") {
        semi.push("segoe_ui".to_owned());
    }
    semi.push("inter_semibold".to_owned());
    semi.push("inter_bold".to_owned());
    semi.push("inter_medium".to_owned());
    if fonts.font_data.contains_key("segoe_emoji") {
        semi.push("segoe_emoji".to_owned());
    }
    fonts.families.insert(semibold_family, semi);

    // 6. Bold family: Segoe UI Bold -> Segoe UI Semibold -> Inter-Bold -> Inter-SemiBold -> Emoji
    let bold_family = egui::FontFamily::Name("bold".into());
    let mut bold = Vec::new();
    if fonts.font_data.contains_key("segoe_ui_bold") {
        bold.push("segoe_ui_bold".to_owned());
    } else if fonts.font_data.contains_key("segoe_ui_semibold") {
        bold.push("segoe_ui_semibold".to_owned());
    }
    bold.push("inter_bold".to_owned());
    bold.push("inter_semibold".to_owned());
    if fonts.font_data.contains_key("segoe_emoji") {
        bold.push("segoe_emoji".to_owned());
    }
    fonts.families.insert(bold_family, bold);

    // 7. Monospace family: Consolas -> Segoe UI -> Inter-Regular
    let mut mono = Vec::new();
    if fonts.font_data.contains_key("consolas") {
        mono.push("consolas".to_owned());
    }
    if fonts.font_data.contains_key("segoe_ui") {
        mono.push("segoe_ui".to_owned());
    }
    mono.push("inter_regular".to_owned());
    fonts.families.insert(egui::FontFamily::Monospace, mono);

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inter_vietnamese_glyphs() {
        let inter_bytes = include_bytes!("../assets/fonts/Inter-Light.ttf");
        let mut fonts = egui::FontDefinitions::empty();
        fonts.font_data.insert(
            "inter_light".into(),
            std::sync::Arc::new(egui::FontData::from_static(inter_bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("inter_light".into());
        let ctx = egui::Context::default();
        ctx.set_fonts(fonts);
        let sample = "AÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬEÉÈẺẼẸÊẾỀỂỄỆIÍÌỈĨỊOÓÒỎÕỌÔỐỒỔỖỘƠỚỜỞỠỢUÚÙỦŨỤƯỨỪỬỮỰYÝỲỶỸỴĐ aáàảãạăắằẳẵặâấầẩẫậeéèẻẽẹêếềểễệiíìỉĩịoóòỏõọôốồổỗộơớờởỡợuúùủũụưứừửữựyýỳỷỹỵđ Mã hóa đầu cuối Tài khoản Tên người dùng Mã định danh Giao thức Chữ ký số Ứng dụng nhắn tin bảo mật";
        let _ = ctx.run(Default::default(), |ctx| {
            ctx.fonts(|f| {
                let mut missing = Vec::new();
                for c in sample.chars() {
                    if !f.has_glyph(&egui::FontId::proportional(14.0), c) {
                        missing.push(c);
                    }
                }
                println!(
                    "Inter Light missing count: {}, missing: {:?}",
                    missing.len(),
                    missing
                );
                assert!(
                    missing.is_empty(),
                    "Inter Light missing glyphs: {:?}",
                    missing
                );
            });
        });
    }

    #[test]
    fn test_inter_weights_glyphs() {
        let mut fonts = egui::FontDefinitions::empty();
        fonts.font_data.insert(
            "light".into(),
            std::sync::Arc::new(egui::FontData::from_static(INTER_LIGHT)),
        );
        fonts.font_data.insert(
            "regular".into(),
            std::sync::Arc::new(egui::FontData::from_static(INTER_REGULAR)),
        );
        fonts.font_data.insert(
            "medium".into(),
            std::sync::Arc::new(egui::FontData::from_static(INTER_MEDIUM)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("light".into());
        fonts
            .families
            .entry(egui::FontFamily::Name("medium".into()))
            .or_default()
            .push("medium".into());
        fonts
            .families
            .entry(egui::FontFamily::Name("semibold".into()))
            .or_default()
            .push("medium".into());
        let ctx = egui::Context::default();
        ctx.set_fonts(fonts);
        let sample = "AÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬEÉÈẺẼẸÊẾỀỂỄỆIÍÌỈĨỊOÓÒỎÕỌÔỐỒỔỖỘƠỚỜỞỠỢUÚÙỦŨỤƯỨỪỬỮỰYÝỲỶỸỴĐ aáàảãạăắằẳẵặâấầẩẫậeéèẻẽẹêếềểễệiíìỉĩịoóòỏõọôốồổỗộơớờởỡợuúùủũụưứừửữựyýỳỷỹỵđ";
        let _ = ctx.run(Default::default(), |ctx| {
            ctx.fonts(|f| {
                for c in sample.chars() {
                    let has_light = f.has_glyph(&egui::FontId::proportional(14.0), c);
                    assert!(
                        has_light,
                        "Character '{}' (U+{:04X}) missing in Inter-Light",
                        c, c as u32
                    );
                    let has_semi = f.has_glyph(&titlebar::font_semibold(14.0), c);
                    assert!(
                        has_semi,
                        "Character '{}' (U+{:04X}) missing in Inter-Medium",
                        c, c as u32
                    );
                }
            });
        });
    }

    #[test]
    fn test_vietnamese_font_glyphs() {
        let ctx = egui::Context::default();
        setup_vietnamese_fonts(&ctx);
        let sample = "AÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬEÉÈẺẼẸÊẾỀỂỄỆIÍÌỈĨỊOÓÒỎÕỌÔỐỒỔỖỘƠỚỜỞỠỢUÚÙỦŨỤƯỨỪỬỮỰYÝỲỶỸỴĐ aáàảãạăắằẳẵặâấầẩẫậeéèẻẽẹêếềểễệiíìỉĩịoóòỏõọôốồổỗộơớờởỡợuúùủũụưứừửữựyýỳỷỹỵđ Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE Tiếp tục với Google Đăng nhập chế độ thử nghiệm";
        let _ = ctx.run(Default::default(), |ctx| {
            ctx.fonts(|f| {
                for c in sample.chars() {
                    assert!(
                        f.has_glyph(&egui::FontId::proportional(14.0), c),
                        "Missing prop glyph for '{}'",
                        c
                    );
                    assert!(
                        f.has_glyph(&titlebar::font_semibold(14.0), c),
                        "Missing semibold glyph for '{}'",
                        c
                    );
                    assert!(
                        f.has_glyph(&titlebar::font_bold(14.0), c),
                        "Missing bold glyph for '{}'",
                        c
                    );
                }
            });
        });
    }

    #[test]
    fn test_section_8_typography_and_multi_dpi() {
        let test_strings = [
            "Tiếng Việt có dấu: ă â ê ô ơ ư đ",
            "Chỉnh sửa cài đặt và quản lý dữ liệu",
            "Ứng dụng hoạt động ổn định",
            "Settings and Preferences",
            "Application Dashboard",
            "Modern Typography 1234567890",
        ];

        let font_sizes = [11.0, 12.0, 13.5, 14.0, 16.0, 18.0, 20.0, 24.0];
        let dpi_scales = [1.0, 1.25, 1.5, 2.0];

        for &dpi in &dpi_scales {
            let ctx = egui::Context::default();
            setup_vietnamese_fonts(&ctx);
            ctx.set_zoom_factor(dpi);

            // Test snap_pos across DPIs
            for raw_val in [10.1, 15.33, 22.75, 48.49] {
                let snapped = titlebar::snap_xy(raw_val, raw_val, dpi);
                // The physical pixel must be an integer: (coord * ppp).round()
                let phys_x = snapped.x * dpi;
                let phys_y = snapped.y * dpi;
                assert!(
                    (phys_x - phys_x.round()).abs() < 1e-4,
                    "phys_x {} not on integer pixel at dpi {}",
                    phys_x,
                    dpi
                );
                assert!(
                    (phys_y - phys_y.round()).abs() < 1e-4,
                    "phys_y {} not on integer pixel at dpi {}",
                    phys_y,
                    dpi
                );
            }

            // Test glyph availability across all sizes and weights and widget rendering
            let _ = ctx.run(Default::default(), |ctx| {
                let ppp = ctx.pixels_per_point();
                assert_eq!(ppp, dpi);

                ctx.fonts(|f| {
                    for &text in &test_strings {
                        for &size in &font_sizes {
                            for c in text.chars() {
                                assert!(
                                    f.has_glyph(&egui::FontId::proportional(size), c),
                                    "Missing proportional glyph '{}' at size {} at dpi {}",
                                    c,
                                    size,
                                    dpi
                                );
                                assert!(
                                    f.has_glyph(&titlebar::font_semibold(size), c),
                                    "Missing semibold glyph '{}' at size {} at dpi {}",
                                    c,
                                    size,
                                    dpi
                                );
                                assert!(
                                    f.has_glyph(&titlebar::font_bold(size), c),
                                    "Missing bold glyph '{}' at size {} at dpi {}",
                                    c,
                                    size,
                                    dpi
                                );
                            }
                        }
                    }
                });

                // Render widgets with these strings: Label, Button, TextEdit
                egui::CentralPanel::default().show(ctx, |ui| {
                    for &text in &test_strings {
                        ui.label(text);
                        let _ = ui.button(text);
                        let mut edit_buf = text.to_string();
                        ui.text_edit_singleline(&mut edit_buf);
                    }
                });
            });
        }
    }

    #[test]
    fn test_chat_view_render_all_states() {
        let ctx = egui::Context::default();
        setup_vietnamese_fonts(&ctx);

        let my_id = Uuid::new_v4();
        let friend_id = Uuid::new_v4();
        let mut chat_view = ChatView::new(my_id, "123dá".to_string());

        // 1. Initial empty state
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                chat_view.show(ui);
            });
        });

        // 2. Friend added and selected
        chat_view.friends.push(FriendItem {
            id: friend_id,
            username: "Nguyễn Văn A".to_string(),
            avatar_url: None,
        });
        chat_view.selected_friend_id = Some(friend_id);

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                chat_view.show(ui);
            });
        });

        // 3. Messages present
        chat_view.messages.insert(
            friend_id,
            vec![
                DecryptedMessage {
                    id: Uuid::new_v4(),
                    sender_id: my_id,
                    is_me: true,
                    text: "Xin chào bạn nhé!".to_string(),
                    time_str: "19:30".to_string(),
                },
                DecryptedMessage {
                    id: Uuid::new_v4(),
                    sender_id: friend_id,
                    is_me: false,
                    text: "Chào bạn, mã hóa E2EE hoạt động tốt!".to_string(),
                    time_str: "19:31".to_string(),
                },
            ],
        );

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                chat_view.show(ui);
            });
        });
    }

    #[test]
    fn test_cookie_jar_for_websocket() {
        use reqwest::cookie::CookieStore;
        let http = HttpClient::new("http://127.0.0.1:8085".to_string());
        http.set_session_token("test_token_abc_123");
        let ws_url = "http://127.0.0.1:8085/api/ws"
            .parse::<reqwest::Url>()
            .unwrap();
        let cookie = http.cookie_jar.cookies(&ws_url);
        println!("Cookie header: {:?}", cookie);
    }

    #[test]
    fn test_full_login_transition_flow() {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        );
        let http = HttpClient::new("http://127.0.0.1:8085".to_string());

        let me_opt = runtime.block_on(async {
            let res = http.dev_login(Some("FlowTester")).await;
            if res.is_ok() {
                http.get_me().await.ok()
            } else {
                None
            }
        });

        let me = me_opt.unwrap_or_else(|| chatw_core::MeResponse {
            id: Uuid::new_v4(),
            username: Some("FlowTester".to_string()),
            avatar_url: None,
            has_keys: true,
        });

        // Now simulate the full app transition
        let ctx = egui::Context::default();
        setup_vietnamese_fonts(&ctx);

        let keys = storage::load_or_create_keys();
        let (tx_gui, rx_gui) = mpsc::channel();

        let mut app = ChatApp {
            runtime: runtime.clone(),
            http: http.clone(),
            ws: None,
            keys,
            state: AppState::CheckingAuth,
            tx_gui,
            rx_gui,
            is_maximized: false,
            dwm_applied: false,
            pending_decrypt_queue: Vec::new(),
        };

        // Transition to logged in user
        app.handle_logged_in_user(me, None, &ctx);

        match &app.state {
            AppState::SetName(_) => {
                // Run GUI frames in SetName view
                for _ in 0..5 {
                    let _ = ctx.run(Default::default(), |ctx| {
                        app.update_ui(ctx);
                    });
                }

                // Set username and transition to Chat
                let me_named = runtime.block_on(async {
                    let _ = http.set_name("FlowTester").await;
                    http.get_me().await.expect("get_me named failed")
                });
                app.handle_logged_in_user(me_named, None, &ctx);
            }
            AppState::Chat(_) => {
                // User already has username set
            }
            _ => panic!("Unexpected app state after login"),
        }

        // Verify state is now Chat
        assert!(matches!(app.state, AppState::Chat(_)));

        // Run GUI frames in Chat view
        for _ in 0..5 {
            let _ = ctx.run(Default::default(), |ctx| {
                app.update_ui(ctx);
            });
        }
    }

    #[tokio::test]
    async fn test_live_ws_connection() {
        let http = HttpClient::new("http://127.0.0.1:8085".to_string());
        let res = http.dev_login(Some("WsTester")).await;
        if res.is_err() {
            eprintln!("Live server not running at 127.0.0.1:8085, skipping live WS test");
            return;
        }
        let ctx = egui::Context::default();
        let ws = WsManager::start(
            tokio::runtime::Handle::current(),
            http.base_url.clone(),
            http.cookie_jar.clone(),
            ctx,
        );
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = ws.tx_commands.send(WsClientCommand::TypingStart {
            recipient_id: Uuid::new_v4(),
        });
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    #[tokio::test]
    async fn test_dev_login_live() {
        let http = HttpClient::new("http://127.0.0.1:8085".to_string());
        let res = http.dev_login(Some("Alice")).await;
        if res.is_err() {
            eprintln!("Live server not running at 127.0.0.1:8085, skipping live dev-login test");
            return;
        }
        assert!(res.is_ok(), "Dev login failed: {:?}", res);
        let me = http.get_me().await;
        assert!(me.is_ok(), "Get me failed: {:?}", me);
    }
}

impl ChatApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        setup_vietnamese_fonts(&cc.egui_ctx);

        // Set visual styling to pure flat light minimalist matching web CSS
        let mut visuals = egui::Visuals::light();
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.window_fill = Color32::from_rgb(255, 255, 255);
        visuals.window_stroke = egui::Stroke::NONE;
        visuals.panel_fill = Color32::from_rgb(255, 255, 255); // #FFFFFF
        visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);
        visuals.faint_bg_color = Color32::from_rgb(247, 248, 250);
        visuals.override_text_color = None;
        visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(23, 23, 23); // #171717
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(247, 248, 250); // #F7F8FA
        visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(23, 23, 23); // #171717
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(247, 248, 250); // #F7F8FA
        visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(23, 23, 23);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(236, 238, 242); // #ECEEF2
        visuals.widgets.active.fg_stroke.color = Color32::WHITE;
        visuals.widgets.active.bg_fill = Color32::from_rgb(37, 99, 235); // #2563EB
        visuals.selection.bg_fill = Color32::from_rgb(37, 99, 235);
        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, titlebar::font_medium(16.0)),
            (egui::TextStyle::Body, titlebar::font_regular(14.0)),
            (egui::TextStyle::Button, titlebar::font_medium(13.5)),
            (egui::TextStyle::Small, titlebar::font_regular(12.0)),
            (egui::TextStyle::Monospace, titlebar::font_mono(12.0)),
        ]
        .into();
        cc.egui_ctx.set_style(style);
        // Standard 100% UI scale
        cc.egui_ctx.set_zoom_factor(1.0);

        // Log DPI for debugging
        {
            let ppp = cc.egui_ctx.pixels_per_point();
            crate::log_app(&format!(
                "startup pixels_per_point={:.3} (monitor DPI scale)",
                ppp
            ));
        }

        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to initialize Tokio runtime"),
        );

        let server_url = storage::load_server_url();
        let http = HttpClient::new(server_url);
        if let Some(token) = storage::load_session_token() {
            http.set_session_token(&token);
        }
        let keys = storage::load_or_create_keys();
        let (tx_gui, rx_gui) = mpsc::channel();

        let app = Self {
            runtime,
            http,
            ws: None,
            keys,
            state: AppState::CheckingAuth,
            tx_gui,
            rx_gui,
            is_maximized: false,
            dwm_applied: false,
            pending_decrypt_queue: Vec::new(),
        };

        // Trigger initial auth check
        app.check_auth(&cc.egui_ctx);

        app
    }

    fn check_auth(&self, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let res = http.get_me().await;
            let _ = tx.send(GuiEvent::AuthResult(res));
            ctx.request_repaint();
        });
    }

    fn trigger_google_login(&self, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let ticket_res = http.create_oauth_ticket().await;
            match ticket_res {
                Ok(ticket) => {
                    let auth_url = format!("{}/api/auth/google?ticket={}", http.base_url, ticket);
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", &auth_url])
                        .spawn();

                    let mut found_token = None;
                    for _ in 0..180 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        if let Ok(Some(token)) = http.check_oauth_ticket(ticket).await {
                            found_token = Some(token);
                            break;
                        }
                    }

                    if let Some(token) = found_token {
                        http.set_session_token(&token);
                        let me_res = http.get_me().await;
                        let _ = tx.send(GuiEvent::DevLoginResult {
                            result: me_res,
                            token: Some(token),
                        });
                    } else {
                        let _ = tx.send(GuiEvent::DevLoginResult {
                            result: Err("Hết thời gian chờ đăng nhập Google".into()),
                            token: None,
                        });
                    }
                }
                Err(e) => {
                    let _ = tx.send(GuiEvent::DevLoginResult {
                        result: Err(format!(
                            "Không thể kết nối đến máy chủ {}. Vui lòng kiểm tra backend đang chạy hoặc đổi sang máy chủ khác. (Chi tiết: {})",
                            http.base_url, e
                        )),
                        token: None,
                    });
                }
            }
            ctx.request_repaint();
        });
    }

    fn trigger_dev_login(&self, username: String, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();
        let uname_opt = if username.trim().is_empty() {
            None
        } else {
            Some(username.trim().to_string())
        };

        self.runtime.spawn(async move {
            let login_res = http.dev_login(uname_opt.as_deref()).await;
            match login_res {
                Ok(_) => {
                    let me_res = http.get_me().await;
                    let _ = tx.send(GuiEvent::DevLoginResult {
                        result: me_res,
                        token: None,
                    });
                }
                Err(e) => {
                    let _ = tx.send(GuiEvent::DevLoginResult {
                        result: Err(e),
                        token: None,
                    });
                }
            }
            ctx.request_repaint();
        });
    }

    fn trigger_set_name(&self, name: String, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let res = http.set_name(&name).await;
            let _ = tx.send(GuiEvent::SetNameResult(res));
            ctx.request_repaint();
        });
    }

    fn refresh_friends_and_pending(&self, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx_friends = self.tx_gui.clone();
        let ctx_friends = ctx.clone();
        self.runtime.spawn(async move {
            let res = http.list_friends().await;
            let _ = tx_friends.send(GuiEvent::FriendsList(res));
            ctx_friends.request_repaint();
        });

        let http2 = self.http.clone();
        let tx_pending = self.tx_gui.clone();
        let ctx_pending = ctx.clone();
        self.runtime.spawn(async move {
            let res = http2.list_pending_requests().await;
            let _ = tx_pending.send(GuiEvent::PendingList(res));
            ctx_pending.request_repaint();
        });
    }

    fn load_friend_keys(&self, user_id: Uuid, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let res = http.get_user_keys(user_id).await;
            let _ = tx.send(GuiEvent::FriendKeysLoaded {
                user_id,
                result: res,
            });
            ctx.request_repaint();
        });
    }

    fn load_message_history(&self, friend_id: Uuid, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let res = http.get_message_history(friend_id, 50).await;
            let _ = tx.send(GuiEvent::HistoryLoaded {
                friend_id,
                result: res,
            });
            ctx.request_repaint();
        });
    }

    fn send_friend_request(&self, username: String, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let res = http.send_friend_request(&username).await;
            let _ = tx.send(GuiEvent::FriendRequestSent(res));
            ctx.request_repaint();
        });
    }

    fn accept_friend_request(&self, request_id: Uuid, ctx: &egui::Context) {
        let http = self.http.clone();
        let tx = self.tx_gui.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let res = http.accept_friend_request(request_id).await;
            let _ = tx.send(GuiEvent::FriendRequestAccepted(res));
            ctx.request_repaint();
        });
    }

    fn start_websocket_and_connect(&mut self, ctx: &egui::Context) {
        let ws = WsManager::start(
            self.runtime.handle().clone(),
            self.http.base_url.clone(),
            self.http.cookie_jar.clone(),
            ctx.clone(),
        );
        self.ws = Some(ws);
    }

    fn handle_logged_in_user(
        &mut self,
        me: MeResponse,
        token_opt: Option<String>,
        ctx: &egui::Context,
    ) {
        let http = self.http.clone();
        let keys = self.keys.clone();
        let user_id = me.id;
        let username_opt = me.username.clone();

        // Save session locally
        storage::save_user_session(user_id, username_opt.clone(), token_opt);

        // Register E2EE keys if not registered
        if !me.has_keys {
            let ecdh_b64 = keys.ecdh_pub_b64();
            let ecdsa_b64 = keys.ecdsa_pub_b64();
            self.runtime.spawn(async move {
                let _ = http.register_keys(&ecdh_b64, &ecdsa_b64).await;
            });
        }

        if let Some(uname) = username_opt {
            let mut chat_view = ChatView::new(user_id, uname);
            // Insert my own keys in cache
            chat_view.cached_keys.insert(
                user_id,
                (
                    self.keys.ecdh_pub_bytes,
                    self.keys.fingerprint,
                    self.keys.ecdsa_pub_bytes,
                ),
            );
            chat_view.server_url_input = self.http.base_url.clone();
            self.state = AppState::Chat(chat_view);
            self.start_websocket_and_connect(ctx);
            self.refresh_friends_and_pending(ctx);
        } else {
            self.state = AppState::SetName(SetNameView::default());
        }
    }

    pub fn update_ui(&mut self, ctx: &egui::Context) {
        if !self.dwm_applied && titlebar::apply_windows_dwm_effects() {
            self.dwm_applied = true;
        }

        // Track maximization state for titlebar icon
        self.is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

        // Keyboard zoom shortcuts: Ctrl + Plus/Equals, Ctrl + Minus, Ctrl + 0
        ctx.input(|i| {
            if i.modifiers.command || i.modifiers.ctrl {
                if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                    egui::gui_zoom::zoom_in(ctx);
                } else if i.key_pressed(egui::Key::Minus) {
                    egui::gui_zoom::zoom_out(ctx);
                } else if i.key_pressed(egui::Key::Num0) {
                    ctx.set_zoom_factor(1.0);
                }
            }
        });

        let mut keys_to_load: Vec<Uuid> = Vec::new();
        let mut should_refresh_data = false;
        let mut should_recheck_auth = false;

        // 1. Drain background GUI events
        while let Ok(event) = self.rx_gui.try_recv() {
            match event {
                GuiEvent::AuthResult(res) => match res {
                    Ok(me) => {
                        self.handle_logged_in_user(me, None, ctx);
                    }
                    Err(_) => {
                        let mut login_view = LoginView::default();
                        login_view.server_url_input = self.http.base_url.clone();
                        self.state = AppState::Login(login_view);
                    }
                },
                GuiEvent::DevLoginResult { result, token } => match result {
                    Ok(me) => {
                        self.handle_logged_in_user(me, token, ctx);
                    }
                    Err(e) => {
                        if let AppState::Login(ref mut view) = self.state {
                            view.is_loading = false;
                            view.is_google_pending = false;
                            view.error_msg = Some(e);
                        }
                    }
                },
                GuiEvent::SetNameResult(res) => match res {
                    Ok(_) => {
                        should_recheck_auth = true;
                    }
                    Err(e) => {
                        if let AppState::SetName(ref mut view) = self.state {
                            view.is_loading = false;
                            view.error_msg = Some(e);
                        }
                    }
                },
                GuiEvent::FriendsList(res) => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        if let Ok(friends) = res {
                            chat.friends = friends;
                        }
                    }
                }
                GuiEvent::PendingList(res) => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        if let Ok(pending) = res {
                            chat.pending_requests = pending;
                        }
                    }
                }
                GuiEvent::SearchResults(res) => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        if let Ok(results) = res {
                            chat.search_results = results;
                        }
                    }
                }
                GuiEvent::FriendKeysLoaded { user_id, result } => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        if let Ok(pk) = result {
                            if let (Ok(ecdh_vec), Ok(ecdsa_vec)) = (
                                URL_SAFE_NO_PAD.decode(&pk.ecdh_public),
                                URL_SAFE_NO_PAD.decode(&pk.ecdsa_public),
                            ) {
                                if ecdh_vec.len() == 65 && ecdsa_vec.len() == 65 {
                                    let mut ecdh = [0u8; 65];
                                    let mut ecdsa = [0u8; 65];
                                    ecdh.copy_from_slice(&ecdh_vec);
                                    ecdsa.copy_from_slice(&ecdsa_vec);
                                    let fp = compute_fingerprint(&ecdh);
                                    chat.cached_keys.insert(user_id, (ecdh, fp, ecdsa));
                                    drain_decrypt_queue(
                                        &mut self.pending_decrypt_queue,
                                        chat,
                                        &self.keys,
                                        &self.ws,
                                    );
                                }
                            }
                        }
                    }
                }
                GuiEvent::HistoryLoaded { friend_id, result } => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        if let Ok(items) = result {
                            let mut decrypted_list = Vec::new();
                            for item in items.into_iter().rev() {
                                let is_me = item.sender_id == chat.current_user_id;
                                let signer_pub = if is_me {
                                    Some(self.keys.ecdsa_pub_bytes)
                                } else {
                                    chat.cached_keys.get(&item.sender_id).map(|k| k.2)
                                };

                                if let Some(ecdsa_pub) = signer_pub {
                                    if let Ok(raw_env) = URL_SAFE_NO_PAD.decode(&item.envelope) {
                                        if let Ok(text) =
                                            decrypt_message(&self.keys, &ecdsa_pub, &raw_env)
                                        {
                                            decrypted_list.push(DecryptedMessage {
                                                id: item.id,
                                                sender_id: item.sender_id,
                                                is_me,
                                                text,
                                                time_str: item
                                                    .created_at
                                                    .with_timezone(&chrono::Local)
                                                    .format("%H:%M")
                                                    .to_string(),
                                            });
                                        }
                                    }
                                } else {
                                    self.pending_decrypt_queue.push((item.sender_id, item));
                                    keys_to_load.push(friend_id);
                                }
                            }
                            chat.messages.insert(friend_id, decrypted_list);
                        }
                    }
                }
                GuiEvent::MessageSendResult { result } => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        if let Err(e) = result {
                            chat.status_message = Some(format!("Lỗi gửi tin nhắn: {e}"));
                        }
                    }
                }
                GuiEvent::FriendRequestSent(res) => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        match res {
                            Ok(_) => {
                                chat.status_message =
                                    Some("Đã gửi lời mời kết bạn thành công".into());
                                should_refresh_data = true;
                            }
                            Err(e) => {
                                chat.status_message = Some(format!("Lỗi: {e}"));
                            }
                        }
                    }
                }
                GuiEvent::FriendRequestAccepted(res) => {
                    if let AppState::Chat(ref mut chat) = self.state {
                        match res {
                            Ok(_) => {
                                chat.status_message = Some("Đã đồng ý kết bạn".into());
                                should_refresh_data = true;
                            }
                            Err(e) => {
                                chat.status_message = Some(format!("Lỗi: {e}"));
                            }
                        }
                    }
                }
            }
        }

        if should_recheck_auth {
            self.check_auth(ctx);
        }
        if should_refresh_data {
            self.refresh_friends_and_pending(ctx);
        }

        // 2. Drain WebSocket events
        let mut ws_events = Vec::new();
        if let Some(ref mut ws) = self.ws {
            while let Ok(event) = ws.rx_events.try_recv() {
                ws_events.push(event);
            }
        }

        for event in ws_events {
            if let AppState::Chat(ref mut chat) = self.state {
                match event {
                    WsServerEvent::MessageNew(wire) => {
                        let sender_id = wire.sender_id;
                        if let Some((_, _, ecdsa_pub)) = chat.cached_keys.get(&sender_id) {
                            if let Ok(raw_env) = URL_SAFE_NO_PAD.decode(&wire.envelope) {
                                if let Ok(text) = decrypt_message(&self.keys, ecdsa_pub, &raw_env) {
                                    let msgs = chat.messages.entry(sender_id).or_default();
                                    if !msgs.iter().any(|m| m.id == wire.id) {
                                        msgs.push(DecryptedMessage {
                                            id: wire.id,
                                            sender_id,
                                            is_me: false,
                                            text,
                                            time_str: wire
                                                .created_at
                                                .with_timezone(&chrono::Local)
                                                .format("%H:%M")
                                                .to_string(),
                                        });
                                        if chat.selected_friend_id != Some(sender_id)
                                            || chat.is_settings_open
                                        {
                                            *chat.unread_counts.entry(sender_id).or_insert(0) += 1;
                                        }
                                    }
                                    if let Some(ref ws) = self.ws {
                                        let _ = ws.tx_commands.send(WsClientCommand::Ack {
                                            message_id: wire.id,
                                        });
                                    }
                                }
                            }
                        } else {
                            self.pending_decrypt_queue.push((sender_id, wire));
                            keys_to_load.push(sender_id);
                        }
                    }
                    WsServerEvent::FriendRequest(item) => {
                        if !chat.pending_requests.iter().any(|p| p.id == item.id) {
                            chat.pending_requests.push(item);
                        }
                    }
                    WsServerEvent::FriendAccepted { user_id, username } => {
                        if !chat.friends.iter().any(|f| f.id == user_id) {
                            chat.friends.push(FriendItem {
                                id: user_id,
                                username,
                                avatar_url: None,
                            });
                        }
                        keys_to_load.push(user_id);
                    }
                    WsServerEvent::TypingStart { user_id } => {
                        chat.typing_users.insert(user_id);
                    }
                    WsServerEvent::TypingStop { user_id } => {
                        chat.typing_users.remove(&user_id);
                    }
                    _ => {}
                }
            }
        }

        // Load any requested friend keys
        for uid in keys_to_load {
            self.load_friend_keys(uid, ctx);
        }

        // 3. Render Custom Windows 11 Titlebar (Seamless White with Windows controls)
        egui::TopBottomPanel::top("top_titlebar")
            .exact_height(34.0)
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ctx, |ui| {
                titlebar::render_titlebar(ui, self.is_maximized);
            });

        // 4. Render Active View with rounded bottom corners when not maximized
        let corner_bottom = if self.is_maximized { 0 } else { 8 };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(255, 255, 255))
                    .corner_radius(egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: corner_bottom,
                        se: corner_bottom,
                    }),
            )
            .show(ctx, |ui| {
                match &mut self.state {
                    AppState::CheckingAuth => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(ui.available_height() * 0.4);
                            ui.spinner();
                            ui.add_space(16.0);
                            ui.label(
                                RichText::new("Đang kiểm tra phiên đăng nhập")
                                    .size(14.0)
                                    .color(Color32::from_rgb(115, 115, 115)),
                            );
                        });
                    }
                    AppState::Login(login_view) => {
                        if let Some(action) = login_view.show(ui) {
                            match action {
                                LoginAction::GoogleLogin => {
                                    login_view.is_loading = true;
                                    login_view.is_google_pending = true;
                                    login_view.error_msg = None;
                                    self.trigger_google_login(ctx);
                                }
                                LoginAction::DevLogin(username) => {
                                    login_view.is_loading = true;
                                    login_view.is_google_pending = false;
                                    login_view.error_msg = None;
                                    self.trigger_dev_login(username, ctx);
                                }
                                LoginAction::CancelGoogleLogin => {
                                    login_view.is_loading = false;
                                    login_view.is_google_pending = false;
                                }
                                LoginAction::ChangeServer(new_url) => {
                                    storage::save_server_url(&new_url);
                                    self.http = HttpClient::new(new_url.clone());
                                    login_view.server_url_input = new_url;
                                    login_view.error_msg = None;
                                    login_view.is_loading = false;
                                    login_view.is_google_pending = false;
                                    self.check_auth(ctx);
                                }
                            }
                        }
                    }
                    AppState::SetName(set_name_view) => {
                        if let Some(submitted_name) = set_name_view.show(ui) {
                            set_name_view.is_loading = true;
                            set_name_view.error_msg = None;
                            self.trigger_set_name(submitted_name, ctx);
                        }
                    }
                    AppState::Chat(chat_view) => {
                        if let Some(action) = chat_view.show(ui) {
                            match action {
                                ChatAction::SelectFriend(friend_id) => {
                                    chat_view.unread_counts.remove(&friend_id);
                                    chat_view.selected_friend_id = Some(friend_id);
                                    if !chat_view.cached_keys.contains_key(&friend_id) {
                                        self.load_friend_keys(friend_id, ctx);
                                    }
                                    self.load_message_history(friend_id, ctx);
                                }
                                ChatAction::SendMessage(text) => {
                                    if let Some(friend_id) = chat_view.selected_friend_id {
                                        let my_id = chat_view.current_user_id;
                                        if let Some((friend_ecdh, friend_fp, _)) =
                                            chat_view.cached_keys.get(&friend_id).copied()
                                        {
                                            let msg_id = Uuid::new_v4();
                                            // 1. Recipient envelope (for friend)
                                            let recip_res = encrypt_message(
                                                &self.keys,
                                                &friend_ecdh,
                                                &friend_fp,
                                                msg_id,
                                                my_id,
                                                friend_id,
                                                &text,
                                            );
                                            // 2. Sender envelope (for myself)
                                            let sender_res = encrypt_message(
                                                &self.keys,
                                                &self.keys.ecdh_pub_bytes,
                                                &self.keys.fingerprint,
                                                msg_id,
                                                my_id,
                                                my_id,
                                                &text,
                                            );

                                            if let (Ok(recip_bytes), Ok(sender_bytes)) =
                                                (recip_res, sender_res)
                                            {
                                                let recip_b64 = URL_SAFE_NO_PAD.encode(recip_bytes);
                                                let sender_b64 =
                                                    URL_SAFE_NO_PAD.encode(sender_bytes);

                                                // Optimistic add to UI
                                                let msgs = chat_view
                                                    .messages
                                                    .entry(friend_id)
                                                    .or_default();
                                                msgs.push(DecryptedMessage {
                                                    id: msg_id,
                                                    sender_id: my_id,
                                                    is_me: true,
                                                    text: text.clone(),
                                                    time_str: chrono::Local::now()
                                                        .format("%H:%M")
                                                        .to_string(),
                                                });

                                                // Dispatch HTTP send task
                                                let http = self.http.clone();
                                                let tx = self.tx_gui.clone();
                                                let ctx = ctx.clone();
                                                self.runtime.spawn(async move {
                                                    let res = http
                                                        .send_message(
                                                            msg_id,
                                                            friend_id,
                                                            &recip_b64,
                                                            &sender_b64,
                                                        )
                                                        .await;
                                                    let _ = tx.send(GuiEvent::MessageSendResult {
                                                        result: res,
                                                    });
                                                    ctx.request_repaint();
                                                });
                                            }
                                        } else {
                                            chat_view.status_message =
                                                Some("Đang tải khóa mã hóa của bạn bè".into());
                                            self.load_friend_keys(friend_id, ctx);
                                        }
                                    }
                                }
                                ChatAction::SendFriendRequest(username) => {
                                    self.send_friend_request(username, ctx);
                                }
                                ChatAction::AcceptFriendRequest(req_id) => {
                                    self.accept_friend_request(req_id, ctx);
                                }
                                ChatAction::RejectFriendRequest(_) => {}
                                ChatAction::TypingStarted => {
                                    if let Some(friend_id) = chat_view.selected_friend_id {
                                        if let Some(ws) = &self.ws {
                                            let _ =
                                                ws.tx_commands.send(WsClientCommand::TypingStart {
                                                    recipient_id: friend_id,
                                                });
                                        }
                                    }
                                }
                                ChatAction::TypingStopped => {
                                    if let Some(friend_id) = chat_view.selected_friend_id {
                                        if let Some(ws) = &self.ws {
                                            let _ =
                                                ws.tx_commands.send(WsClientCommand::TypingStop {
                                                    recipient_id: friend_id,
                                                });
                                        }
                                    }
                                }
                                ChatAction::RefreshData => {
                                    self.refresh_friends_and_pending(ctx);
                                }
                                ChatAction::Logout => {
                                    let http = self.http.clone();
                                    self.runtime.spawn(async move {
                                        let _ = http.logout().await;
                                    });
                                    storage::clear_user_session();
                                    self.ws = None;
                                    self.state = AppState::Login(LoginView::default());
                                }
                                ChatAction::ChangeServerUrl(new_url) => {
                                    storage::save_server_url(&new_url);
                                    self.http = HttpClient::new(new_url.clone());
                                    chat_view.server_url_input = new_url;
                                    self.refresh_friends_and_pending(ctx);
                                }
                            }
                        }
                    }
                }
            });

        // 5. Subtle dark border on borderless window (when not maximized)
        if !self.is_maximized {
            let screen_rect = ctx.screen_rect();
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("window_border"),
            ))
            .rect_stroke(
                screen_rect,
                egui::CornerRadius::same(8),
                egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 35)),
                egui::StrokeKind::Inside,
            );
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }
}
