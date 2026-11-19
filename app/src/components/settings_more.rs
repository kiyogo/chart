//! More settings pages: chat behaviour, notifications, privacy & blocking,
//! app lock (PIN), automatic replies, language and deep appearance options.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        chat_extras::WALLPAPER_NAMES,
        icons::{BellIcon, BlockIcon, ImageIcon, LockIcon},
    },
    i18n::{LANGUAGES, t},
    models::{BlockedUser, PrivacySettings},
    platform,
    prefs::{Prefs, PrefsCtx},
    services::{library, social},
};

fn ctx() -> PrefsCtx {
    use_context::<PrefsCtx>()
}

#[component]
pub fn Toggle(label: String, #[props(default)] desc: String, checked: bool, on_change: EventHandler<bool>, #[props(default)] disabled: bool) -> Element {
    rsx! {
        label { class: "settings-row toggle-row",
            span { class: "grow",
                span { class: "settings-label", "{label}" }
                if !desc.is_empty() { span { class: "settings-row-desc", "{desc}" } }
            }
            input { r#type: "checkbox", class: "switch", checked: checked, disabled: disabled, onchange: move |e| on_change.call(e.checked()) }
        }
    }
}

#[component]
pub fn Segmented(options: Vec<(String, String)>, value: String, on_change: EventHandler<String>, #[props(default)] label: String) -> Element {
    rsx! {
        div { class: "segmented", role: "radiogroup", aria_label: "{label}",
            for (v, l) in options {
                {
                    let v2 = v.clone();
                    rsx! {
                        button { key: "{v}", r#type: "button", role: "radio", aria_checked: "{value == v}", class: if value == v { "active" } else { "" },
                            onclick: move |_| on_change.call(v2.clone()),
                            "{l}"
                        }
                    }
                }
            }
        }
    }
}

fn opts(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// Server-side privacy settings shared by several pages.
fn use_privacy() -> (Signal<Option<PrivacySettings>>, Signal<Option<String>>) {
    let mut p = use_signal(|| Option::<PrivacySettings>::None);
    let err = use_signal(|| Option::<String>::None);
    use_hook(move || {
        spawn(async move {
            if let Ok(s) = social::privacy_get().await {
                p.set(Some(s));
            }
        });
    });
    (p, err)
}

fn save_privacy(mut p: Signal<Option<PrivacySettings>>, mut err: Signal<Option<String>>, fields: serde_json::Value) {
    spawn(async move {
        match social::privacy_set(fields).await {
            Ok(s) => {
                p.set(Some(s));
                err.set(None);
            }
            Err(_) => err.set(Some("Không lưu được thay đổi".into())),
        }
    });
}

#[component]
pub fn ChatPrefsSettings() -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let (privacy, err) = use_privacy();
    let pv = privacy().unwrap_or_default();
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Khi trò chuyện" }
            Toggle { label: "Bấm đúp vùng cạnh tin nhắn để trả lời", desc: "Bấm đúp vào khoảng trống bên cạnh một tin nhắn để trả lời tin đó", checked: pr.double_click_reply, on_change: move |v| c.update(|p| p.double_click_reply = v) }
            Toggle { label: "Nhấn Enter để gửi", desc: "Tắt để Enter xuống dòng và Ctrl + Enter để gửi", checked: pr.enter_to_send, on_change: move |v| c.update(|p| p.enter_to_send = v) }
            Toggle { label: "Gợi ý sticker phù hợp với nội dung đang soạn", desc: "Ví dụ gõ \"cảm ơn\" sẽ gợi ý sticker Thank you", checked: pr.sticker_suggest, on_change: move |v| c.update(|p| p.sticker_suggest = v) }
            Toggle { label: "Gợi ý @ nhắc tên", desc: "Gõ @ để chọn người cần nhắc trong nhóm hoặc cuộc trò chuyện", checked: pr.mention_suggest, on_change: move |v| c.update(|p| p.mention_suggest = v) }
            Toggle { label: "Gợi ý emoji khi gõ dấu hai chấm", desc: "Ví dụ :smile, :heart, :fire", checked: pr.emoji_suggest, on_change: move |v| c.update(|p| p.emoji_suggest = v) }
            Toggle { label: "Hiện ảnh đại diện trong tin nhắn", checked: pr.show_avatars, on_change: move |v| c.update(|p| p.show_avatars = v) }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Hiển thị nội dung Excel khi dán" }
            p { class: "settings-group-desc", "Khi dán các ô từ Excel, Google Sheets hoặc bảng tính khác vào ô soạn tin." }
            Segmented { label: "Cách dán bảng tính", value: pr.paste_mode.clone(), options: opts(&[("ask", "Luôn hỏi"), ("image", "Dạng ảnh"), ("table", "Dạng bảng"), ("text", "Dạng chữ")]), on_change: move |v: String| c.update(|p| p.paste_mode = v) }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Trạng thái" }
            if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
            Toggle { label: "Hiện trạng thái \"đang soạn tin\"", desc: "Bạn bè thấy khi bạn đang soạn tin và bạn cũng thấy họ. Tắt thì cả hai chiều đều ẩn.", checked: pv.typing, disabled: privacy().is_none(),
                on_change: move |v: bool| save_privacy(privacy, err, json!({ "typing": v })) }
            Toggle { label: "Hiện \"Đã xem\"", desc: "Tắt để ẩn đã xem: người khác không biết bạn đã đọc và bạn cũng không thấy họ đã đọc.", checked: pv.read_receipts, disabled: privacy().is_none(),
                on_change: move |v: bool| save_privacy(privacy, err, json!({ "read_receipts": v })) }
        }
    }
}

#[component]
pub fn NotificationSettings() -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let mut perm = use_signal(|| Option::<String>::None);
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", BellIcon {} " Tùy chọn thông báo" }
            Toggle { label: "Nhận thông báo khi có tin nhắn mới", desc: "Thông báo hệ thống khi ChatW không ở trên cùng", checked: pr.notify,
                on_change: move |v: bool| {
                    c.update(|p| p.notify = v);
                    if v {
                        spawn(async move {
                            let r = platform::request_notification_permission().await;
                            perm.set(Some(r));
                        });
                    }
                },
            }
            if let Some(p) = perm() {
                div { class: if p == "granted" { "status-badge success" } else { "status-badge error" },
                    match p.as_str() {
                        "granted" => "Đã cho phép thông báo",
                        "denied" => "Trình duyệt đang chặn thông báo. Hãy cho phép trong cài đặt trang web.",
                        _ => "Thiết bị này không hỗ trợ thông báo",
                    }
                }
            }
            Toggle { label: "Hiện nội dung tin nhắn trong thông báo", desc: "Tắt để thông báo chỉ ghi tên người gửi", checked: pr.notify_preview, on_change: move |v| c.update(|p| p.notify_preview = v) }
            Toggle { label: "Thông báo tin nhắn nhóm", checked: pr.notify_groups, on_change: move |v| c.update(|p| p.notify_groups = v) }
            Toggle { label: "Thông báo cuộc gọi đến", checked: pr.notify_calls, on_change: move |v| c.update(|p| p.notify_calls = v) }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Âm thanh" }
            Toggle { label: "Phát âm thanh khi có thông báo", checked: pr.sound, on_change: move |v| c.update(|p| p.sound = v) }
            div { class: "settings-row",
                span { class: "settings-label", "Âm báo" }
                Segmented { label: "Âm báo", value: pr.sound_name.clone(), options: opts(&[("pop", "Pop"), ("chime", "Chuông"), ("soft", "Nhẹ"), ("bubble", "Bong bóng")]),
                    on_change: move |v: String| { platform::play_sound(&v); c.update(|p| p.sound_name = v); },
                }
            }
            p { class: "settings-group-desc", "Tắt thông báo cho từng cuộc trò chuyện trong menu ⋯ của cuộc trò chuyện đó." }
        }
    }
}

#[component]
pub fn PrivacySettingsPanel() -> Element {
    let (privacy, err) = use_privacy();
    let pv = privacy().unwrap_or_default();
    let mut blocked = use_signal(Vec::<BlockedUser>::new);
    use_hook(move || {
        spawn(async move {
            if let Ok(b) = social::blocks_list().await {
                blocked.set(b);
            }
        });
    });
    rsx! {
        if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
        div { class: "settings-group",
            div { class: "settings-group-title", "Ai có thể liên hệ với bạn" }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Ai được gửi lời mời kết bạn" }
                Segmented { label: "Lời mời kết bạn", value: pv.friend_requests.clone(), options: opts(&[("everyone", "Mọi người"), ("nobody", "Không ai")]),
                    on_change: move |v: String| save_privacy(privacy, err, json!({ "friend_requests": v })) }
            }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Ai được gọi cho bạn" }
                Segmented { label: "Cuộc gọi", value: pv.calls.clone(), options: opts(&[("friends", "Bạn bè"), ("nobody", "Không ai")]),
                    on_change: move |v: String| save_privacy(privacy, err, json!({ "calls": v })) }
            }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Ai được thêm bạn vào nhóm" }
                Segmented { label: "Thêm vào nhóm", value: pv.group_invites.clone(), options: opts(&[("everyone", "Mọi người"), ("friends", "Bạn bè"), ("nobody", "Không ai")]),
                    on_change: move |v: String| save_privacy(privacy, err, json!({ "group_invites": v })) }
            }
            Toggle { label: "Cho phép tìm thấy tôi bằng tên tài khoản", desc: "Tắt để chỉ bạn bè mới thấy bạn trong tìm kiếm", checked: pv.discoverable, disabled: privacy().is_none(),
                on_change: move |v: bool| save_privacy(privacy, err, json!({ "discoverable": v })) }
            Toggle { label: "Hiện \"Đã xem\"", checked: pv.read_receipts, disabled: privacy().is_none(), on_change: move |v: bool| save_privacy(privacy, err, json!({ "read_receipts": v })) }
            Toggle { label: "Hiện trạng thái \"đang soạn tin\"", checked: pv.typing, disabled: privacy().is_none(), on_change: move |v: bool| save_privacy(privacy, err, json!({ "typing": v })) }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", BlockIcon { size: 18 } " Danh sách chặn" }
            p { class: "settings-group-desc", "Người bị chặn không thể nhắn tin, gọi, gửi lời mời kết bạn hay thêm bạn vào nhóm. Họ không được thông báo." }
            if blocked.read().is_empty() { div { class: "sidebar-empty", "Bạn chưa chặn ai" } }
            for b in blocked() {
                {
                    let id = b.id.clone();
                    rsx! {
                        div { key: "{b.id}", class: "member-row",
                            Avatar { name: b.username.clone(), avatar_url: b.avatar_url.clone(), size: 36 }
                            span { class: "grow member-name", "{b.username}" }
                            button { r#type: "button", class: "btn-secondary",
                                onclick: move |_| { let id = id.clone(); spawn(async move {
                                    if social::unblock_user(&id).await.is_ok() { blocked.with_mut(|l| l.retain(|x| x.id != id)); }
                                }); },
                                "Bỏ chặn"
                            }
                        }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Mã hóa đầu cuối" }
            p { class: "settings-group-desc", "Tin nhắn riêng, \"Chính mình\", ảnh và thẻ mời trong tin nhắn riêng được mã hóa trên thiết bị. Máy chủ chỉ chuyển tiếp và không đọc được. Tin nhắn nhóm, kênh cộng đồng và plugin được máy chủ lưu ở dạng thường." }
        }
    }
}

/// Salted PBKDF2-SHA256 (210 000 rounds) via WebCrypto; the PIN never leaves memory.
pub async fn hash_pin(pin: &str, salt_b64: &str) -> Option<String> {
    let p = serde_json::to_string(pin).ok()?;
    let s = serde_json::to_string(salt_b64).ok()?;
    let mut ev = dioxus::document::eval(&format!(
        r#"
        (async () => {{
            try {{
                const enc = new TextEncoder();
                const salt = Uint8Array.from(atob({s}), (c) => c.charCodeAt(0));
                const key = await crypto.subtle.importKey('raw', enc.encode({p}), 'PBKDF2', false, ['deriveBits']);
                const bits = await crypto.subtle.deriveBits({{ name: 'PBKDF2', hash: 'SHA-256', salt, iterations: 210000 }}, key, 256);
                dioxus.send(btoa(String.fromCharCode(...new Uint8Array(bits))));
            }} catch (e) {{ dioxus.send(''); }}
        }})();
        "#
    ));
    ev.recv::<String>().await.ok().filter(|h| !h.is_empty())
}

pub async fn new_salt() -> String {
    let mut ev = dioxus::document::eval("const a = new Uint8Array(16); crypto.getRandomValues(a); dioxus.send(btoa(String.fromCharCode(...a)));");
    ev.recv::<String>().await.unwrap_or_default()
}

pub fn same(a: &str, b: &str) -> bool {
    let (x, y) = (a.as_bytes(), b.as_bytes());
    x.len() == y.len() && x.iter().zip(y).fold(0u8, |acc, (p, q)| acc | (p ^ q)) == 0
}

pub fn valid_pin(pin: &str) -> bool {
    (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit())
}

#[component]
pub fn LockSettings(on_lock_now: EventHandler<()>) -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let mut pin1 = use_signal(String::new);
    let mut pin2 = use_signal(String::new);
    let mut current = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);
    let has_pin = !pr.pin_hash.is_empty();
    let set_pin = move |_| {
        let (a, b) = (pin1(), pin2());
        if !valid_pin(&a) {
            msg.set(Some((true, "Mã PIN gồm 4 đến 8 chữ số".into())));
            return;
        }
        if a != b {
            msg.set(Some((true, "Hai lần nhập mã PIN không khớp".into())));
            return;
        }
        busy.set(true);
        let old = current();
        spawn(async move {
            let p = c.prefs.peek().clone();
            if !p.pin_hash.is_empty() {
                let ok = hash_pin(&old, &p.pin_salt).await.is_some_and(|h| same(&h, &p.pin_hash));
                if !ok {
                    msg.set(Some((true, "Mã PIN hiện tại không đúng".into())));
                    busy.set(false);
                    return;
                }
            }
            let salt = new_salt().await;
            match hash_pin(&a, &salt).await {
                Some(h) => {
                    c.update(move |p| {
                        p.pin_salt = salt;
                        p.pin_hash = h;
                        p.lock_enabled = true;
                    });
                    pin1.set(String::new());
                    pin2.set(String::new());
                    current.set(String::new());
                    msg.set(Some((false, "Đã đặt mã PIN. ChatW sẽ khóa khi bạn rời đi quá lâu.".into())));
                }
                None => msg.set(Some((true, "Thiết bị không hỗ trợ tạo mã PIN an toàn".into()))),
            }
            busy.set(false);
        });
    };
    let remove_pin = move |_| {
        let old = current();
        spawn(async move {
            let p = c.prefs.peek().clone();
            if hash_pin(&old, &p.pin_salt).await.is_some_and(|h| same(&h, &p.pin_hash)) {
                c.update(|p| {
                    p.pin_hash.clear();
                    p.pin_salt.clear();
                    p.lock_enabled = false;
                });
                current.set(String::new());
                msg.set(Some((false, "Đã tắt mã PIN".into())));
            } else {
                msg.set(Some((true, "Mã PIN hiện tại không đúng".into())));
            }
        });
    };
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", LockIcon { size: 18 } " Mã PIN khóa ứng dụng" }
            p { class: "settings-group-desc", "Khóa giao diện ChatW trên thiết bị này khi bạn không hoạt động quá lâu hoặc mở lại ứng dụng. Mã PIN chỉ lưu dạng băm có muối PBKDF2 trên thiết bị, không gửi lên máy chủ." }
            if has_pin {
                Toggle { label: "Bật khóa bằng mã PIN", checked: pr.lock_enabled, on_change: move |v| c.update(|p| p.lock_enabled = v) }
                Toggle { label: "Yêu cầu mã PIN mỗi khi mở ChatW", checked: pr.lock_on_start, on_change: move |v| c.update(|p| p.lock_on_start = v) }
                div { class: "settings-row",
                    span { class: "settings-label", "Khóa sau khi không hoạt động" }
                    Segmented { label: "Thời gian khóa", value: pr.lock_minutes.to_string(), options: opts(&[("1", "1 phút"), ("5", "5 phút"), ("15", "15 phút"), ("30", "30 phút"), ("60", "1 giờ")]),
                        on_change: move |v: String| c.update(|p| p.lock_minutes = v.parse().unwrap_or(5)) }
                }
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-secondary", disabled: !pr.lock_enabled, onclick: move |_| on_lock_now.call(()), LockIcon { size: 16 } "Khóa ngay" }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", if has_pin { "Đổi hoặc tắt mã PIN" } else { "Đặt mã PIN" } }
            div { class: "form-grid",
                if has_pin {
                    label { class: "form-field", span { "Mã PIN hiện tại" }
                        input { r#type: "password", class: "pin-input", inputmode: "numeric", autocomplete: "off", maxlength: "8", value: "{current}", oninput: move |e| current.set(e.value().chars().filter(|c| c.is_ascii_digit()).collect()) }
                    }
                }
                label { class: "form-field", span { "Mã PIN mới từ 4 đến 8 số" }
                    input { r#type: "password", class: "pin-input", inputmode: "numeric", autocomplete: "new-password", maxlength: "8", value: "{pin1}", oninput: move |e| pin1.set(e.value().chars().filter(|c| c.is_ascii_digit()).collect()) }
                }
                label { class: "form-field", span { "Nhập lại mã PIN mới" }
                    input { r#type: "password", class: "pin-input", inputmode: "numeric", autocomplete: "new-password", maxlength: "8", value: "{pin2}", oninput: move |e| pin2.set(e.value().chars().filter(|c| c.is_ascii_digit()).collect()) }
                }
            }
            if let Some((is_err, m)) = msg() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{m}" } }
            div { class: "inline-form",
                button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: set_pin, if has_pin { "Đổi mã PIN" } else { "Đặt mã PIN" } }
                if has_pin { button { r#type: "button", class: "btn-danger-outline", onclick: remove_pin, "Tắt mã PIN" } }
            }
        }
    }
}

#[component]
pub fn AutoReplySettings() -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let mut text = use_signal(|| pr.auto_reply_text.clone());
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Tin nhắn tự động khi vắng mặt" }
            p { class: "settings-group-desc", "Tự động gửi một tin nhắn khi có người nhắn cho bạn, tối đa một lần mỗi 6 giờ cho mỗi người. Vì tin nhắn được mã hóa đầu cuối, máy chủ không thể trả lời thay bạn: trả lời tự động hoạt động khi ChatW đang mở trên ít nhất một thiết bị của bạn." }
            Toggle { label: "Bật trả lời tự động", checked: pr.auto_reply, on_change: move |v| c.update(|p| p.auto_reply = v) }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Khi nào trả lời" }
                Segmented { label: "Khi nào trả lời", value: pr.auto_reply_mode.clone(), options: opts(&[("away", "Khi tôi vắng mặt"), ("hours", "Theo khung giờ"), ("always", "Luôn luôn")]),
                    on_change: move |v: String| c.update(|p| p.auto_reply_mode = v) }
                if pr.auto_reply_mode == "hours" {
                    div { class: "inline-form",
                        label { class: "form-field", span { "Từ" } input { r#type: "time", value: "{pr.auto_reply_from}", onchange: move |e| { let v = e.value(); c.update(move |p| p.auto_reply_from = v) } } }
                        label { class: "form-field", span { "Đến" } input { r#type: "time", value: "{pr.auto_reply_to}", onchange: move |e| { let v = e.value(); c.update(move |p| p.auto_reply_to = v) } } }
                    }
                }
            }
            label { class: "form-field",
                span { "Nội dung" }
                textarea { rows: "3", maxlength: "500", value: "{text}", oninput: move |e| text.set(e.value()), onchange: move |_| { let v = text(); c.update(move |p| p.auto_reply_text = v) } }
            }
            p { class: "settings-group-desc", "Tin trả lời tự động được đánh dấu [Tự động] để người nhận biết." }
        }
    }
}

#[component]
pub fn LanguageSettings() -> Element {
    let c = ctx();
    let cur = c.prefs.read().language.clone();
    let _ = t;
    rsx! {
        div { class: "settings-group",
            div { class: "lang-grid", role: "radiogroup", aria_label: "Ngôn ngữ",
                for (code, native, _english) in LANGUAGES.iter().copied() {
                    button { key: "{code}", r#type: "button", role: "radio", aria_checked: "{cur == code}", lang: "{code}",
                        class: if cur == code { "lang-tile active" } else { "lang-tile" },
                        onclick: move |_| c.update(|p| p.language = code.to_string()),
                        span { class: "lang-native", "{native}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdvancedLookSettings() -> Element {
    let c = ctx();
    let pr: Prefs = c.prefs.read().clone();
    let mut uploading = use_signal(|| false);
    let mut msg = use_signal(|| Option::<String>::None);
    let accent_val = if pr.custom_accent.is_empty() { "#2764E7".to_string() } else { pr.custom_accent.clone() };
    let self_val = if pr.bubble_self.is_empty() { accent_val.clone() } else { pr.bubble_self.clone() };
    let other_val = if pr.bubble_other.is_empty() { "#EEF1F5".to_string() } else { pr.bubble_other.clone() };
    let wp_color = pr.app_wallpaper.strip_prefix("color:").unwrap_or("#1F2A44").to_string();
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Màu sắc" }
            div { class: "settings-row",
                span { class: "grow", span { class: "settings-label", "Màu nhấn tùy chọn" } span { class: "settings-row-desc", "Thay cho bảng màu nhấn có sẵn" } }
                input { r#type: "color", aria_label: "Màu nhấn", value: "{accent_val}", onchange: move |e| { let v = e.value().to_uppercase(); c.update(move |p| p.custom_accent = v) } }
                if !pr.custom_accent.is_empty() { button { r#type: "button", class: "btn-ghost", onclick: move |_| c.update(|p| p.custom_accent.clear()), "Mặc định" } }
            }
            div { class: "settings-row",
                span { class: "grow", span { class: "settings-label", "Bong bóng tin nhắn của bạn" } }
                input { r#type: "color", aria_label: "Màu bong bóng của bạn", value: "{self_val}", onchange: move |e| { let v = e.value().to_uppercase(); c.update(move |p| p.bubble_self = v) } }
                if !pr.bubble_self.is_empty() { button { r#type: "button", class: "btn-ghost", onclick: move |_| c.update(|p| p.bubble_self.clear()), "Mặc định" } }
            }
            div { class: "settings-row",
                span { class: "grow", span { class: "settings-label", "Bong bóng tin nhắn người khác" } }
                input { r#type: "color", aria_label: "Màu bong bóng người khác", value: "{other_val}", onchange: move |e| { let v = e.value().to_uppercase(); c.update(move |p| p.bubble_other = v) } }
                if !pr.bubble_other.is_empty() { button { r#type: "button", class: "btn-ghost", onclick: move |_| c.update(|p| p.bubble_other.clear()), "Mặc định" } }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Hình dạng và bố cục" }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Bong bóng" }
                Segmented { label: "Bong bóng", value: pr.bubble_shape.clone(), options: opts(&[("round", "Tròn"), ("soft", "Mềm"), ("square", "Vuông"), ("leaf", "Chiếc lá")]), on_change: move |v: String| c.update(|p| p.bubble_shape = v) }
            }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Ảnh đại diện" }
                Segmented { label: "Ảnh đại diện", value: pr.avatar_shape.clone(), options: opts(&[("circle", "Tròn"), ("squircle", "Bo góc mềm"), ("square", "Vuông")]), on_change: move |v: String| c.update(|p| p.avatar_shape = v) }
            }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Độ rộng tin nhắn" }
                Segmented { label: "Độ rộng tin nhắn", value: pr.chat_width.clone(), options: opts(&[("narrow", "Hẹp"), ("normal", "Vừa"), ("wide", "Rộng"), ("full", "Toàn màn hình")]), on_change: move |v: String| c.update(|p| p.chat_width = v) }
            }
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Phông chữ" }
                Segmented { label: "Phông chữ", value: pr.font_family.clone(), options: opts(&[("system", "Hệ thống"), ("rounded", "Bo tròn"), ("serif", "Có chân"), ("mono", "Đơn cách"), ("dyslexic", "Dễ đọc")]), on_change: move |v: String| c.update(|p| p.font_family = v) }
            }
            Toggle { label: "Giảm chuyển động", desc: "Tắt hiệu ứng động và chuyển cảnh", checked: pr.reduce_motion, on_change: move |v| c.update(|p| p.reduce_motion = v) }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", ImageIcon { size: 18 } " Hình nền toàn ứng dụng" }
            p { class: "settings-group-desc", "Hình nền phía sau toàn bộ ChatW trên thiết bị này. Hình nền riêng cho từng đoạn chat đặt trong menu của đoạn chat đó." }
            div { class: "wallpaper-grid",
                button { r#type: "button", class: if pr.app_wallpaper.is_empty() { "wallpaper-tile active" } else { "wallpaper-tile" }, onclick: move |_| c.update(|p| p.app_wallpaper.clear()),
                    span { class: "wallpaper-sample wp-none" } span { "Không" }
                }
                for (id, name) in WALLPAPER_NAMES.iter().copied().filter(|(id, _)| *id != "none") {
                    button { key: "{id}", r#type: "button", class: if pr.app_wallpaper == id { "wallpaper-tile active" } else { "wallpaper-tile" },
                        onclick: move |_| c.update(|p| p.app_wallpaper = id.to_string()),
                        span { class: "wallpaper-sample wp-{id}" } span { "{name}" }
                    }
                }
            }
            div { class: "inline-form",
                input { r#type: "color", aria_label: "Màu nền ứng dụng", value: "{wp_color}", onchange: move |e| { let v = format!("color:{}", e.value().to_uppercase()); c.update(move |p| p.app_wallpaper = v) } }
                input { id: "app-wp-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                    onchange: move |_| {
                        uploading.set(true);
                        spawn(async move {
                            match platform::read_selected_file_raw("app-wp-file").await {
                                Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                    Ok(url) => c.update(move |p| p.app_wallpaper = url),
                                    Err(_) => msg.set(Some("Không tải ảnh lên được".into())),
                                },
                                Err(e) if e == "too_large" => msg.set(Some("Ảnh lớn hơn 2 MB".into())),
                                Err(_) => {}
                            }
                            uploading.set(false);
                        });
                    },
                }
                label { r#for: "app-wp-file", class: "btn-secondary", role: "button", tabindex: "0", ImageIcon { size: 16 } if uploading() { "Đang tải lên" } else { "Tải ảnh của bạn" } }
            }
            if let Some(m) = msg() { div { class: "status-badge error", "{m}" } }
            if !pr.app_wallpaper.is_empty() {
                div { class: "settings-row",
                    span { class: "settings-label", "Độ tối phủ lên ảnh: {pr.app_wallpaper_dim}%" }
                    input { r#type: "range", min: "0", max: "80", value: "{pr.app_wallpaper_dim}", aria_label: "Độ tối", oninput: move |e| { let v: u8 = e.value().parse().unwrap_or(20); c.update(move |p| p.app_wallpaper_dim = v) } }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Làm mờ: {pr.app_wallpaper_blur}px" }
                    input { r#type: "range", min: "0", max: "24", value: "{pr.app_wallpaper_blur}", aria_label: "Độ mờ", oninput: move |e| { let v: u8 = e.value().parse().unwrap_or(0); c.update(move |p| p.app_wallpaper_blur = v) } }
                }
            }
        }
        div { class: "settings-group",
            button { r#type: "button", class: "btn-ghost", onclick: move |_| c.update(|p| {
                let d = Prefs::default();
                p.custom_accent = d.custom_accent; p.bubble_self = d.bubble_self; p.bubble_other = d.bubble_other;
                p.bubble_shape = d.bubble_shape; p.avatar_shape = d.avatar_shape; p.chat_width = d.chat_width;
                p.font_family = d.font_family; p.app_wallpaper = d.app_wallpaper; p.app_wallpaper_dim = d.app_wallpaper_dim;
                p.app_wallpaper_blur = d.app_wallpaper_blur; p.reduce_motion = d.reduce_motion;
            }), "Khôi phục giao diện mặc định" }
        }
    }
}

/// Spec 009: settings sync between devices and the avatar-motion budget for lists.
#[component]
pub fn SyncMotionSettings() -> Element {
    let ctx = use_context::<PrefsCtx>();
    let p = ctx.prefs.read().clone();
    let version = crate::services::settings_sync::known_version();
    rsx! {
        div { class: "settings-section",
            div { class: "settings-group-title", "Đồng bộ" }
            Toggle {
                label: "Đồng bộ cài đặt với các thiết bị khác",
                desc: "Giao diện, tùy chỉnh, thông báo, gợi ý, sticker / GIF / emoji yêu thích đi theo tài khoản. Mã PIN khóa ứng dụng luôn chỉ ở thiết bị này.",
                checked: p.sync_settings,
                on_change: move |v: bool| ctx.update(move |x| x.sync_settings = v),
            }
            if p.sync_settings && version > 0 {
                div { class: "settings-row-desc", style: "padding: 0 var(--space-3);", "Phiên bản đồng bộ: {version}" }
            }
            div { class: "settings-row",
                span { class: "grow",
                    span { class: "settings-label", "Hiệu ứng avatar trong danh sách" }
                    span { class: "settings-row-desc", "Danh sách trò chuyện và bạn bè có thể có nhiều avatar động. \"Khi rê chuột\" tiết kiệm pin và CPU nhất mà vẫn xem được." }
                }
                Segmented {
                    label: "Hiệu ứng avatar trong danh sách",
                    options: vec![("always".into(), "Luôn chạy".into()), ("hover".into(), "Khi rê chuột".into()), ("off".into(), "Tắt".into())],
                    value: p.avatar_fx_lists.clone(),
                    on_change: move |v: String| ctx.update(move |x| x.avatar_fx_lists = v.clone()),
                }
            }
        }
    }
}
