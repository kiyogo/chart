use dioxus::prelude::*;
use futures_util::StreamExt;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    components::{
        avatar::Avatar,
        calls::{ActiveCallOverlay, IncomingCallModal},
        details_panel::DmDetailsPanel,
        enterprise::{EnterpriseWorkspace, tasks_projects::CreateTaskFromMsgDialog},
        feed_view::FeedView,
        groups::{AddMemberModal, CreateGroupModal, GroupInfoPanel},
        icons::{
            BackIcon, BlockIcon, DevicesIcon, GroupIcon, HashIcon, MessageIcon, MicIcon, MicOffIcon, MoreIcon,
            PanelRightIcon, PhoneIcon, PhoneOffIcon, ShieldIcon, UserPlusIcon, VideoIcon, VolumeIcon,
        },
        messages::{Composer, MentionCandidate, MessageList, ViewMessage, preview_text},
        search_page::SearchPage,
        plugins_view::BotChatView,
        chat_extras::{ChatAction, ChatMoreMenu, PinnedBar, RenameDeviceDialog, WallpaperPicker, wallpaper_attrs},
        group_manage::GroupManageDialog,
        invite::{InviteDialog, InviteFriendsDialog},
        navigation::conversations::{ConvOpen, ConversationList, PoppedChannel, split_device_prefix},
        rich::{Rich, parse_rich, reminder_due},
        navigation::{
            modals::{CommunityModal, CreateCategoryModal, CreateChannelModal},
            rail::AppRail,
            sidebar::NavigationSidebar,
            NavSection,
        },
        profile_modal::{ProfileModal, ProfilePopover},
        guide::WelcomeTour,
        quick_switcher::QuickSwitcher,
        server_members_panel::{ServerHeaderTools, ServerMembersPanel},
        settings_view::SettingsView,
        vico_ai_view::VicoAiView,
    },
    prefs::{Prefs, PrefsCtx},
    crypto::e2ee::{
        PeerKeys, decrypt_chat_message, get_or_verify_peer_keys,
        get_user_keys,
    },
    media,
    models::{
        CallSessionItem, ChannelItem, ChannelMessageItem, DisplayMessage, FriendItem,
        GroupCategoryItem, GroupChannelItem, GroupItem, GroupMessageItem, GroupReactionItem,
        MessageStatus, PendingRequestItem, ServerCategoryItem, ServerItem, ServerMemberItem,
        UserSearchResult, VoiceStateItem, WsEvent, BotItem, BusinessData, ChatLabel, ConvPref,
        GroupMemberItem, GroupSettings, PrivacySettings, SessionItem,
    },
    platform,
    services::{api, calls, dm, enterprise, groups, library, social, ws},
    i18n::{t, tn},
};

/// Marks automatic replies so two auto-replying accounts never loop.
const AUTO_PREFIX: &str = "\u{2063}[Tự động] ";

fn in_hours(from: &str, to: &str) -> bool {
    let parse = |s: &str| -> Option<u32> {
        let (h, m) = s.split_once(':')?;
        Some(h.trim().parse::<u32>().ok()? * 60 + m.trim().parse::<u32>().ok()?)
    };
    let (Some(a), Some(b)) = (parse(from), parse(to)) else {
        return false;
    };
    use chrono::Timelike;
    let now = chrono::Local::now();
    let n = now.hour() * 60 + now.minute();
    if a <= b { n >= a && n < b } else { n >= a || n < b }
}

/// Reminder registry kept on this device so reminders fire even when their
/// conversation is not open: (id, local "YYYY-MM-DDTHH:MM", title, where).
fn register_reminders(me: &str, texts: &[(String, String)]) {
    let key = format!("chatw_reminders_{me}");
    let mut list: Vec<(String, String, String, String)> =
        platform::storage_get(&key).and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
    let mut changed = false;
    for (text, place) in texts {
        if let Some(Rich::Reminder(r)) = parse_rich(text) {
            if !list.iter().any(|x| x.0 == r.id) {
                list.push((r.id.clone(), r.at.clone(), r.t.clone(), place.clone()));
                changed = true;
            }
        }
    }
    if changed {
        if list.len() > 300 {
            list.drain(0..list.len() - 300);
        }
        platform::storage_set(&key, &serde_json::to_string(&list).unwrap_or_default());
    }
}

#[derive(Clone, PartialEq)]
struct AddFriendStatus {
    is_error: bool,
    message: String,
}

fn load_saved_previews(storage_key: &str) -> HashMap<String, String> {
    platform::storage_get(storage_key)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn persist_previews(storage_key: &str, map: &HashMap<String, String>) {
    if let Ok(json) = serde_json::to_string(map) {
        platform::storage_set(storage_key, &json);
    }
}

#[component]
pub fn Chat(
    current_user_id: String,
    current_username: String,
    current_user_avatar_url: Option<String>,
    on_logout: EventHandler<()>,
    on_avatar_updated: EventHandler<String>,
    #[props(default)] on_username_changed: Option<EventHandler<String>>,
) -> Element {
    let preview_storage_key = format!("chatw_previews_{current_user_id}");
    let uid_for_prefs = current_user_id.clone();
    let prefs_ctx = use_context_provider(move || PrefsCtx {
        prefs: Signal::new(Prefs::load(&uid_for_prefs)),
        user_id: Signal::new(uid_for_prefs.clone()),
    });
    let prefs = prefs_ctx.prefs;
    let mut quick_switch_open = use_signal(|| false);
    // Spec 013: welcome screen, my profile in a community, events, moderator tools.
    let mut welcome_for = use_signal(|| Option::<(String, crate::services::community::Welcome)>::None);
    let mut community_profile_open = use_signal(|| false);
    let mut events_open = use_signal(|| false);
    let mut can_manage_channels = use_signal(|| false);
    let tour_key = format!("chatw_tour_v1_{current_user_id}");
    let mut tour_open = use_signal({ let k = tour_key.clone(); move || platform::storage_get(&k).is_none() });
    let mut profile_popover = use_signal(|| Option::<(String, f64, f64)>::None);

    let mut user_keys = use_signal(|| get_user_keys(Some(&current_user_id)));
    let mut friends = use_signal(Vec::<FriendItem>::new);
    let mut active_friend = use_signal(|| Option::<FriendItem>::None);
    let mut messages = use_signal(Vec::<DisplayMessage>::new);
    let mut input_text = use_signal(String::new);
    let mut is_emoji_open = use_signal(|| false);
    let mut is_friend_typing = use_signal(|| false);
    let mut is_sending = use_signal(|| false);
    let mut is_settings_open = use_signal(|| false);

    // Avatar & status note state
    let mut my_avatar_url = use_signal(|| current_user_avatar_url.clone());
    let mut my_status_text = use_signal(|| Option::<String>::None);
    let mut my_status_emoji = use_signal(|| Option::<String>::None);
    let mut feed_refresh_tick = use_signal(|| 0u64);

    // Partner Info Sidebar (responsive auto-open on >= 1024px)
    let mut is_partner_info_open = use_signal(|| platform::initial_window_width() >= 1280);
    let mut partner_fingerprint = use_signal(|| Option::<String>::None);

    // Sidebar Tabs
    let mut nav_section = use_signal(|| NavSection::DirectMessages);
    let mut server_categories = use_signal(Vec::<ServerCategoryItem>::new);
    let mut voice_participants = use_signal(Vec::<VoiceStateItem>::new);
    let mut is_community_modal_open = use_signal(|| false);
    let mut is_create_category_open = use_signal(|| false);
    let mut create_channel_for = use_signal(|| Option::<Option<String>>::None);
    let mut is_server_invite_open = use_signal(|| false);
    let mut group_reply_to = use_signal(|| Option::<(String, String)>::None);
    let mut highlight_msg = use_signal(|| Option::<String>::None);
    let mut servers = use_signal(Vec::<ServerItem>::new);
    let mut selected_server = use_signal(|| Option::<ServerItem>::None);
    let mut server_members = use_signal(Vec::<ServerMemberItem>::new);
    let mut server_roles = use_signal(Vec::<social::ServerRole>::new);
    let mut is_server_members_open = use_signal(|| false);
    let mut enterprise_config = use_signal(|| Option::<enterprise::EnterpriseConfig>::None);
    let mut active_ent_module = use_signal(|| Option::<String>::None);
    let mut ent_refresh_tick = use_signal(|| 0u64);
    let mut create_task_from = use_signal(|| Option::<(String, String, String)>::None);
    let mut channels = use_signal(Vec::<ChannelItem>::new);
    let mut selected_channel = use_signal(|| Option::<ChannelItem>::None);
    let mut channel_messages = use_signal(Vec::<ChannelMessageItem>::new);
    let mut channel_input = use_signal(String::new);
    let mut active_voice_channel = use_signal(|| Option::<ChannelItem>::None);
    let mut is_voice_muted = use_signal(|| false);

    // Groups State
    let mut groups = use_signal(Vec::<GroupItem>::new);
    let mut active_group = use_signal(|| Option::<GroupItem>::None);
    let mut active_group_channel = use_signal(|| Option::<GroupChannelItem>::None);
    let mut group_channels = use_signal(Vec::<GroupChannelItem>::new);
    let mut group_categories = use_signal(Vec::<GroupCategoryItem>::new);
    let mut group_messages = use_signal(Vec::<GroupMessageItem>::new);
    let mut group_input = use_signal(String::new);
    let mut is_create_group_open = use_signal(|| false);
    let mut is_group_info_open = use_signal(|| false);
    let mut is_group_invite_open = use_signal(|| false);
    let mut is_group_add_member_open = use_signal(|| false);

    // Profile State
    let mut viewing_profile_user_id = use_signal(|| Option::<String>::None);

    // Calls State
    let mut active_call_session = use_signal(|| Option::<CallSessionItem>::None);
    let mut incoming_call_session = use_signal(|| Option::<CallSessionItem>::None);

    // Friends & Search
    let mut friend_input = use_signal(String::new);
    let mut search_results = use_signal(Vec::<UserSearchResult>::new);
    let mut add_friend_status = use_signal(|| Option::<AddFriendStatus>::None);
    let mut is_adding_friend = use_signal(|| false);
    let mut pending_requests = use_signal(Vec::<PendingRequestItem>::new);

    // Privacy, organisation, self chat ("Chính mình"), bots, business
    let my_id_sig = use_signal(|| current_user_id.clone());
    let mut privacy = use_signal(PrivacySettings::default);
    let mut blocked_ids = use_signal(Vec::<String>::new);
    let mut chat_labels = use_signal(Vec::<ChatLabel>::new);
    let mut conv_prefs = use_signal(Vec::<ConvPref>::new);
    let mut bot_chats = use_signal(Vec::<BotItem>::new);
    let mut popouts = use_signal(Vec::<PoppedChannel>::new);
    let mut popout_cache = use_signal(HashMap::<String, PoppedChannel>::new);
    let mut business = use_signal(BusinessData::default);
    let mut my_sessions = use_signal(Vec::<SessionItem>::new);
    let mut active_bot = use_signal(|| Option::<BotItem>::None);
    let mut bot_event = use_signal(|| Option::<serde_json::Value>::None);
    let mut partner_read_seq = use_signal(|| Option::<i64>::None);
    let mut dm_reply_to = use_signal(|| Option::<(String, String)>::None);
    let mut chat_wallpaper = use_signal(|| Option::<String>::None);
    let mut wallpaper_picker_open = use_signal(|| false);
    let mut chat_menu_open = use_signal(|| false);
    let mut rename_device = use_signal(|| Option::<(String, String)>::None);
    let mut group_members = use_signal(Vec::<GroupMemberItem>::new);
    let mut group_settings = use_signal(|| Option::<GroupSettings>::None);
    let mut invite_open = use_signal(|| Option::<(String, String)>::None);
    let mut toast = use_signal(|| Option::<String>::None);

    // App lock (PIN): locked at start when the app was left idle too long or
    // when "ask on every start" is on; the idle watcher re-locks later.
    let mut locked = use_signal(|| {
        let p = prefs.peek();
        if !p.lock_enabled || p.pin_hash.is_empty() {
            return false;
        }
        let last = platform::storage_get("chatw_last_active").and_then(|v| v.parse::<f64>().ok());
        let now = chrono::Utc::now().timestamp_millis() as f64;
        p.lock_on_start || last.is_some_and(|t| now - t >= p.lock_minutes as f64 * 60_000.0)
    });
    use_effect(move || {
        let p = prefs.read().clone();
        platform::apply_customization(&p);
        crate::i18n::set_language(&p.language);
    });
    // Only the lock settings matter here: other preference changes must not
    // restart the idle watcher.
    let lock_cfg = use_memo(move || {
        let p = prefs.read();
        (p.lock_enabled, !p.pin_hash.is_empty(), p.lock_minutes)
    });
    use_effect(move || {
        let (on, has_pin, minutes) = lock_cfg();
        if on && has_pin {
            spawn(async move {
                let mut ev = platform::idle_watch(minutes);
                while let Ok(msg) = ev.recv::<String>().await {
                    let p = prefs.peek();
                    if msg == "lock" && p.lock_enabled && !p.pin_hash.is_empty() {
                        locked.set(true);
                    }
                }
            });
        }
    });
    let mut group_manage_open = use_signal(|| false);
    let mut server_settings_open = use_signal(|| false);
    let mut invite_friends_for = use_signal(|| Option::<(String, String)>::None);
    // Toasts dismiss themselves.
    use_effect(move || {
        if let Some(msg) = toast() {
            spawn(async move {
                platform::sleep_ms(4500).await;
                if toast.peek().as_deref() == Some(msg.as_str()) {
                    toast.set(None);
                }
            });
        }
    });

    // Previews & Unread counts
    let saved_previews_init = load_saved_previews(&preview_storage_key);
    let mut last_previews = use_signal(move || saved_previews_init);
    let mut unread_counts = use_signal(HashMap::<String, usize>::new);

    // In-memory peer keys cache & typing state
    let mut peer_keys_cache = use_signal(HashMap::<String, PeerKeys>::new);
    let mut is_currently_typing = use_signal(|| false);
    let mut typing_seq = use_signal(|| 0u64);

    // Keep my_avatar_url synced if prop changes
    {
        let mut last_prop_avatar = use_signal(|| current_user_avatar_url.clone());
        if *last_prop_avatar.read() != current_user_avatar_url {
            last_prop_avatar.set(current_user_avatar_url.clone());
            my_avatar_url.set(current_user_avatar_url.clone());
        }
    }

    // Update document.title whenever unread counts or pending requests change
    use_effect(move || {
        let total_unread: usize = unread_counts.read().values().copied().sum();
        let reqs = pending_requests.read().len();
        let combined = total_unread + reqs;
        if combined > 0 {
            platform::set_document_title("chatw");
        } else {
            platform::set_document_title("chatw");
        }
    });

    // Live user search when friend_input changes
    use_effect(move || {
        let q = friend_input().trim().to_string();
        if q.is_empty() {
            search_results.set(Vec::new());
            return;
        }
        spawn(async move {
            match api::users_search(&q).await {
                Ok(res) => search_results.set(res),
                Err(_) => search_results.set(Vec::new()),
            }
        });
    });

    let reload_social = use_callback(move |_: ()| {
        let me_uid = my_id_sig.peek().clone();
        spawn(async move {
            if let Ok(me_prof) = api::get_user_profile(&me_uid).await {
                my_status_text.set(me_prof.status_text.filter(|s| !s.trim().is_empty()));
                my_status_emoji.set(me_prof.status_emoji.filter(|s| !s.trim().is_empty()));
            }
            if let Ok(p) = social::privacy_get().await {
                privacy.set(p);
            }
            if let Ok(b) = social::blocks_list().await {
                blocked_ids.set(b.into_iter().map(|u| u.id).collect());
            }
            if let Ok(l) = social::labels_list().await {
                chat_labels.set(l);
            }
            if let Ok(p) = social::prefs_list().await {
                conv_prefs.set(p);
            }
            if let Ok(b) = social::bots_chats().await {
                bot_chats.set(b);
            }
            if let Ok(b) = social::business_get().await {
                business.set(b);
            }
            if let Ok(s) = library::sessions_list().await {
                my_sessions.set(s);
            }
            let _ = crate::services::profile_ext::load_contacts().await;
            crate::services::settings_sync::pull(prefs_ctx).await;
        });
    });

    // Channels popped out of communities into Messages (resolved lazily, cached).
    use_effect(move || {
        let wanted: Vec<String> = conv_prefs
            .read()
            .iter()
            .filter(|p| p.conv_type == "channel" && p.in_messages)
            .map(|p| p.conv_id.clone())
            .collect();
        let srvs = servers();
        spawn(async move {
            let missing: Vec<String> = wanted.iter().filter(|id| !popout_cache.peek().contains_key(*id)).cloned().collect();
            if !missing.is_empty() {
                for srv in srvs.iter() {
                    if let Ok(chs) = api::servers_get_channels(&srv.id).await {
                        for ch in chs {
                            if missing.contains(&ch.id) {
                                popout_cache.write().insert(ch.id.clone(), PoppedChannel { channel: ch, server_name: srv.name.clone() });
                            }
                        }
                    }
                    if missing.iter().all(|m| popout_cache.peek().contains_key(m)) {
                        break;
                    }
                }
            }
            let list: Vec<PoppedChannel> = wanted.iter().filter_map(|id| popout_cache.peek().get(id).cloned()).collect();
            popouts.set(list);
        });
    });

    // Sound + system notification for an incoming message, honouring mute and settings.
    let notify_incoming = move |conv_type: &'static str, conv_id: String, title: String, body: String, active_here: bool| {
        let muted = conv_prefs.peek().iter().any(|p| p.conv_type == conv_type && p.conv_id == conv_id && p.muted);
        let pr = prefs.peek().clone();
        if muted || (conv_type == "group" && !pr.notify_groups) {
            return;
        }
        spawn(async move {
            let visible = platform::is_window_active().await;
            if visible && active_here {
                return;
            }
            if pr.sound {
                platform::play_sound(&pr.sound_name);
            }
            if pr.notify && !visible {
                let shown = if pr.notify_preview { body } else { "Tin nhắn mới".to_string() };
                platform::show_notification(&title, &shown, &format!("{conv_type}:{conv_id}"));
            }
        });
    };

    // Automatic replies and the Kinh doanh greeting, sent from this device (E2EE).
    let maybe_auto_reply = move |partner_id: String, incoming: String| {
        let me = my_id_sig.peek().clone();
        if partner_id == me || blocked_ids.peek().contains(&partner_id) || incoming.starts_with('\u{2063}') {
            return;
        }
        let pr = prefs.peek().clone();
        let biz = business.peek().clone();
        spawn(async move {
            if let Some(g) = biz
                .profile
                .as_ref()
                .filter(|p| p.enabled)
                .and_then(|p| p.greeting.clone())
                .filter(|g| !g.trim().is_empty())
            {
                let key = format!("chatw_greeted_{me}");
                let mut set: Vec<String> = platform::storage_get(&key).and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
                if !set.contains(&partner_id) {
                    set.push(partner_id.clone());
                    if set.len() > 5000 {
                        set.remove(0);
                    }
                    platform::storage_set(&key, &serde_json::to_string(&set).unwrap_or_default());
                    let _ = dm::send_encrypted_dm(&me, &partner_id, &format!("{AUTO_PREFIX}{}", g.trim()), None).await;
                    return;
                }
            }
            if !pr.auto_reply || pr.auto_reply_text.trim().is_empty() {
                return;
            }
            let should = match pr.auto_reply_mode.as_str() {
                "always" => true,
                "hours" => in_hours(&pr.auto_reply_from, &pr.auto_reply_to),
                _ => !platform::is_window_active().await,
            };
            if !should {
                return;
            }
            let key = format!("chatw_autoreply_{me}");
            let mut map: HashMap<String, i64> = platform::storage_get(&key).and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
            let now = chrono::Utc::now().timestamp();
            if map.get(&partner_id).is_some_and(|t| now - t < 6 * 3600) {
                return;
            }
            map.insert(partner_id.clone(), now);
            map.retain(|_, ts| now - *ts < 86400);
            platform::storage_set(&key, &serde_json::to_string(&map).unwrap_or_default());
            let _ = dm::send_encrypted_dm(&me, &partner_id, &format!("{AUTO_PREFIX}{}", pr.auto_reply_text.trim()), None).await;
        });
    };

    // Reminders: check every 20 seconds and notify once when due.
    use_future(move || async move {
        loop {
            platform::sleep_ms(20_000).await;
            let me = my_id_sig.peek().clone();
            let key = format!("chatw_reminders_{me}");
            let fired_key = format!("chatw_reminders_fired_{me}");
            let list: Vec<(String, String, String, String)> =
                platform::storage_get(&key).and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
            let mut fired: Vec<String> = platform::storage_get(&fired_key).and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
            let now = chrono::Local::now();
            let mut changed = false;
            for (id, at, title, place) in list {
                if fired.contains(&id) {
                    continue;
                }
                let r = crate::components::rich::ReminderData { id: id.clone(), t: title.clone(), at };
                if let Some(due) = reminder_due(&r) {
                    if due <= now && now - due < chrono::Duration::hours(12) {
                        let pr = prefs.peek().clone();
                        if pr.sound {
                            platform::play_sound("chime");
                        }
                        platform::show_notification("Nhắc hẹn", &format!("{title} · {place}"), &format!("reminder:{id}"));
                        toast.set(Some(format!("Nhắc hẹn: {title} · {place}")));
                        fired.push(id);
                        changed = true;
                    }
                }
            }
            if changed {
                if fired.len() > 500 {
                    fired.drain(0..fired.len() - 500);
                }
                platform::storage_set(&fired_key, &serde_json::to_string(&fired).unwrap_or_default());
            }
        }
    });

    // Initial mount: load keys, friends, requests, servers, install resize/audio listeners, connect WS
    let uid_for_mount = current_user_id.clone();
    let pkey_for_mount = preview_storage_key.clone();
    use_effect(move || {
        platform::install_audio_unlock();

        // Check actual viewport width & subscribe to resize events
        spawn(async move {
            let w = platform::get_viewport_width().await;
            is_partner_info_open.set(w >= 1280);
            let mut eval = dioxus::document::eval(
                r#"
                let wide = window.innerWidth >= 1280;
                window.addEventListener('resize', () => {
                    const now = window.innerWidth >= 1280;
                    if (now !== wide) { wide = now; dioxus.send(now); }
                });
                "#,
            );
            while let Ok(now_wide) = eval.recv::<bool>().await {
                if !now_wide {
                    is_partner_info_open.set(false);
                    is_group_info_open.set(false);
                }
            }
        });

        let uid = uid_for_mount.clone();
        let pkey = pkey_for_mount.clone();
        spawn(async move {
            user_keys.set(get_user_keys(Some(&uid)));
            if let Ok(list) = api::friends_list().await {
                friends.set(list);
            }
            if let Ok(reqs) = api::friends_get_requests().await {
                pending_requests.set(reqs);
            }
            if let Ok(srvs) = api::servers_list().await {
                servers.set(srvs);
            }
            if let Ok(grps) = groups::groups_list().await {
                groups.set(grps);
            }
            reload_social.call(());

            let (_ws_handle, mut rx) = ws::connect_ws();
            while let Some(event) = rx.next().await {
                match event {
                    WsEvent::MessageNew {
                        id,
                        seq,
                        sender_id,
                        recipient_id,
                        envelope,
                        reply_to_id,
                        created_at,
                    } => {
                        let Some(keys) = user_keys() else {
                            continue;
                        };
                        let is_from_me = sender_id == uid;
                        let partner_id = if is_from_me {
                            recipient_id.clone()
                        } else {
                            sender_id.clone()
                        };

                        let is_self_msg = partner_id == uid;
                        if !is_self_msg
                            && !friends.read().iter().any(|f| f.id == partner_id)
                            && let Ok(list) = api::friends_list().await {
                                friends.set(list);
                            }

                        // Bump partner to top of conversation list
                        friends.with_mut(|list| {
                            if let Some(idx) = list.iter().position(|f| f.id == partner_id)
                                && idx > 0 {
                                    let f = list.remove(idx);
                                    list.insert(0, f);
                                }
                        });

                        let is_for_active = active_friend()
                            .as_ref()
                            .map(|af| af.id == partner_id)
                            .unwrap_or(false);

                        let sender_ecdsa_raw = if is_from_me {
                            Some(keys.ecdsa_raw)
                        } else {
                            let cached = peer_keys_cache.read().get(&sender_id).cloned();
                            match cached {
                                Some(pk) => Some(pk.ecdsa_raw),
                                None => match api::keys_get(&sender_id).await {
                                    Ok(pk_resp) => match get_or_verify_peer_keys(
                                        &pk_resp.user_id,
                                        &pk_resp.ecdh_public,
                                        &pk_resp.ecdsa_public,
                                        &pk_resp.fingerprint,
                                    ) {
                                        Ok(peer_k) => {
                                            let raw = peer_k.ecdsa_raw;
                                            peer_keys_cache
                                                .write()
                                                .insert(sender_id.clone(), peer_k);
                                            Some(raw)
                                        }
                                        Err(_) => None,
                                    },
                                    Err(_) => None,
                                },
                            }
                        };

                        let dec_res = sender_ecdsa_raw
                            .ok_or_else(|| "missing_key".to_string())
                            .and_then(|ecdsa| decrypt_chat_message(&envelope, &keys, &ecdsa));

                        match dec_res {
                            Ok(decrypted) => {
                                last_previews.with_mut(|map| {
                                    map.insert(partner_id.clone(), decrypted.text.clone());
                                    persist_previews(&pkey, map);
                                });

                                let place = if is_self_msg {
                                    t("chat.self").to_string()
                                } else {
                                    friends.peek().iter().find(|f| f.id == partner_id).map(|f| f.username.clone()).unwrap_or_default()
                                };
                                register_reminders(&uid, &[(split_device_prefix(&decrypted.text).1, place.clone())]);
                                if !is_from_me {
                                    notify_incoming("dm", partner_id.clone(), place.clone(), crate::components::messages::preview_text(&decrypted.text), is_for_active);
                                    maybe_auto_reply(partner_id.clone(), decrypted.text.clone());
                                    if is_for_active && platform::is_window_active().await {
                                        let pid = partner_id.clone();
                                        spawn(async move {
                                            let _ = social::mark_read(&pid, seq).await;
                                        });
                                    }
                                }
                                if is_for_active {
                                    messages.with_mut(|list| {
                                        if !list.iter().any(|m| m.id == id) {
                                            list.push(DisplayMessage {
                                                id: id.clone(),
                                                seq,
                                                sender_id: sender_id.clone(),
                                                recipient_id: recipient_id.clone(),
                                                text: decrypted.text.clone(),
                                                created_at: created_at.clone(),
                                                is_self: is_from_me,
                                                status: MessageStatus::Sent,
                                                reply_to_id: reply_to_id.clone(),
                                                reactions: Vec::new(),
                                                is_pinned: false,
                                            });
                                        }
                                    });
                                    platform::scroll_chat_to_bottom(false);

                                    if !is_from_me
                                        && (nav_section() != NavSection::DirectMessages
                                            || is_settings_open())
                                    {
                                        unread_counts.with_mut(|m| {
                                            *m.entry(partner_id.clone()).or_insert(0) += 1;
                                        });
                                    }
                                } else if !is_from_me {
                                    unread_counts.with_mut(|m| {
                                        *m.entry(partner_id.clone()).or_insert(0) += 1;
                                    });
                                }
                            }
                            Err(_) => {
                                if is_for_active && !messages.read().iter().any(|m| m.id == id) {
                                    messages.with_mut(|list| {
                                        list.push(DisplayMessage {
                                            id: id.clone(),
                                            seq,
                                            sender_id: sender_id.clone(),
                                            recipient_id: recipient_id.clone(),
                                            text: "Không đọc được trên máy này".to_string(),
                                            created_at: created_at.clone(),
                                            is_self: false,
                                            status: MessageStatus::Error,
                                            reply_to_id: None,
                                            reactions: Vec::new(),
                                            is_pinned: false,
                                        });
                                    });
                                }
                            }
                        }
                    }
                    WsEvent::ChannelMessageNew(payload) => {
                        let msg = payload.into_item();
                        if selected_channel()
                            .as_ref()
                            .map(|c| c.id == msg.channel_id)
                            .unwrap_or(false)
                        {
                            channel_messages.with_mut(|list| {
                                if !list.iter().any(|m| m.id == msg.id) {
                                    list.push(msg);
                                }
                            });
                        }
                    }
                    WsEvent::MessageDeleted { message_id } => {
                        messages.with_mut(|list| {
                            for m in list.iter_mut() {
                                if m.id == message_id {
                                    m.text = "[Tin nhắn đã bị thu hồi]".to_string();
                                }
                            }
                        });
                    }
                    WsEvent::MessageEdited {
                        message_id,
                        envelope,
                    } => {
                        if let (Some(keys), Some(active)) = (user_keys(), active_friend()) {
                            let is_self_msg = messages
                                .read()
                                .iter()
                                .find(|m| m.id == message_id)
                                .map(|m| m.is_self)
                                .unwrap_or(false);
                            let sender_ecdsa = if is_self_msg {
                                Some(keys.ecdsa_raw)
                            } else {
                                peer_keys_cache
                                    .read()
                                    .get(&active.id)
                                    .map(|p| p.ecdsa_raw)
                            };
                            if let Some(ecdsa) = sender_ecdsa
                                && let Ok(dec) = decrypt_chat_message(&envelope, &keys, &ecdsa) {
                                    messages.with_mut(|list| {
                                        for m in list.iter_mut() {
                                            if m.id == message_id {
                                                m.text = dec.text.clone();
                                            }
                                        }
                                    });
                                }
                        }
                    }
                    WsEvent::FriendRequest { .. } => {
                        if let Ok(reqs) = api::friends_get_requests().await {
                            pending_requests.set(reqs);
                        }
                        let pr = prefs.peek().clone();
                        if pr.sound { platform::play_sound(&pr.sound_name); }
                        if pr.notify { platform::show_notification("ChatW", "Bạn có lời mời kết bạn mới", "friend-request"); }
                    }
                    WsEvent::FriendAccepted { .. } => {
                        if let Ok(list) = api::friends_list().await {
                            friends.set(list);
                        }
                        if let Ok(reqs) = api::friends_get_requests().await {
                            pending_requests.set(reqs);
                        }
                    }
                    WsEvent::TypingStart { user_id } => {
                        if privacy.peek().typing && active_friend()
                            .as_ref()
                            .map(|a| a.id == user_id)
                            .unwrap_or(false)
                        {
                            is_friend_typing.set(true);
                        }
                    }
                    WsEvent::TypingStop { user_id }
                        if active_friend()
                            .as_ref()
                            .map(|a| a.id == user_id)
                            .unwrap_or(false)
                        => {
                            is_friend_typing.set(false);
                        }
                    WsEvent::GroupMessageNew(payload) => {
                        let msg = payload.into_item();
                        let is_active_group = active_group().as_ref().map(|g| g.id == msg.group_id).unwrap_or(false);
                        let gname = groups.peek().iter().find(|g| g.id == msg.group_id).map(|g| g.name.clone()).unwrap_or_default();
                        last_previews.with_mut(|map| {
                            map.insert(msg.group_id.clone(), format!("{}: {}", msg.sender_username, crate::components::messages::preview_text(&msg.content)));
                        });
                        register_reminders(&uid, &[(msg.content.clone(), gname.clone())]);
                        if msg.sender_id != uid {
                            if !is_active_group {
                                unread_counts.with_mut(|m| *m.entry(msg.group_id.clone()).or_insert(0) += 1);
                            }
                            notify_incoming("group", msg.group_id.clone(), gname.clone(), format!("{}: {}", msg.sender_username, crate::components::messages::preview_text(&msg.content)), is_active_group);
                        }
                        if is_active_group
                        {
                            group_messages.with_mut(|list| {
                                if !list.iter().any(|m| m.id == msg.id) {
                                    list.push(msg.clone());
                                }
                            });
                            platform::scroll_chat_to_bottom(false);
                        }
                    }
                    WsEvent::GroupMemberJoined(_)
                    | WsEvent::GroupMemberLeft(_)
                    | WsEvent::GroupUpdated(_) => {
                        spawn(async move {
                            if let Ok(grps) = groups::groups_list().await {
                                groups.set(grps);
                            }
                        });
                    }
                    WsEvent::ReactionUpdate(val) => {
                        let msg_id = val.get("message_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let user_id = val.get("user_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let emoji = val.get("emoji").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let added = val.get("action").and_then(|v| v.as_str()) == Some("added");
                        messages.with_mut(|list| {
                            if let Some(m) = list.iter_mut().find(|m| m.id == msg_id) {
                                if let Some(entry) = m.reactions.iter_mut().find(|(e, _)| *e == emoji) {
                                    entry.1.retain(|u| u != &user_id);
                                    if added { entry.1.push(user_id.clone()); }
                                } else if added {
                                    m.reactions.push((emoji.clone(), vec![user_id.clone()]));
                                }
                                m.reactions.retain(|(_, users)| !users.is_empty());
                            }
                        });
                    }
                    WsEvent::GroupReactionUpdate(val) => {
                        let msg_id = val.get("message_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let user_id = val.get("user_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let emoji = val.get("emoji").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let action = val.get("action").and_then(|v| v.as_str()).unwrap_or_default().to_string();

                        group_messages.with_mut(|list| {
                            if let Some(msg) = list.iter_mut().find(|m| m.id == msg_id) {
                                if action == "removed" {
                                    msg.reactions.retain(|r| !(r.user_id == user_id && r.emoji == emoji));
                                } else if action == "added" {
                                    if !msg.reactions.iter().any(|r| r.user_id == user_id && r.emoji == emoji) {
                                        msg.reactions.push(GroupReactionItem {
                                            id: Uuid::new_v4().to_string(),
                                            message_id: msg_id.clone(),
                                            user_id: user_id.clone(),
                                            username: String::new(),
                                            emoji: emoji.clone(),
                                            created_at: String::new(),
                                        });
                                    }
                                }
                            }
                        });
                    }
                    WsEvent::GroupChannelNew(val) => {
                        let gid = val.get("group_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        if active_group().as_ref().map(|g| g.id == gid).unwrap_or(false) {
                            if let Some(ch_val) = val.get("channel") {
                                if let Ok(ch) = serde_json::from_value::<GroupChannelItem>(ch_val.clone()) {
                                    group_channels.with_mut(|list| {
                                        if !list.iter().any(|c| c.id == ch.id) {
                                            list.push(ch);
                                        }
                                    });
                                }
                            }
                        }
                    }
                    WsEvent::GroupChannelDeleted(val) => {
                        let cid = val.get("channel_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        group_channels.with_mut(|list| {
                            list.retain(|c| c.id != cid);
                        });
                    }
                    WsEvent::CallIncoming(payload) => {
                        let sess = payload.into_item();
                        incoming_call_session.set(Some(sess));
                        let pr = prefs.peek().clone();
                        if pr.sound { platform::play_sound("chime"); }
                        if pr.notify_calls { platform::show_notification("ChatW", "Cuộc gọi đến", "call"); }
                    }
                    WsEvent::CallAccepted(val) => {
                        if let Ok(sess) = serde_json::from_value::<CallSessionItem>(val) {
                            incoming_call_session.set(None);
                            active_call_session.set(Some(sess));
                        }
                    }
                    WsEvent::CallRejected(_) | WsEvent::CallEnded(_) => {
                        incoming_call_session.set(None);
                        active_call_session.set(None);
                        media::stop_all_media();
                    }
                    WsEvent::CallSignal(val) => {
                        let call_id = val.get("call_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let sender_id = val.get("sender_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let signal_type = val.get("signal_type").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let data = val.get("data").cloned().unwrap_or(serde_json::Value::Null);

                        let c_id = call_id.clone();
                        let s_id = sender_id.clone();
                        spawn(async move {
                            if let Ok(Some(reply_data)) = media::handle_remote_signal(&c_id, &s_id, &signal_type, data).await {
                                let reply_type = if signal_type == "offer" { "answer" } else { "ice_candidate" };
                                let _ = calls::calls_send_signal(&c_id, Some(s_id), reply_type, reply_data).await;
                            }
                        });
                    }
                    WsEvent::CallParticipantJoined(val) => {
                        if let (Some(sess_id), Some(p_val)) = (
                            val.get("session_id").and_then(|v| v.as_str()),
                            val.get("participant"),
                        ) {
                            if let Ok(p) = serde_json::from_value::<crate::models::CallParticipantItem>(p_val.clone()) {
                                active_call_session.with_mut(|opt| {
                                    if let Some(s) = opt {
                                        if s.id == sess_id && !s.participants.iter().any(|x| x.user_id == p.user_id) {
                                            s.participants.push(p);
                                        }
                                    }
                                });
                            }
                        }
                    }
                    WsEvent::CallParticipantLeft(val) => {
                        if let (Some(sess_id), Some(user_id)) = (
                            val.get("session_id").and_then(|v| v.as_str()),
                            val.get("user_id").and_then(|v| v.as_str()),
                        ) {
                            active_call_session.with_mut(|opt| {
                                if let Some(s) = opt {
                                    if s.id == sess_id {
                                        s.participants.retain(|x| x.user_id != user_id);
                                    }
                                }
                            });
                        }
                    }
                    WsEvent::CallStateUpdate(val) => {
                        if let (Some(sess_id), Some(user_id), Some(muted), Some(cam_off)) = (
                            val.get("session_id").and_then(|v| v.as_str()),
                            val.get("user_id").and_then(|v| v.as_str()),
                            val.get("is_muted").and_then(|v| v.as_bool()),
                            val.get("is_camera_off").and_then(|v| v.as_bool()),
                        ) {
                            active_call_session.with_mut(|opt| {
                                if let Some(s) = opt {
                                    if s.id == sess_id {
                                        for p in s.participants.iter_mut() {
                                            if p.user_id == user_id {
                                                p.muted = muted;
                                                p.camera_off = cam_off;
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                    WsEvent::MessagePinned { message_id, is_pinned } => {
                        messages.with_mut(|list| {
                            for m in list.iter_mut() {
                                if m.id == message_id {
                                    m.is_pinned = is_pinned;
                                }
                            }
                        });
                    }
                    WsEvent::MessageRead(v) => {
                        let reader = v.get("reader_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        if active_friend().as_ref().is_some_and(|f| f.id == reader) {
                            partner_read_seq.set(v.get("seq").and_then(|x| x.as_i64()));
                        }
                    }
                    WsEvent::WallpaperUpdated(v) => {
                        let ct = v.get("conv_type").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let cid = v.get("conv_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let set_by = v.get("set_by").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let value = v.get("value").and_then(|x| x.as_str()).map(str::to_string);
                        let hit = match ct.as_str() {
                            "dm" => {
                                let partner = if cid == uid { set_by } else { cid };
                                active_friend().as_ref().is_some_and(|f| f.id == partner)
                            }
                            "group" => active_group().as_ref().is_some_and(|g| g.id == cid),
                            _ => false,
                        };
                        if hit {
                            chat_wallpaper.set(value);
                        }
                    }
                    WsEvent::BotMessage(v) => {
                        let bid = v.get("bot_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let text = v.get("text").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let viewing = active_bot().as_ref().is_some_and(|b| b.id == bid);
                        if !viewing {
                            unread_counts.with_mut(|m| *m.entry(bid.clone()).or_insert(0) += 1);
                        }
                        let bname = bot_chats.peek().iter().find(|b| b.id == bid).map(|b| b.display_name.clone()).unwrap_or_else(|| "Bot".into());
                        notify_incoming("bot", bid.clone(), bname, text.clone(), viewing);
                        bot_chats.with_mut(|l| {
                            if let Some(b) = l.iter_mut().find(|b| b.id == bid) {
                                b.last_text = Some(text.clone());
                            }
                        });
                        if !bot_chats.peek().iter().any(|b| b.id == bid) {
                            if let Ok(list) = social::bots_chats().await {
                                bot_chats.set(list);
                            }
                        }
                        bot_event.set(Some(v));
                    }
                    WsEvent::GroupJoinRequest(v) => {
                        let gid = v.get("group_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let who = v.get("username").and_then(|x| x.as_str()).unwrap_or("Một người").to_string();
                        let gname = groups.peek().iter().find(|g| g.id == gid).map(|g| g.name.clone()).unwrap_or_default();
                        toast.set(Some(format!("{who} muốn tham gia nhóm {gname}")));
                        if prefs.peek().notify {
                            platform::show_notification("Yêu cầu tham gia nhóm", &format!("{who} muốn tham gia {gname}"), &format!("join:{gid}"));
                        }
                        if active_group().as_ref().is_some_and(|g| g.id == gid) {
                            if let Ok(st) = social::group_settings(&gid).await {
                                group_settings.set(Some(st));
                            }
                        }
                    }
                    WsEvent::GroupSettings(v) => {
                        let gid = v.get("group_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        if active_group().as_ref().is_some_and(|g| g.id == gid) {
                            if let Ok(st) = social::group_settings(&gid).await {
                                group_settings.set(Some(st));
                            }
                            if let Ok(m) = groups::groups_get_members(&gid).await {
                                group_members.set(m);
                            }
                        }
                        if let Ok(grps) = groups::groups_list().await {
                            groups.set(grps);
                        }
                    }
                    WsEvent::PrefsUpdated(v) if v.get("kind").and_then(|k| k.as_str()) == Some("settings") => {
                        let newer = v.get("version").and_then(|x| x.as_i64()).unwrap_or(0) > crate::services::settings_sync::known_version();
                        if newer {
                            crate::services::settings_sync::pull(prefs_ctx).await;
                        }
                    }
                    WsEvent::PrefsUpdated(v) if v.get("kind").and_then(|k| k.as_str()) == Some("contacts") => {
                        let _ = crate::services::profile_ext::load_contacts().await;
                    }
                    WsEvent::SecurityAlert(v) => {
                        let device = v.get("device").and_then(|x| x.as_str()).unwrap_or("thiết bị mới").to_string();
                        let ip = v.get("ip").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let text = format!("Tài khoản vừa đăng nhập từ {device} · {ip}. Không phải bạn? Vào Cài đặt › Thiết bị để đăng xuất thiết bị đó.");
                        toast.set(Some(text.clone()));
                        platform::show_notification("ChatW: đăng nhập mới", &text, "security-alert");
                    }
                    WsEvent::ProfileUpdated(_) => {
                        if let Ok(list) = api::friends_list().await {
                            friends.set(list);
                        }
                        if let Some(srv) = selected_server.peek().clone() {
                            if let Ok(ms) = api::servers_get_members(&srv.id).await {
                                server_members.set(ms);
                            }
                        }
                    }
                    WsEvent::UserPresenceUpdate(v) => {
                        let uid = v.get("user_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let pres = v.get("presence").and_then(|x| x.as_str()).map(str::to_string);
                        let st = v.get("status_text").and_then(|x| x.as_str()).map(str::to_string).filter(|s| !s.trim().is_empty());
                        let se = v.get("status_emoji").and_then(|x| x.as_str()).map(str::to_string).filter(|s| !s.trim().is_empty());
                        if uid == *my_id_sig.peek() {
                            my_status_text.set(st.clone());
                            my_status_emoji.set(se.clone());
                        }
                        if !uid.is_empty() {
                            friends.with_mut(|list| {
                                if let Some(fr) = list.iter_mut().find(|f| f.id == uid) {
                                    if let Some(ref p) = pres {
                                        fr.presence = Some(p.clone());
                                    }
                                    fr.status_text = st.clone();
                                    fr.status_emoji = se.clone();
                                }
                            });
                            active_friend.with_mut(|af| {
                                if let Some(fr) = af.as_mut() {
                                    if fr.id == uid {
                                        if let Some(ref p) = pres {
                                            fr.presence = Some(p.clone());
                                        }
                                        fr.status_text = st.clone();
                                        fr.status_emoji = se.clone();
                                    }
                                }
                            });
                            server_members.with_mut(|list| {
                                if let Some(m) = list.iter_mut().find(|x| x.user_id == uid) {
                                    if let Some(ref p) = pres {
                                        m.presence = Some(p.clone());
                                    }
                                    m.status_text = st.clone();
                                    m.status_emoji = se.clone();
                                }
                            });
                        }
                    }
                    WsEvent::FeedUpdated(_) => {
                        feed_refresh_tick.set(feed_refresh_tick() + 1);
                    }
                    WsEvent::PrefsUpdated(_) => {
                        if let Ok(p) = social::prefs_list().await {
                            conv_prefs.set(p);
                        }
                        if let Ok(l) = social::labels_list().await {
                            chat_labels.set(l);
                        }
                    }
                    WsEvent::EnterpriseEvent(v) => {
                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                        let srv_id = v.get("server_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        if selected_server().as_ref().is_some_and(|s| s.id == srv_id) {
                            let sid = srv_id.clone();
                            spawn(async move {
                                if let Ok(list) = api::servers_list().await {
                                    if let Some(updated) = list.iter().find(|x| x.id == sid).cloned() {
                                        selected_server.set(Some(updated));
                                    }
                                    servers.set(list);
                                }
                                if let Ok(cfg) = enterprise::get_config(&sid).await {
                                    enterprise_config.set(Some(cfg));
                                } else {
                                    enterprise_config.set(None);
                                    active_ent_module.set(None);
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        });
    });

    // Global keyboard shortcut: Ctrl/Cmd + K opens the quick switcher.
    use_effect(move || {
        spawn(async move {
            let mut eval = dioxus::document::eval(
                r#"
                if (!window.__chatwQuickKey) {
                    window.__chatwQuickKey = true;
                    document.addEventListener('keydown', (e) => {
                        if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
                            e.preventDefault();
                            dioxus.send(true);
                        }
                    });
                }
                "#,
            );
            while eval.recv::<bool>().await.is_ok() {
                quick_switch_open.set(true);
            }
        });
    });

    // Helper: select a channel
    let mut select_channel = move |ch: ChannelItem| {
        server_settings_open.set(false);
        active_ent_module.set(None);
        selected_channel.set(Some(ch.clone()));
        active_friend.set(None);
        active_group.set(None);
        active_bot.set(None);
        unread_counts.with_mut(|m| {
            m.remove(&ch.id);
        });
        if ch.channel_type == "text" || ch.channel_type == "announcement" {
            let cid = ch.id.clone();
            spawn(async move {
                if let Ok(msgs) = api::channels_get_messages(&cid, 50).await {
                    channel_messages.set(msgs);
                }
            });
        } else if ch.channel_type == "voice" {
            let ch_clone = ch.clone();
            spawn(async move {
                if let Ok(parts) = api::webrtc_join_voice(&ch_clone.id).await {
                    voice_participants.set(parts);
                    active_voice_channel.set(Some(ch_clone));
                }
            });
        }
    };

    // Helper: select a group
    let select_group = use_callback(move |grp: GroupItem| {
        is_settings_open.set(false);
        server_settings_open.set(false);
        nav_section.set(NavSection::DirectMessages);
        group_reply_to.set(None);
        selected_channel.set(None);
        active_friend.set(None);
        active_bot.set(None);
        active_group.set(Some(grp.clone()));
        group_messages.set(Vec::new());
        group_settings.set(None);
        group_members.set(Vec::new());
        chat_wallpaper.set(None);
        chat_menu_open.set(false);
        unread_counts.with_mut(|m| {
            m.remove(&grp.id);
        });

        let gid = grp.id.clone();
        let gid2 = grp.id.clone();
        spawn(async move {
            if let Ok(st) = social::group_settings(&gid2).await {
                group_settings.set(Some(st));
            }
            if let Ok(m) = groups::groups_get_members(&gid2).await {
                group_members.set(m);
            }
            if let Ok(w) = social::wallpaper_get("group", &gid2).await {
                chat_wallpaper.set(w);
            }
        });
        spawn(async move {
            let _ = groups::groups_list_categories(&gid).await.map(|cats| group_categories.set(cats));
            if let Ok(chs) = groups::groups_list_channels(&gid).await {
                let first = chs.first().cloned();
                group_channels.set(chs);
                active_group_channel.set(first.clone());
                let cid = first.as_ref().map(|c| c.id.as_str());
                if let Ok(msgs) = groups::groups_get_messages(&gid, cid, 50, None).await {
                    group_messages.set(msgs);
                }
            } else if let Ok(msgs) = groups::groups_get_messages(&gid, None, 50, None).await {
                group_messages.set(msgs);
            }
            let gname = groups.peek().iter().find(|g| g.id == gid).map(|g| g.name.clone()).unwrap_or_default();
            let texts: Vec<(String, String)> = group_messages.peek().iter().map(|m| (m.content.clone(), gname.clone())).collect();
            register_reminders(&my_id_sig.peek(), &texts);
            platform::scroll_chat_to_bottom(false);
        });
    });

    // Helper: select a server
    let mut select_server = move |srv: ServerItem| {
        is_settings_open.set(false);
        server_settings_open.set(false);
        nav_section.set(NavSection::Server(srv.id.clone()));
        selected_server.set(Some(srv.clone()));
        active_ent_module.set(None);
        enterprise_config.set(None);
        active_friend.set(None);
        active_group.set(None);
        active_bot.set(None);
        selected_channel.set(None);
        channel_messages.set(Vec::new());
        server_categories.set(Vec::new());
        channels.set(Vec::new());
        server_members.set(Vec::new());
        server_roles.set(Vec::new());
        spawn(async move {
            let vw = platform::get_viewport_width().await;
            is_server_members_open.set(vw >= 1100);
            if srv.is_enterprise {
                if let Ok(ecfg) = enterprise::get_config(&srv.id).await {
                    enterprise_config.set(Some(ecfg));
                }
            }
            if let Ok(cats) = api::servers_get_categories(&srv.id).await {
                server_categories.set(cats);
            }
            if let Ok(ms) = api::servers_get_members(&srv.id).await {
                server_members.set(ms);
            }
            if let Ok(rs) = social::server_roles(&srv.id).await {
                server_roles.set(rs);
            }
            // Community emoji / stickers for the pickers, my permissions, and the welcome screen.
            crate::services::community::activate(&srv.id).await;
            if let Ok((bits, _)) = social::server_my_permissions(&srv.id).await {
                can_manage_channels.set(bits & 4 != 0);
            }
            if let Some(w) = crate::components::community_ui::should_show_welcome(&srv.id).await {
                welcome_for.set(Some((srv.id.clone(), w)));
            }
            if let Ok(chs) = api::servers_get_channels(&srv.id).await {
                let first_text = chs.iter().find(|c| c.channel_type == "text").cloned();
                channels.set(chs);
                // On wide screens open the first text channel; on mobile keep the channel list visible.
                if vw >= 768
                    && let Some(first) = first_text {
                        select_channel(first);
                    }
            }
        });
    };

    // Helper: select a friend DM
    let uid_for_select = current_user_id.clone();
    let pkey_for_select = preview_storage_key.clone();
    let select_friend = use_callback(move |friend: FriendItem| {
        is_settings_open.set(false);
        server_settings_open.set(false);
        nav_section.set(NavSection::DirectMessages);
        selected_channel.set(None);
        active_group.set(None);
        active_bot.set(None);
        active_friend.set(Some(friend.clone()));
        is_friend_typing.set(false);
        messages.set(Vec::new());
        partner_fingerprint.set(None);
        dm_reply_to.set(None);
        partner_read_seq.set(None);
        chat_wallpaper.set(None);
        chat_menu_open.set(false);

        unread_counts.with_mut(|m| {
            m.remove(&friend.id);
        });

        let uid = uid_for_select.clone();
        let pkey = pkey_for_select.clone();
        spawn(async move {
            let Some(keys) = user_keys() else {
                return;
            };

            let cached_pk = peer_keys_cache.read().get(&friend.id).cloned();
            let peer_keys = match cached_pk {
                Some(pk) => {
                    partner_fingerprint.set(Some(pk.fingerprint_b64.clone()));
                    pk
                }
                None => {
                    let Ok(pk_resp) = api::keys_get(&friend.id).await else {
                        return;
                    };
                    partner_fingerprint.set(Some(pk_resp.fingerprint.clone()));
                    let Ok(pk) = get_or_verify_peer_keys(
                        &pk_resp.user_id,
                        &pk_resp.ecdh_public,
                        &pk_resp.ecdsa_public,
                        &pk_resp.fingerprint,
                    ) else {
                        return;
                    };
                    peer_keys_cache.write().insert(friend.id.clone(), pk.clone());
                    pk
                }
            };

            if let Ok(mut wire_list) = api::messages_get(&friend.id, None, Some(50)).await {
                wire_list.sort_by_key(|m| m.seq);
                let mut decrypted_list = Vec::with_capacity(wire_list.len());
                for wire in wire_list {
                    let is_self = wire.sender_id == uid;
                    let sender_ecdsa = if is_self {
                        &keys.ecdsa_raw
                    } else {
                        &peer_keys.ecdsa_raw
                    };
                    match decrypt_chat_message(&wire.envelope, &keys, sender_ecdsa) {
                        Ok(dec) => decrypted_list.push(DisplayMessage {
                            id: wire.id,
                            seq: wire.seq,
                            sender_id: wire.sender_id,
                            recipient_id: wire.recipient_id,
                            text: dec.text,
                            created_at: wire.created_at,
                            is_self,
                            status: MessageStatus::Sent,
                            reply_to_id: wire.reply_to_id,
                            reactions: wire.reactions.iter().map(|r| (r.emoji.clone(), r.user_ids.clone())).collect(),
                            is_pinned: wire.is_pinned,
                        }),
                        Err(_) => decrypted_list.push(DisplayMessage {
                            id: wire.id,
                            seq: wire.seq,
                            sender_id: wire.sender_id,
                            recipient_id: wire.recipient_id,
                            text: "Không đọc được trên máy này".to_string(),
                            created_at: wire.created_at,
                            is_self,
                            status: MessageStatus::Error,
                            reply_to_id: None,
                            reactions: Vec::new(),
                            is_pinned: false,
                        }),
                    }
                }
                if let Some(last) = decrypted_list.last() {
                    let txt = last.text.clone();
                    last_previews.with_mut(|m| {
                        m.insert(friend.id.clone(), txt);
                        persist_previews(&pkey, m);
                    });
                }
                let place = if friend.id == uid { t("chat.self").to_string() } else { friend.username.clone() };
                let texts: Vec<(String, String)> = decrypted_list.iter().map(|m| (split_device_prefix(&m.text).1, place.clone())).collect();
                register_reminders(&uid, &texts);
                let last_partner_seq = decrypted_list.iter().filter(|m| !m.is_self).map(|m| m.seq).max();
                messages.set(decrypted_list);
                platform::scroll_chat_to_bottom(true);
                if friend.id == uid {
                    chat_wallpaper.set(platform::storage_get(&format!("chatw_selfwp_{uid}")));
                }
                if friend.id != uid {
                    if let Some(seq) = last_partner_seq {
                        let _ = social::mark_read(&friend.id, seq).await;
                    }
                    if let Ok(seq) = social::partner_read(&friend.id).await {
                        partner_read_seq.set(seq);
                    }
                    if let Ok(w) = social::wallpaper_get("dm", &friend.id).await {
                        chat_wallpaper.set(w);
                    }
                }
            }
        });
    });

    let me_for_self = current_user_id.clone();
    let open_self_chat = use_callback(move |_: ()| {
        select_friend.call(FriendItem {
            id: me_for_self.clone(),
            username: t("chat.self").to_string(),
            display_name: None,
            nameplate: None,
            custom_nameplate_url: None,
            name_effect: None,
            profile_color: None,
            status_text: None,
            status_emoji: None,
            presence: None,
            avatar_url: None,
            avatar_decoration: None,
            avatar_motion: None,
        });
    });

    let open_bot = use_callback(move |b: BotItem| {
        is_settings_open.set(false);
        server_settings_open.set(false);
        nav_section.set(NavSection::DirectMessages);
        active_friend.set(None);
        active_group.set(None);
        selected_channel.set(None);
        unread_counts.with_mut(|m| {
            m.remove(&b.id);
        });
        if !bot_chats.peek().iter().any(|x| x.id == b.id) {
            bot_chats.with_mut(|l| l.insert(0, b.clone()));
        }
        active_bot.set(Some(b));
    });

    // Open a channel that was popped out into Messages (stays in the Messages section).
    let open_popout = use_callback(move |ch: ChannelItem| {
        is_settings_open.set(false);
        server_settings_open.set(false);
        if let Some(srv) = servers.peek().iter().find(|s| s.id == ch.server_id).cloned() {
            selected_server.set(Some(srv));
        }
        select_channel(ch);
    });

    // Deep links: /invite/g/CODE, /invite/s/CODE, ?dm=username, ?bot=username
    use_hook(move || {
        spawn(async move {
            let Some(link) = platform::take_deep_link().await else {
                return;
            };
            if let Some(rest) = link.strip_prefix("/invite/") {
                let mut it = rest.splitn(2, '/');
                let kind = match it.next() {
                    Some("g") => "group",
                    Some("s") => "server",
                    _ => return,
                };
                let code: String = it.next().unwrap_or("").chars().take_while(|c| c.is_ascii_alphanumeric()).take(64).collect();
                if !code.is_empty() {
                    invite_open.set(Some((kind.to_string(), code)));
                }
            } else if let Some(q) = link.split_once('?').map(|(_, q)| q.to_string()) {
                for pair in q.split('&') {
                    let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                    let v = urlencoding::decode(v).map(|c| c.into_owned()).unwrap_or_default();
                    match k {
                        "dm" => {
                            for _ in 0..20 {
                                if let Some(f) = friends.peek().iter().find(|f| f.username.eq_ignore_ascii_case(&v)).cloned() {
                                    select_friend.call(f);
                                    break;
                                }
                                platform::sleep_ms(300).await;
                            }
                        }
                        "bot" => {
                            if let Ok(b) = social::bot_by_name(&v).await {
                                open_bot.call(b);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
    });

    // Send DM message (also used for polls, notes, reminders, votes, images, invites)
    let uid_for_send = current_user_id.clone();
    let pkey_for_send = preview_storage_key.clone();
    let send_dm_text = use_callback(move |(raw_text, reply_to): (String, Option<String>)| {
        let Some(active) = active_friend() else {
            return;
        };
        if raw_text.trim().is_empty() || user_keys().is_none() {
            return;
        }
        let uid = uid_for_send.clone();
        let pkey = pkey_for_send.clone();
        let wire_text = if active.id == uid {
            let current = my_sessions.peek().iter().find(|x| x.is_current).cloned();
            match current {
                Some(sess) => {
                    let label = sess.device_label.clone().or(sess.device_name.clone()).unwrap_or_else(|| "Thiết bị".into());
                    format!("\u{2060}{}|{}\u{2060}{}", sess.id, label.replace(['|', '\u{2060}'], " "), raw_text)
                }
                None => raw_text.clone(),
            }
        } else {
            raw_text.clone()
        };
        is_sending.set(true);
        spawn(async move {
            match dm::send_encrypted_dm(&uid, &active.id, &wire_text, reply_to.as_deref()).await {
                Ok(sent) => {
                    messages.with_mut(|list| {
                        if !list.iter().any(|m| m.id == sent.id) {
                            list.push(DisplayMessage {
                                id: sent.id.clone(),
                                seq: sent.seq,
                                sender_id: uid.clone(),
                                recipient_id: active.id.clone(),
                                text: wire_text.clone(),
                                created_at: sent.created_at.clone(),
                                is_self: true,
                                status: MessageStatus::Sent,
                                reply_to_id: reply_to.clone(),
                                reactions: Vec::new(),
                                is_pinned: false,
                            });
                        }
                    });
                    last_previews.with_mut(|m| {
                        m.insert(active.id.clone(), wire_text.clone());
                        persist_previews(&pkey, m);
                    });
                    register_reminders(&uid, &[(raw_text.clone(), active.username.clone())]);
                    platform::scroll_chat_to_bottom(false);
                }
                Err(code) => {
                    let msg = match code.as_str() {
                        "blocked" => "Không gửi được: một trong hai người đã chặn người kia",
                        "not_friends" => "Không gửi được: hai bạn chưa là bạn bè",
                        "self_chat_disabled" => "\"Chính mình\" đã bị xóa. Tạo lại để tiếp tục ghi chú",
                        "missing_local_keys" => "Thiết bị này chưa có khóa mã hóa",
                        _ => "Không gửi được",
                    };
                    messages.with_mut(|list| {
                        list.push(DisplayMessage {
                            id: Uuid::new_v4().to_string(),
                            seq: 0,
                            sender_id: uid.clone(),
                            recipient_id: active.id.clone(),
                            text: msg.to_string(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            is_self: true,
                            status: MessageStatus::Error,
                            reply_to_id: None,
                            reactions: Vec::new(),
                            is_pinned: false,
                        });
                    });
                }
            }
            is_sending.set(false);
            platform::focus_chat_input();
        });
    });

    let uname_for_dm_cmd = current_username.clone();
    let trigger_send_message = use_callback(move |_: ()| {
        let Some(active) = active_friend() else {
            return;
        };
        let raw_text = input_text().trim().to_string();
        if raw_text.is_empty() || is_sending() || user_keys().is_none() {
            return;
        }
        let text = if raw_text.starts_with('/') {
            crate::components::plugins_view::try_run_plugin_command(&raw_text, &uname_for_dm_cmd).unwrap_or(raw_text)
        } else {
            raw_text
        };
        if is_currently_typing() {
            is_currently_typing.set(false);
            let next_seq = typing_seq() + 1;
            typing_seq.set(next_seq);
            let aid = active.id.clone();
            spawn(async move {
                let _ = api::typing_send(&aid, false).await;
            });
        }
        input_text.set(String::new());
        is_emoji_open.set(false);
        let reply = dm_reply_to().map(|(id, _)| id);
        dm_reply_to.set(None);
        send_dm_text.call((text, reply));
    });

    // Keep a copy of any message in "Chính mình": sent to myself, end-to-end encrypted.
    let uid_for_save = current_user_id.clone();
    let save_to_self = use_callback(move |text: String| {
        let uid = uid_for_save.clone();
        let current = my_sessions.peek().iter().find(|x| x.is_current).cloned();
        let wire = match current {
            Some(sess) => {
                let label = sess.device_label.clone().or(sess.device_name.clone()).unwrap_or_else(|| "Thiết bị".into());
                format!("\u{2060}{}|{}\u{2060}{}", sess.id, label.replace(['|', '\u{2060}'], " "), text)
            }
            None => text,
        };
        spawn(async move {
            match dm::send_encrypted_dm(&uid, &uid, &wire, None).await {
                Ok(_) => toast.set(Some("Đã lưu vào \"Chính mình\"".into())),
                Err(_) => toast.set(Some("Không lưu được vào \"Chính mình\"".into())),
            }
        });
    });

    // End-to-end encrypted image for direct messages: the image is sealed with a
    // fresh AES-256-GCM key, the ciphertext is uploaded, and the key travels only
    // inside the E2EE message.
    let send_dm_image = use_callback(move |(bytes, mime): (Vec<u8>, String)| {
        if bytes.len() > 8 * 1024 * 1024 {
            toast.set(Some("Ảnh lớn hơn 8 MB".into()));
            return;
        }
        spawn(async move {
            let sealed = match crate::crypto::blob::seal(&bytes) {
                Ok(s) => s,
                Err(_) => {
                    toast.set(Some("Không mã hóa được ảnh".into()));
                    return;
                }
            };
            match api::attachments_upload(sealed.cipher, "application/octet-stream").await {
                Ok(url) => {
                    let data = crate::components::rich::ImageData { u: url, k: sealed.key_b64, n: sealed.nonce_b64, m: mime };
                    if let Ok(json) = serde_json::to_string(&data) {
                        send_dm_text.call((format!("img:{json}"), None));
                    }
                }
                Err(_) => toast.set(Some("Không tải ảnh lên được".into())),
            }
        });
    });

    // Send channel message
    let uname_for_ch_cmd = current_username.clone();
    let trigger_send_channel_message = use_callback(move |_: ()| {
        let Some(ch) = selected_channel() else {
            return;
        };
        let raw_content = channel_input().trim().to_string();
        if raw_content.is_empty() {
            return;
        }
        let ent_srv_id = selected_server().filter(|s| s.is_enterprise).map(|s| s.id);
        if let Some(ref sid) = ent_srv_id {
            if raw_content == "/workspace" || raw_content.starts_with("/workspace ") {
                channel_input.set(String::new());
                selected_channel.set(None);
                active_ent_module.set(Some("overview".to_string()));
                return;
            }
            if raw_content.starts_with('/') {
                let mut parts = raw_content.splitn(2, char::is_whitespace);
                let cmd = parts.next().unwrap_or("").to_lowercase();
                let arg = parts.next().unwrap_or("").trim().to_string();
                let sid_cl = sid.clone();
                let ch_cl = ch.clone();
                if matches!(
                    cmd.as_str(),
                    "/task" | "/checkin" | "/checkout" | "/leave" | "/meet" | "/approve" | "/announce"
                ) {
                    channel_input.set(String::new());
                    spawn(async move {
                        let msg_opt: Option<String> = match cmd.as_str() {
                            "/task" => {
                                let title = if arg.is_empty() { "Công việc mới từ kênh chat".to_string() } else { arg };
                                let body = serde_json::json!({
                                    "title": title,
                                    "priority": "medium",
                                    "source_channel_id": ch_cl.id,
                                });
                                match enterprise::create_task(&sid_cl, body).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã tạo công việc doanh nghiệp".into()));
                                        Some(format!("[Công việc mới] {title}"))
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không tạo được công việc: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            "/checkin" => {
                                match enterprise::check_in(&sid_cl, "onsite", &arg).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã chấm công vào ca".into()));
                                        Some(if arg.is_empty() {
                                            "[Chấm công] Đã chấm công vào ca làm việc".to_string()
                                        } else {
                                            format!("[Chấm công] Đã chấm công vào ca ({arg})")
                                        })
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không chấm công được: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            "/checkout" => {
                                match enterprise::check_out(&sid_cl, &arg).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã chấm công tan ca".into()));
                                        Some(if arg.is_empty() {
                                            "[Chấm công] Đã chấm công kết thúc ca làm việc".to_string()
                                        } else {
                                            format!("[Chấm công] Đã chấm công tan ca ({arg})")
                                        })
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không chấm công tan ca được: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            "/leave" => {
                                let reason = if arg.is_empty() { "Nghỉ phép cá nhân".to_string() } else { arg };
                                let body = serde_json::json!({
                                    "leave_type": "annual",
                                    "start_date": "2026-11-20",
                                    "end_date": "2026-11-20",
                                    "half_day": "full",
                                    "reason": reason,
                                });
                                match enterprise::create_leave(&sid_cl, body).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã gửi đơn xin nghỉ phép".into()));
                                        Some(format!("[Nghỉ phép] Đã gửi đơn xin nghỉ phép: {reason}"))
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không gửi được đơn nghỉ phép: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            "/meet" => {
                                let title = if arg.is_empty() { "Họp đồng bộ nhanh".to_string() } else { arg };
                                let body = serde_json::json!({
                                    "title": title,
                                    "meeting_type": "video",
                                    "channel_id": ch_cl.id,
                                    "starts_at": "2026-11-20T09:00:00Z",
                                    "ends_at": "2026-11-20T10:00:00Z",
                                });
                                match enterprise::create_meeting(&sid_cl, body).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã đặt lịch cuộc họp".into()));
                                        Some(format!("[Cuộc họp] Đã đặt lịch họp: {title}"))
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không đặt được lịch họp: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            "/approve" => {
                                let title = if arg.is_empty() { "Đề xuất phê duyệt nội bộ".to_string() } else { arg };
                                let body = serde_json::json!({
                                    "title": title,
                                    "category": "custom",
                                    "form_data": { "channel": ch_cl.name },
                                });
                                match enterprise::submit_approval(&sid_cl, body).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã gửi đề xuất phê duyệt".into()));
                                        Some(format!("[Phê duyệt] Đã gửi đề xuất: {title}"))
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không gửi được đề xuất: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            "/announce" => {
                                let text = if arg.is_empty() { "Thông báo vận hành nội bộ".to_string() } else { arg };
                                let body = serde_json::json!({
                                    "title": text,
                                    "content": text,
                                    "scope": "company",
                                    "priority": "important",
                                    "pinned": false,
                                });
                                match enterprise::create_announcement(&sid_cl, body).await {
                                    Ok(_) => {
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                        toast.set(Some("Đã đăng thông báo nội bộ".into()));
                                        Some(format!("[Thông báo nội bộ] {text}"))
                                    }
                                    Err(e) => {
                                        toast.set(Some(format!("Không đăng được thông báo: {}", e.code)));
                                        None
                                    }
                                }
                            }
                            _ => None,
                        };
                        if let Some(post_txt) = msg_opt {
                            if let Ok(posted) = api::channels_post_message(&ch_cl.id, &post_txt).await {
                                channel_messages.with_mut(|list| {
                                    if !list.iter().any(|m| m.id == posted.id) {
                                        list.push(posted);
                                    }
                                });
                            }
                        }
                    });
                    return;
                }
            }
        }
        let content = if raw_content.starts_with('/') {
            crate::components::plugins_view::try_run_plugin_command(&raw_content, &uname_for_ch_cmd).unwrap_or(raw_content)
        } else {
            raw_content
        };
        channel_input.set(String::new());
        spawn(async move {
            if let Ok(posted) = api::channels_post_message(&ch.id, &content).await {
                channel_messages.with_mut(|list| {
                    if !list.iter().any(|m| m.id == posted.id) {
                        list.push(posted);
                    }
                });
            }
        });
    });

    // Send group message
    let uname_for_grp_cmd = current_username.clone();
    let trigger_send_group_message = use_callback(move |_: ()| {
        let Some(grp) = active_group() else {
            return;
        };
        let raw_text = group_input().trim().to_string();
        if raw_text.is_empty() {
            return;
        }
        let text = if raw_text.starts_with('/') {
            crate::components::plugins_view::try_run_plugin_command(&raw_text, &uname_for_grp_cmd).unwrap_or(raw_text)
        } else {
            raw_text
        };
        group_input.set(String::new());
        let reply = group_reply_to().map(|(id, _)| id);
        group_reply_to.set(None);
        let gid = grp.id.clone();
        let cid = active_group_channel().map(|c| c.id);
        spawn(async move {
            if let Ok(msg) = groups::groups_send_message(&gid, cid.as_deref(), &text, reply).await {
                group_messages.with_mut(|list| {
                    if !list.iter().any(|m| m.id == msg.id) {
                        list.push(msg);
                    }
                });
                platform::scroll_chat_to_bottom(false);
            }
        });
    });

    let send_group_raw = use_callback(move |raw: String| {
        let Some(grp) = active_group() else {
            return;
        };
        let cid = active_group_channel().map(|c| c.id);
        spawn(async move {
            match groups::groups_send_message(&grp.id, cid.as_deref(), &raw, None).await {
                Ok(msg) => {
                    group_messages.with_mut(|list| {
                        if !list.iter().any(|m| m.id == msg.id) {
                            list.push(msg);
                        }
                    });
                    platform::scroll_chat_to_bottom(false);
                }
                Err(e) => toast.set(Some(match e.code.as_str() {
                    "insufficient_permissions" | "forbidden" => "Bạn không có quyền thực hiện việc này trong nhóm".to_string(),
                    _ => "Không gửi được".to_string(),
                })),
            }
        });
    });
    let send_group_image = use_callback(move |(bytes, mime): (Vec<u8>, String)| {
        spawn(async move {
            match library::upload_media(bytes, &mime).await {
                Ok(url) => {
                    let data = crate::components::rich::ImageData { u: url, k: String::new(), n: String::new(), m: mime };
                    if let Ok(json) = serde_json::to_string(&data) {
                        send_group_raw.call(format!("img:{json}"));
                    }
                }
                Err(_) => toast.set(Some("Không tải ảnh lên được".into())),
            }
        });
    });

    // ---------------------------------------------------------------------
    // Derived view state
    // ---------------------------------------------------------------------
    let total_unread: usize = unread_counts.read().values().copied().sum();
    let pending_len = pending_requests.read().len();
    let active_mod_page = crate::components::plugins_view::ACTIVE_PLUGIN_PAGE.read().clone();
    let active_left_plugin = crate::components::plugins_view::ACTIVE_LEFT_PLUGIN.read().clone();
    let server_settings_active = server_settings_open() && selected_server().is_some();
    let is_chat_selected = active_friend().is_some()
        || active_group().is_some()
        || selected_channel().is_some()
        || active_ent_module().is_some()
        || active_bot().is_some()
        || active_mod_page.is_some()
        || is_settings_open()
        || server_settings_active
        || matches!(nav_section(), NavSection::Search | NavSection::Feed | NavSection::VicoAi);
    let section = if is_settings_open() { NavSection::Settings } else { nav_section() };

    let start_call = move |kind: &'static str, user: Option<String>, group: Option<String>| {
        spawn(async move {
            if let Ok(sess) = calls::calls_start(kind, user, group).await {
                active_call_session.set(Some(sess));
            }
        });
    };

    let mut jump_to_message = move |id: String| {
        highlight_msg.set(Some(id.clone()));
        let id_json = serde_json::to_string(&format!("msg-{id}")).unwrap_or_default();
        let _ = dioxus::document::eval(&format!(
            "const el = document.getElementById({id_json}); if (el) el.scrollIntoView({{ block: 'center', behavior: 'smooth' }});"
        ));
        spawn(async move {
            platform::sleep_ms(2000).await;
            highlight_msg.set(None);
        });
    };

    let refresh_friend_state = move || {
        spawn(async move {
            if let Ok(list) = api::friends_list().await {
                friends.set(list);
            }
            if let Ok(reqs) = api::friends_get_requests().await {
                pending_requests.set(reqs);
            }
        });
    };

    let username_for_add = current_username.clone();
    let add_friend_submit = move |_| {
        let target = friend_input().trim().to_string();
        if target.is_empty() || is_adding_friend() {
            return;
        }
        if target.to_lowercase() == username_for_add.to_lowercase() {
            add_friend_status.set(Some(AddFriendStatus { is_error: true, message: "Không thể tự kết bạn với chính mình".to_string() }));
            return;
        }
        is_adding_friend.set(true);
        add_friend_status.set(None);
        spawn(async move {
            match api::friends_send_request_by_username(&target).await {
                Ok(_) => {
                    friend_input.set(String::new());
                    add_friend_status.set(Some(AddFriendStatus { is_error: false, message: "Đã gửi lời mời kết bạn".to_string() }));
                    refresh_friend_state();
                }
                Err(err) => {
                    let msg = match err.code.as_str() {
                        "receiver_not_found" => "Không tìm thấy người dùng",
                        "already_friends" => "Hai người đã là bạn bè",
                        "request_already_pending" => "Đã gửi lời mời trước đó",
                        _ => "Không thể gửi lời mời",
                    };
                    add_friend_status.set(Some(AddFriendStatus { is_error: true, message: msg.to_string() }));
                }
            }
            is_adding_friend.set(false);
        });
    };

    // DM messages → view model ("Chính mình" shows the sending device as the sender)
    let self_mode = active_friend().as_ref().is_some_and(|f| f.id == current_user_id);
    let sessions_now = my_sessions();
    let current_sid = sessions_now.iter().find(|x| x.is_current).map(|x| x.id.clone());
    let dm_view: Vec<ViewMessage> = match active_friend() {
        Some(af) => {
            let list = messages.read();
            list.iter()
                .map(|m| {
                    let (dev, body) = if self_mode { split_device_prefix(&m.text) } else { (None, m.text.clone()) };
                    let (dev_sid, dev_label) = match dev.as_deref().and_then(|d| d.split_once('|')) {
                        Some((a, b)) => (Some(a.to_string()), b.to_string()),
                        None => (None, dev.clone().unwrap_or_default()),
                    };
                    let live = dev_sid
                        .as_ref()
                        .and_then(|sid| sessions_now.iter().find(|x| &x.id == sid))
                        .and_then(|x| x.device_label.clone().or(x.device_name.clone()));
                    let label = live.unwrap_or_else(|| if dev_label.is_empty() { "Thiết bị này".to_string() } else { dev_label.clone() });
                    let _ = &current_sid;
                    let mine = if self_mode { true } else { m.is_self };
                    ViewMessage {
                        id: m.id.clone(),
                        sender_id: if self_mode { dev_sid.clone().unwrap_or_default() } else { m.sender_id.clone() },
                        sender_name: if self_mode { label } else if m.is_self { current_username.clone() } else { af.username.clone() },
                        sender_avatar: if self_mode { None } else if m.is_self { my_avatar_url() } else { af.avatar_url.clone() },
                        text: body,
                        created_at: m.created_at.clone(),
                        is_self: mine,
                        is_error: m.status == MessageStatus::Error,
                        is_pending: false,
                        reply_text: m.reply_to_id.as_ref().map(|rid| {
                            list.iter()
                                .find(|x| &x.id == rid)
                                .map(|x| preview_text(&split_device_prefix(&x.text).1))
                                .unwrap_or_else(|| "Tin nhắn gốc không còn".to_string())
                        }),
                        reactions: m
                            .reactions
                            .iter()
                            .map(|(e, users)| (e.clone(), users.len(), users.iter().any(|u| u == &current_user_id)))
                            .collect(),
                        is_pinned: m.is_pinned,
                        is_bot: false,
                        highlighted: false,
                    }
                })
                .collect()
        }
        None => Vec::new(),
    };
    let dm_pins: Vec<(String, String, String)> =
        dm_view.iter().filter(|m| m.is_pinned).map(|m| (m.id.clone(), preview_text(&m.text), m.sender_name.clone())).collect();
    let dm_seen_id: Option<String> = if self_mode {
        None
    } else {
        partner_read_seq().and_then(|rs| {
            messages
                .read()
                .iter()
                .filter(|m| m.is_self && m.seq > 0 && m.seq <= rs && m.status != MessageStatus::Error)
                .last()
                .map(|m| m.id.clone())
        })
    };
    let dm_mentions: Vec<MentionCandidate> = active_friend()
        .filter(|f| f.id != current_user_id)
        .map(|f| {
            vec![MentionCandidate {
                username: f.username.clone(),
                display: f.display_name.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| f.username.clone()),
                avatar_url: f.avatar_url.clone(),
            }]
        })
        .unwrap_or_default();
    let active_blocked = active_friend().as_ref().is_some_and(|f| blocked_ids.read().contains(&f.id));

    // Group role context
    let roles: HashMap<String, String> = group_members.read().iter().map(|m| (m.user_id.clone(), m.role.clone())).collect();
    let gs_now = group_settings();
    let highlight_admins = gs_now.as_ref().is_some_and(|s| s.highlight_admins);
    let my_group_role = gs_now
        .as_ref()
        .and_then(|s| s.my_role.clone())
        .or_else(|| active_group().and_then(|g| g.role.clone()))
        .unwrap_or_else(|| "member".into());
    let is_group_admin = matches!(my_group_role.as_str(), "owner" | "admin");
    let g_can = |f: fn(&GroupSettings) -> bool| is_group_admin || gs_now.as_ref().map(f).unwrap_or(true);
    let (g_send, g_poll, g_note, g_reminder, g_pin) =
        (g_can(|s| s.perm_send), g_can(|s| s.perm_poll), g_can(|s| s.perm_note), g_can(|s| s.perm_reminder), g_can(|s| s.perm_pin));
    let group_mentions: Vec<MentionCandidate> = {
        let mut v: Vec<MentionCandidate> = group_members
            .read()
            .iter()
            .filter(|m| m.user_id != current_user_id)
            .map(|m| MentionCandidate { username: m.username.clone(), display: m.username.clone(), avatar_url: m.avatar_url.clone() })
            .collect();
        if !v.is_empty() {
            v.insert(0, MentionCandidate { username: "all".into(), display: "Tất cả thành viên".into(), avatar_url: None });
        }
        v
    };
    let quick_replies: Vec<(String, String)> = business.read().quick_replies.iter().map(|q| (q.shortcut.clone(), q.text.clone())).collect();
    let (wp_cls, wp_style) = wallpaper_attrs(chat_wallpaper().as_deref(), &prefs.read().wallpaper);
    let active_conv_key: Option<String> = if let Some(b) = active_bot() {
        Some(format!("bot:{}", b.id))
    } else if let Some(f) = active_friend() {
        Some(if f.id == current_user_id { "self".to_string() } else { format!("dm:{}", f.id) })
    } else if let Some(g) = active_group() {
        Some(format!("group:{}", g.id))
    } else {
        selected_channel().map(|c| format!("channel:{}", c.id))
    };
    let popped_now = selected_channel()
        .as_ref()
        .is_some_and(|c| conv_prefs.read().iter().any(|p| p.conv_type == "channel" && p.conv_id == c.id && p.in_messages));
    let community_groups: Vec<GroupItem> = groups()
        .into_iter()
        .filter(|g| conv_prefs.read().iter().any(|p| p.conv_type == "group" && p.conv_id == g.id && p.in_communities))
        .collect();

    // Optimistic conversation preference update (pin, labels, mute, placement).
    let mut apply_pref = move |(ct, cid, fields): (String, String, serde_json::Value)| {
        conv_prefs.with_mut(|list| {
            let idx = list.iter().position(|p| p.conv_type == ct && p.conv_id == cid);
            let mut p = idx.map(|i| list[i].clone()).unwrap_or(ConvPref {
                conv_type: ct.clone(),
                conv_id: cid.clone(),
                pinned_at: None,
                label_ids: Vec::new(),
                in_messages: false,
                in_communities: false,
                muted: false,
            });
            if let Some(v) = fields.get("pinned").and_then(|v| v.as_bool()) {
                p.pinned_at = if v { Some(chrono::Utc::now().to_rfc3339()) } else { None };
            }
            if let Some(v) = fields.get("label_ids").and_then(|v| v.as_array()) {
                p.label_ids = v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
            }
            if let Some(v) = fields.get("in_messages").and_then(|v| v.as_bool()) {
                p.in_messages = v;
            }
            if let Some(v) = fields.get("in_communities").and_then(|v| v.as_bool()) {
                p.in_communities = v;
            }
            if let Some(v) = fields.get("muted").and_then(|v| v.as_bool()) {
                p.muted = v;
            }
            match idx {
                Some(i) => list[i] = p,
                None => list.push(p),
            }
        });
        spawn(async move {
            if social::pref_set(&ct, &cid, fields).await.is_err() {
                toast.set(Some("Không lưu được thay đổi".into()));
                if let Ok(p) = social::prefs_list().await {
                    conv_prefs.set(p);
                }
            }
        });
    };
    let do_block = move |(uid, block): (String, bool)| {
        spawn(async move {
            let res = if block { social::block_user(&uid).await.map(|_| ()) } else { social::unblock_user(&uid).await.map(|_| ()) };
            match res {
                Ok(_) => {
                    blocked_ids.with_mut(|l| {
                        l.retain(|x| x != &uid);
                        if block {
                            l.push(uid.clone());
                        }
                    });
                    toast.set(Some(if block { "Đã chặn. Hai bạn không thể nhắn tin hay gọi cho nhau.".into() } else { "Đã bỏ chặn".into() }));
                }
                Err(_) => toast.set(Some("Không thực hiện được".into())),
            }
        });
    };
    let mut confirm_clear_self = use_signal(|| false);

    // Group messages → view model (reactions aggregated per emoji)
    let group_view: Vec<ViewMessage> = if active_group().is_some() {
        let list = group_messages.read();
        list.iter()
            .map(|m| {
                let mut agg: Vec<(String, usize, bool)> = Vec::new();
                for r in &m.reactions {
                    let mine = r.user_id == current_user_id;
                    if let Some(e) = agg.iter_mut().find(|e| e.0 == r.emoji) {
                        e.1 += 1;
                        e.2 |= mine;
                    } else {
                        agg.push((r.emoji.clone(), 1, mine));
                    }
                }
                ViewMessage {
                    id: m.id.clone(),
                    sender_id: m.sender_id.clone(),
                    sender_name: m.sender_username.clone(),
                    sender_avatar: m.sender_avatar_url.clone(),
                    text: m.content.clone(),
                    created_at: m.created_at.clone(),
                    is_self: m.sender_id == current_user_id,
                    is_error: false,
                    is_pending: false,
                    reply_text: m.reply_to_id.as_ref().map(|rid| {
                        list.iter()
                            .find(|x| &x.id == rid)
                            .map(|x| format!("{}: {}", x.sender_username, x.content))
                            .unwrap_or_else(|| "Tin nhắn gốc không còn".to_string())
                    }),
                    reactions: agg,
                    is_pinned: m.is_pinned,
                    is_bot: m.is_bot,
                    highlighted: highlight_admins && matches!(roles.get(&m.sender_id).map(|r| r.as_str()), Some("owner") | Some("admin")),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let group_pins: Vec<(String, String, String)> =
        group_view.iter().filter(|m| m.is_pinned).map(|m| (m.id.clone(), preview_text(&m.text), m.sender_name.clone())).collect();

    // Channel messages → view model
    let channel_view: Vec<ViewMessage> = channel_messages
        .read()
        .iter()
        .map(|m| ViewMessage {
            id: m.id.clone(),
            sender_id: m.sender_id.clone(),
            sender_name: m.sender_username.clone(),
            sender_avatar: m.sender_avatar_url.clone(),
            text: m.content.clone(),
            created_at: m.created_at.clone(),
            is_self: m.sender_id == current_user_id,
            is_error: false,
            is_pending: false,
            reply_text: None,
            reactions: Vec::new(),
            is_pinned: false,
            is_bot: false,
            highlighted: false,
        })
        .collect();

    let my_fingerprint = user_keys().map(|k| k.fingerprint_b64);
    let feedback = add_friend_status().map(|s| (s.is_error, s.message));

    rsx! {
        div { class: "workspace-shell",
            crate::components::plugins_view::PluginGlobalOverlay {}
            AppRail {
                active_section: section.clone(),
                hidden_mobile: is_chat_selected && active_left_plugin.is_none(),
                unread_dm_count: total_unread,
                pending_friend_count: pending_len,
                current_username: current_username.clone(),
                current_avatar_url: my_avatar_url(),
                pinned_servers: if prefs.read().pin_communities { servers() } else { Vec::new() },
                pinned_groups: if prefs.read().pin_groups { groups() } else { Vec::new() },
                active_group_id: active_group().map(|g| g.id),
                on_select_dm: move |_| {
                    is_settings_open.set(false);
                    server_settings_open.set(false);
                    nav_section.set(NavSection::DirectMessages);
                },
                on_select_friends: move |_| {
                    is_settings_open.set(false);
                    server_settings_open.set(false);
                    nav_section.set(NavSection::Friends);
                },
                on_select_communities: move |_| {
                    is_settings_open.set(false);
                    server_settings_open.set(false);
                    nav_section.set(NavSection::Communities);
                    spawn(async move {
                        if let Ok(list) = api::servers_list().await {
                            servers.set(list);
                        }
                    });
                },
                on_quick_switch: move |_| quick_switch_open.set(true),
                on_select_search: move |_| {
                    is_settings_open.set(false);
                    server_settings_open.set(false);
                    nav_section.set(NavSection::Search);
                },
                on_select_feed: move |_| {
                    is_settings_open.set(false);
                    server_settings_open.set(false);
                    nav_section.set(NavSection::Feed);
                },
                on_select_vico_ai: move |_| {
                    is_settings_open.set(false);
                    server_settings_open.set(false);
                    nav_section.set(NavSection::VicoAi);
                },
                on_select_server: move |srv: ServerItem| select_server(srv),
                on_select_group: move |g: GroupItem| select_group.call(g),
                on_open_profile: {
                    let me = current_user_id.clone();
                    move |_| viewing_profile_user_id.set(Some(me.clone()))
                },
                on_open_settings: move |_| {
                    server_settings_open.set(false);
                    is_settings_open.set(true);
                },
                on_logout: move |_| on_logout.call(()),
            }

            if !matches!(section, NavSection::Settings | NavSection::Search | NavSection::Feed | NavSection::VicoAi) && active_mod_page.is_none() {
                if let Some(lpid) = active_left_plugin.clone() {
                    aside { class: "nav-sidebar",
                        crate::components::plugins_view::PluginLeftAppView {
                            key: "{lpid}",
                            plugin_id: lpid,
                            my_username: current_username.clone(),
                            on_close: move |_| *crate::components::plugins_view::ACTIVE_LEFT_PLUGIN.write() = None,
                        }
                    }
                } else {
                    NavigationSidebar {
                    dm_content: rsx! {
                        ConversationList {
                            my_id: current_user_id.clone(),
                            friends: friends(),
                            groups: groups(),
                            bots: bot_chats(),
                            popouts: popouts(),
                            prefs: conv_prefs(),
                            labels: chat_labels(),
                            unread_counts: unread_counts(),
                            last_previews: last_previews(),
                            self_chat: privacy.read().self_chat,
                            active_key: active_conv_key.clone(),
                            blocked: blocked_ids(),
                            on_open: move |o: ConvOpen| match o {
                                ConvOpen::SelfChat => open_self_chat.call(()),
                                ConvOpen::Dm(f) => select_friend.call(f),
                                ConvOpen::Group(g) => select_group.call(g),
                                ConvOpen::Channel(c) => open_popout.call(c),
                                ConvOpen::Bot(b) => open_bot.call(b),
                            },
                            on_pref: apply_pref,
                            on_create_group: move |_| is_create_group_open.set(true),
                            on_create_self: move |_| {
                                spawn(async move {
                                    if social::self_chat_enable().await.is_ok() {
                                        privacy.with_mut(|p| p.self_chat = true);
                                        open_self_chat.call(());
                                    } else {
                                        toast.set(Some("Không tạo được \"Chính mình\"".into()));
                                    }
                                });
                            },
                            on_delete_self: move |_| confirm_clear_self.set(true),
                            on_labels_changed: move |l: Vec<ChatLabel>| chat_labels.set(l),
                            on_block: do_block,
                            on_mark_read: move |id: String| unread_counts.with_mut(|m| { m.remove(&id); }),
                            my_username: current_username.clone(),
                            my_avatar_url: my_avatar_url(),
                            my_status_text: my_status_text(),
                            my_status_emoji: my_status_emoji(),
                            on_my_status_updated: move |(st, se)| {
                                my_status_text.set(st);
                                my_status_emoji.set(se);
                            },
                            on_note_reply: move |(fr, draft_txt): (FriendItem, String)| {
                                select_friend.call(fr);
                                input_text.set(draft_txt);
                            },
                        }
                    },
                    community_groups: community_groups.clone(),
                    on_open_server_settings: move |_| {
                        is_settings_open.set(false);
                        server_settings_open.set(!server_settings_open());
                    },
                    is_server_settings_open: server_settings_active,
                    enterprise_config: enterprise_config(),
                    active_ent_module: if server_settings_active { None } else { active_ent_module() },
                    on_select_ent_module: move |m: String| {
                        server_settings_open.set(false);
                        selected_channel.set(None);
                        active_ent_module.set(Some(m));
                    },
                    active_section: section.clone(),
                    current_username: current_username.clone(),
                    current_avatar_url: my_avatar_url(),
                    hidden_mobile: is_chat_selected,
                    on_select_communities: move |_| {
                        is_settings_open.set(false);
                        server_settings_open.set(false);
                        nav_section.set(NavSection::Communities);
                        spawn(async move {
                            if let Ok(list) = api::servers_list().await {
                                servers.set(list);
                            }
                        });
                    },
                    servers: servers(),
                    on_select_server: move |srv: ServerItem| select_server(srv),
                    on_open_create_server: move |_| is_community_modal_open.set(true),
                    friends: friends(),
                    active_friend_id: active_friend().map(|f| f.id),
                    groups: groups(),
                    active_group_id: active_group().map(|g| g.id),
                    unread_counts: unread_counts(),
                    last_previews: last_previews(),
                    on_select_friend: move |f: FriendItem| select_friend.call(f),
                    on_select_group: move |g: GroupItem| select_group.call(g),
                    on_open_create_group: move |_| is_create_group_open.set(true),
                    friend_input: friend_input(),
                    search_results: search_results(),
                    pending_requests: pending_requests(),
                    add_friend_feedback: feedback,
                    is_adding_friend: is_adding_friend(),
                    on_friend_input_change: move |v: String| {
                        friend_input.set(v);
                        add_friend_status.set(None);
                    },
                    on_add_friend_submit: add_friend_submit,
                    on_send_friend_request_by_id: move |rid: String| {
                        spawn(async move {
                            if api::friends_send_request_by_id(&rid).await.is_ok() {
                                search_results.with_mut(|list| {
                                    for u in list.iter_mut() {
                                        if u.id == rid {
                                            u.relationship = "pending_sent".to_string();
                                        }
                                    }
                                });
                                refresh_friend_state();
                            }
                        });
                    },
                    on_accept_friend_request: move |rid: String| {
                        spawn(async move {
                            if api::friends_accept_request(&rid).await.is_ok() {
                                refresh_friend_state();
                            }
                        });
                    },
                    selected_server: selected_server(),
                    server_categories: server_categories(),
                    server_channels: channels(),
                    selected_channel_id: if server_settings_active { None } else { selected_channel().map(|c| c.id) },
                    active_voice_channel: active_voice_channel(),
                    voice_participants: voice_participants(),
                    is_voice_muted: is_voice_muted(),
                    on_select_channel: move |ch: ChannelItem| select_channel(ch),
                    on_open_create_category: move |_| is_create_category_open.set(true),
                    on_open_create_channel: move |cat: Option<String>| create_channel_for.set(Some(cat)),
                    on_open_server_invite: move |_| is_server_invite_open.set(true),
                    on_toggle_voice_mute: move |_| {
                        if let Some(ch) = active_voice_channel() {
                            let next = !is_voice_muted();
                            is_voice_muted.set(next);
                            spawn(async move {
                                let _ = api::webrtc_update_voice_state(&ch.id, next).await;
                            });
                        }
                    },
                    on_leave_voice: move |_| {
                        if let Some(ch) = active_voice_channel() {
                            spawn(async move {
                                let _ = api::webrtc_leave_voice(&ch.id).await;
                                active_voice_channel.set(None);
                                voice_participants.set(Vec::new());
                            });
                        }
                    },
                    }
                }
            }

            if !matches!(section, NavSection::Settings | NavSection::Search | NavSection::Feed | NavSection::VicoAi) && active_mod_page.is_none() {
                div {
                    class: "panel-resizer resizer-left",
                    "data-resizer": "sidebar",
                    title: "Kéo để thay đổi kích thước danh sách",
                }
            }

            main { class: if is_chat_selected && active_left_plugin.is_none() { "main-stage" } else { "main-stage hidden-mobile" },
                if let Some(pid) = active_mod_page.clone() {
                    crate::components::plugins_view::PluginCustomPageView {
                        key: "{pid}",
                        plugin_id: pid,
                        my_username: current_username.clone(),
                        on_close: move |_| *crate::components::plugins_view::ACTIVE_PLUGIN_PAGE.write() = None,
                        on_open_bot_chat: move |b: BotItem| {
                            *crate::components::plugins_view::ACTIVE_PLUGIN_PAGE.write() = None;
                            open_bot.call(b);
                        },
                    }
                } else if is_settings_open() {
                    SettingsView {
                        current_user_id: current_user_id.clone(),
                        current_username: current_username.clone(),
                        my_avatar_url: my_avatar_url(),
                        user_keys: user_keys(),
                        on_close: move |_| is_settings_open.set(false),
                        on_open_profile: {
                            let me = current_user_id.clone();
                            move |_| viewing_profile_user_id.set(Some(me.clone()))
                        },
                        on_avatar_updated: move |url: String| {
                            my_avatar_url.set(Some(url.clone()));
                            on_avatar_updated.call(url);
                        },
                        on_keys_imported: move |k| user_keys.set(Some(k)),
                        on_username_changed: move |n: String| if let Some(h) = on_username_changed { h.call(n) },
                        on_replay_tour: move |_| tour_open.set(true),
                        on_lock_now: move |_| locked.set(true),
                        on_logout: move |_| on_logout.call(()),
                    }
                } else if server_settings_active {
                    if let Some(srv) = selected_server() {
                        crate::components::server_settings::ServerSettingsView {
                            key: "{srv.id}",
                            server: srv.clone(),
                            my_id: current_user_id.clone(),
                            on_close: {
                                let sid = srv.id.clone();
                                move |_| {
                                    server_settings_open.set(false);
                                    let sid = sid.clone();
                                    spawn(async move {
                                        if let Ok(ms) = api::servers_get_members(&sid).await {
                                            server_members.set(ms);
                                        }
                                        if let Ok(rs) = social::server_roles(&sid).await {
                                            server_roles.set(rs);
                                        }
                                    });
                                }
                            },
                            on_enterprise_changed: {
                                let sid = srv.id.clone();
                                move |_| {
                                    let sid = sid.clone();
                                    spawn(async move {
                                        if let Ok(list) = api::servers_list().await {
                                            if let Some(u) = list.iter().find(|x| x.id == sid).cloned() {
                                                selected_server.set(Some(u));
                                            }
                                            servers.set(list);
                                        }
                                        if let Ok(ecfg) = enterprise::get_config(&sid).await {
                                            enterprise_config.set(Some(ecfg));
                                        } else {
                                            enterprise_config.set(None);
                                            active_ent_module.set(None);
                                        }
                                        if let Ok(cats) = api::servers_get_categories(&sid).await {
                                            server_categories.set(cats);
                                        }
                                        if let Ok(chs) = api::servers_get_channels(&sid).await {
                                            channels.set(chs);
                                        }
                                        if let Ok(ms) = api::servers_get_members(&sid).await {
                                            server_members.set(ms);
                                        }
                                        if let Ok(rs) = social::server_roles(&sid).await {
                                            server_roles.set(rs);
                                        }
                                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                                    });
                                }
                            },
                        }
                    }
                } else if section == NavSection::Search {
                    SearchPage {
                        friends: friends(),
                        groups: groups(),
                        servers: servers(),
                        on_open_friend: move |f: FriendItem| select_friend.call(f),
                        on_open_group: move |g: GroupItem| select_group.call(g),
                        on_open_server: move |srv: ServerItem| select_server(srv),
                        on_open_bot: move |b: BotItem| open_bot.call(b),
                        on_joined: move |_| {
                            spawn(async move {
                                if let Ok(list) = api::servers_list().await {
                                    servers.set(list);
                                }
                                if let Ok(grps) = groups::groups_list().await {
                                    groups.set(grps);
                                }
                            });
                        },
                        on_add_friend: move |rid: String| {
                            spawn(async move {
                                if api::friends_send_request_by_id(&rid).await.is_ok() {
                                    toast.set(Some("Đã gửi lời mời kết bạn".into()));
                                }
                            });
                        },
                    }
                } else if section == NavSection::Feed {
                    FeedView {
                        my_id: current_user_id.clone(),
                        my_username: current_username.clone(),
                        my_avatar_url: my_avatar_url(),
                        my_status_text: my_status_text(),
                        my_status_emoji: my_status_emoji(),
                        friends: friends(),
                        refresh_tick: feed_refresh_tick(),
                        on_back: move |_| nav_section.set(NavSection::DirectMessages),
                        on_my_status_updated: move |(st, se)| {
                            my_status_text.set(st);
                            my_status_emoji.set(se);
                        },
                        on_open_friend: move |f: FriendItem| select_friend.call(f),
                        on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                        on_note_reply: move |(fr, draft_txt): (FriendItem, String)| {
                            select_friend.call(fr);
                            input_text.set(draft_txt);
                        },
                    }
                } else if section == NavSection::VicoAi {
                    VicoAiView {
                        my_username: current_username.clone(),
                        my_avatar_url: my_avatar_url(),
                        on_back: move |_| nav_section.set(NavSection::DirectMessages),
                    }
                } else if let Some(b) = active_bot() {
                    BotChatView {
                        key: "{b.id}",
                        bot: b.clone(),
                        my_id: current_user_id.clone(),
                        my_name: current_username.clone(),
                        incoming: bot_event(),
                        groups: groups(),
                        on_back: move |_| active_bot.set(None),
                    }
                } else if let Some(active) = active_friend() {
                    div { class: "chat-main-container",
                        div { class: "chat-column {wp_cls}", style: "{wp_style}",
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại danh sách",
                                    onclick: move |_| active_friend.set(None),
                                    BackIcon {}
                                }
                                if self_mode {
                                    div { class: "chat-header-identity",
                                        span { class: "self-chat-avatar", aria_hidden: "true", DevicesIcon { size: 20 } }
                                        div { class: "chat-header-info",
                                            h2 { class: "chat-header-title", "{t(\"chat.self\")}" }
                                            span { class: "chat-header-meta secure", ShieldIcon { size: 12 } "Chỉ mình bạn · {sessions_now.len()} thiết bị" }
                                        }
                                    }
                                } else {
                                    button {
                                        r#type: "button",
                                        class: "chat-header-identity",
                                        title: "Xem hồ sơ",
                                        onclick: {
                                            let id = active.id.clone();
                                            move |_| viewing_profile_user_id.set(Some(id.clone()))
                                        },
                                        Avatar { name: active.username.clone(), avatar_url: active.avatar_url.clone(), size: 38 }
                                        div { class: "chat-header-info",
                                            h2 { class: "chat-header-title", "{active.username}" }
                                            if active_blocked {
                                                span { class: "chat-header-meta", BlockIcon { size: 12 } " Đã chặn" }
                                            } else if is_friend_typing() && privacy.read().typing {
                                                span { class: "chat-header-meta", style: "color: var(--accent-text);", "{tn(\"chat.typing\", &active.username)}" }
                                            } else {
                                                span { class: "chat-header-meta secure", ShieldIcon { size: 12 } "{t(\"chat.e2ee\")}" }
                                            }
                                        }
                                    }
                                }
                                div { class: "chat-header-actions",
                                    if !self_mode && !active_blocked {
                                        button {
                                            r#type: "button",
                                            class: "icon-btn",
                                            title: "Gọi thoại",
                                            aria_label: "Gọi thoại",
                                            onclick: {
                                                let id = active.id.clone();
                                                move |_| start_call("audio", Some(id.clone()), None)
                                            },
                                            PhoneIcon { size: 20 }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "icon-btn",
                                            title: "Gọi video",
                                            aria_label: "Gọi video",
                                            onclick: {
                                                let id = active.id.clone();
                                                move |_| start_call("video", Some(id.clone()), None)
                                            },
                                            VideoIcon { size: 20 }
                                        }
                                    }
                                    if !self_mode {
                                        button {
                                            r#type: "button",
                                            "data-testid": "toggle-details",
                                            class: if is_partner_info_open() { "icon-btn active" } else { "icon-btn" },
                                            title: "Thông tin cuộc trò chuyện",
                                            aria_label: "Thông tin tài khoản",
                                            aria_pressed: "{is_partner_info_open()}",
                                            onclick: move |_| is_partner_info_open.set(!is_partner_info_open()),
                                            PanelRightIcon { size: 20 }
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: if chat_menu_open() { "icon-btn active" } else { "icon-btn" },
                                        title: "Tùy chọn khác",
                                        aria_label: "Tùy chọn khác",
                                        aria_haspopup: "menu",
                                        aria_expanded: "{chat_menu_open()}",
                                        onclick: move |e| { e.stop_propagation(); chat_menu_open.set(!chat_menu_open()); },
                                        MoreIcon { size: 18 }
                                    }
                                }
                                if chat_menu_open() {
                                    ChatMoreMenu {
                                        actions: if self_mode { vec![ChatAction::Wallpaper, ChatAction::ClearSelf] } else if active_blocked { vec![ChatAction::Wallpaper, ChatAction::Unblock] } else { vec![ChatAction::Wallpaper, ChatAction::Block] },
                                        on_close: move |_| chat_menu_open.set(false),
                                        on_action: {
                                            let pid = active.id.clone();
                                            move |a: ChatAction| match a {
                                                ChatAction::Wallpaper => wallpaper_picker_open.set(true),
                                                ChatAction::Block => do_block((pid.clone(), true)),
                                                ChatAction::Unblock => do_block((pid.clone(), false)),
                                                ChatAction::ClearSelf => confirm_clear_self.set(true),
                                                _ => {}
                                            }
                                        },
                                    }
                                }
                            }

                            if self_mode {
                                div { class: "device-strip", aria_label: "Thiết bị của bạn",
                                    span { class: "device-strip-label", "Thiết bị:" }
                                    for sess in sessions_now.iter().cloned() {
                                        {
                                            let label = sess.device_label.clone().or(sess.device_name.clone()).unwrap_or_else(|| "Thiết bị".into());
                                            let sid = sess.id.clone();
                                            let l2 = label.clone();
                                            rsx! {
                                                button { key: "{sess.id}", r#type: "button", class: if sess.is_current { "device-chip current" } else { "device-chip" },
                                                    title: "Bấm để đổi tên thiết bị",
                                                    onclick: move |_| rename_device.set(Some((sid.clone(), l2.clone()))),
                                                    DevicesIcon { size: 14 }
                                                    "{label}"
                                                    if sess.is_current { span { class: "device-chip-tag", "thiết bị này" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            PinnedBar {
                                items: dm_pins.clone(),
                                on_jump: move |id: String| jump_to_message(id),
                                on_unpin: move |mid: String| {
                                    spawn(async move {
                                        if api::messages_pin(&mid, false).await.is_ok() {
                                            messages.with_mut(|l| for m in l.iter_mut() { if m.id == mid { m.is_pinned = false; } });
                                        }
                                    });
                                },
                            }

                            if active_blocked {
                                div { class: "chat-banner danger", role: "status",
                                    BlockIcon { size: 16 }
                                    span { "Bạn đã chặn {active.username}. Hai bạn không thể nhắn tin hoặc gọi cho nhau." }
                                    button { r#type: "button", class: "btn-secondary", onclick: { let pid = active.id.clone(); move |_| do_block((pid.clone(), false)) }, "Bỏ chặn" }
                                }
                            }

                            MessageList {
                                messages: dm_view.clone(),
                                e2ee: true,
                                show_sender_names: self_mode,
                                intro_title: if self_mode { t("chat.self").to_string() } else { active.username.clone() },
                                intro_sub: if self_mode {
                                    "Ghi chú, liên kết, ảnh và nhắc hẹn chỉ mình bạn thấy, đồng bộ giữa các thiết bị đã đăng nhập. Mỗi tin nhắn ghi rõ thiết bị đã gửi; bấm vào tên thiết bị để đổi tên.".to_string()
                                } else {
                                    format!("Đây là khởi đầu cuộc trò chuyện trực tiếp giữa bạn và {}.", active.username)
                                },
                                highlight_id: highlight_msg(),
                                my_id: current_user_id.clone(),
                                my_username: current_username.clone(),
                                seen_id: dm_seen_id.clone(),
                                double_click_reply: prefs.read().double_click_reply,
                                on_open_profile: move |uid: String| {
                                    if self_mode {
                                        if let Some(sess) = my_sessions.peek().iter().find(|x| x.id == uid) {
                                            rename_device.set(Some((sess.id.clone(), sess.device_label.clone().or(sess.device_name.clone()).unwrap_or_default())));
                                        }
                                    } else {
                                        viewing_profile_user_id.set(Some(uid));
                                    }
                                },
                                on_profile_click: move |(uid, x, y): (String, f64, f64)| {
                                    if self_mode {
                                        if let Some(sess) = my_sessions.peek().iter().find(|s| s.id == uid) {
                                            rename_device.set(Some((sess.id.clone(), sess.device_label.clone().or(sess.device_name.clone()).unwrap_or_default())));
                                        }
                                    } else {
                                        profile_popover.set(Some((uid, x, y)));
                                    }
                                },
                                on_react: move |(mid, emoji): (String, String)| {
                                    spawn(async move {
                                        let _ = api::messages_toggle_reaction(&mid, &emoji).await;
                                    });
                                },
                                on_reply: move |m: ViewMessage| {
                                    dm_reply_to.set(Some((m.id.clone(), format!("{}: {}", m.sender_name, preview_text(&m.text)))));
                                    platform::focus_chat_input();
                                },
                                on_pin: move |(mid, pin): (String, bool)| {
                                    spawn(async move {
                                        match api::messages_pin(&mid, pin).await {
                                            Ok(_) => messages.with_mut(|l| for m in l.iter_mut() { if m.id == mid { m.is_pinned = pin; } }),
                                            Err(_) => toast.set(Some("Không ghim được tin nhắn".into())),
                                        }
                                    });
                                },
                                on_send_raw: move |raw: String| send_dm_text.call((raw, None)),
                                on_save_self: if self_mode { None } else { Some(EventHandler::new(move |t: String| save_to_self.call(t))) },
                                on_open_invite: move |(kind, code): (String, String)| invite_open.set(Some((kind, code))),
                            }

                            div { class: "typing-indicator", aria_live: "polite",
                                if is_friend_typing() && !self_mode && privacy.read().typing {
                                    span { class: "typing-dots", span {} span {} span {} }
                                    span { "{tn(\"chat.typing\", &active.username)}" }
                                }
                            }

                            Composer {
                                input_id: "dm-chat-input".to_string(),
                                value: input_text(),
                                placeholder: if self_mode { "Ghi chú cho chính mình".to_string() } else { tn("chat.message_to", &active.username) },
                                sending: is_sending(),
                                disabled: user_keys().is_none() || active_blocked,
                                disabled_reason: if active_blocked {
                                    Some(format!("Bạn đã chặn {}. Bỏ chặn để tiếp tục nhắn tin.", active.username))
                                } else if user_keys().is_none() {
                                    Some("Thiết bị này chưa có khóa mã hóa. Nhập khóa trong Cài đặt › Bảo mật để nhắn tin.".to_string())
                                } else {
                                    None
                                },
                                reply_to: dm_reply_to().map(|(_, t)| t),
                                on_cancel_reply: move |_| dm_reply_to.set(None),
                                mentions: dm_mentions.clone(),
                                quick_replies: quick_replies.clone(),
                                on_input: move |val: String| {
                                    input_text.set(val.clone());
                                    if let Some(af) = active_friend() {
                                        if af.id == *my_id_sig.peek() || !privacy.peek().typing {
                                            return;
                                        }
                                        if !is_currently_typing() && !val.trim().is_empty() {
                                            is_currently_typing.set(true);
                                            let fid = af.id.clone();
                                            spawn(async move {
                                                let _ = api::typing_send(&fid, true).await;
                                            });
                                        }
                                        let next_seq = typing_seq() + 1;
                                        typing_seq.set(next_seq);
                                        let fid = af.id.clone();
                                        spawn(async move {
                                            platform::sleep_ms(2500).await;
                                            if typing_seq() == next_seq && is_currently_typing() {
                                                is_currently_typing.set(false);
                                                let _ = api::typing_send(&fid, false).await;
                                            }
                                        });
                                    }
                                },
                                on_send: move |_| trigger_send_message.call(()),
                                on_sticker: move |url: String| {
                                    // Stickers travel inside the E2EE envelope like any other text.
                                    send_dm_text.call((url, None));
                                },
                                on_raw: move |raw: String| send_dm_text.call((raw, None)),
                                on_image: move |img: (Vec<u8>, String)| send_dm_image.call(img),
                            }
                        }

                        if is_partner_info_open() && !self_mode {
                            div {
                                class: "panel-resizer resizer-right",
                                "data-resizer": "details",
                                title: "Kéo để thay đổi kích thước chi tiết",
                            }
                            DmDetailsPanel {
                                user_id: active.id.clone(),
                                username: active.username.clone(),
                                avatar_url: active.avatar_url.clone(),
                                fingerprint: partner_fingerprint(),
                                my_fingerprint: my_fingerprint.clone(),
                                messages: dm_view.clone(),
                                on_close: move |_| is_partner_info_open.set(false),
                                on_focus_composer: move |_| platform::focus_chat_input(),
                                on_audio_call: {
                                    let id = active.id.clone();
                                    move |_| start_call("audio", Some(id.clone()), None)
                                },
                                on_video_call: {
                                    let id = active.id.clone();
                                    move |_| start_call("video", Some(id.clone()), None)
                                },
                                on_view_profile: {
                                    let id = active.id.clone();
                                    move |_| viewing_profile_user_id.set(Some(id.clone()))
                                },
                                on_jump_to: move |id: String| jump_to_message(id),
                            }
                        }
                    }
                } else if let Some(grp) = active_group() {
                    div { class: "chat-main-container",
                        div { class: "chat-column {wp_cls}", style: "{wp_style}",
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại danh sách",
                                    onclick: move |_| active_group.set(None),
                                    BackIcon {}
                                }
                                button {
                                    r#type: "button",
                                    class: "chat-header-identity",
                                    title: "Thông tin nhóm",
                                    onclick: move |_| is_group_info_open.set(true),
                                    Avatar { name: grp.name.clone(), avatar_url: grp.avatar_url.clone(), size: 38 }
                                    div { class: "chat-header-info",
                                        h2 { class: "chat-header-title", "{grp.name}" }
                                        span { class: "chat-header-meta", "{grp.member_count} thành viên" }
                                    }
                                }
                                div { class: "chat-header-actions",
                                    button {
                                        r#type: "button",
                                        class: "icon-btn",
                                        title: "Gọi thoại nhóm",
                                        aria_label: "Gọi thoại nhóm",
                                        onclick: {
                                            let gid = grp.id.clone();
                                            move |_| start_call("audio", None, Some(gid.clone()))
                                        },
                                        PhoneIcon { size: 20 }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "icon-btn",
                                        title: "Gọi video nhóm",
                                        aria_label: "Gọi video nhóm",
                                        onclick: {
                                            let gid = grp.id.clone();
                                            move |_| start_call("video", None, Some(gid.clone()))
                                        },
                                        VideoIcon { size: 20 }
                                    }
                                    button {
                                        r#type: "button",
                                        class: if is_group_info_open() { "icon-btn active" } else { "icon-btn" },
                                        title: "Thông tin nhóm",
                                        aria_label: "Thông tin nhóm",
                                        aria_pressed: "{is_group_info_open()}",
                                        onclick: move |_| is_group_info_open.set(!is_group_info_open()),
                                        PanelRightIcon { size: 20 }
                                    }
                                    button {
                                        r#type: "button",
                                        class: if chat_menu_open() { "icon-btn active" } else { "icon-btn" },
                                        title: "Tùy chọn khác",
                                        aria_label: "Tùy chọn khác",
                                        aria_haspopup: "menu",
                                        aria_expanded: "{chat_menu_open()}",
                                        onclick: move |e| { e.stop_propagation(); chat_menu_open.set(!chat_menu_open()); },
                                        MoreIcon { size: 18 }
                                        if is_group_admin && gs_now.as_ref().is_some_and(|s| s.pending_requests > 0) {
                                            span { class: "dot-badge", aria_label: "Có yêu cầu tham gia" }
                                        }
                                    }
                                }
                                if chat_menu_open() {
                                    ChatMoreMenu {
                                        actions: vec![ChatAction::ManageGroup, ChatAction::InviteFriends, ChatAction::CopyLink, ChatAction::Wallpaper],
                                        on_close: move |_| chat_menu_open.set(false),
                                        on_action: {
                                            let g = grp.clone();
                                            move |a: ChatAction| match a {
                                                ChatAction::Wallpaper => wallpaper_picker_open.set(true),
                                                ChatAction::ManageGroup => group_manage_open.set(true),
                                                ChatAction::InviteFriends => invite_friends_for.set(Some(("group".to_string(), g.id.clone()))),
                                                ChatAction::CopyLink => {
                                                    let gid = g.id.clone();
                                                    spawn(async move {
                                                        match social::group_invite_link(&gid).await {
                                                            Ok(code) => {
                                                                let url = format!("{}/invite/g/{}", platform::app_origin().await, code);
                                                                platform::copy_to_clipboard(&url);
                                                                toast.set(Some("Đã sao chép liên kết mời".into()));
                                                            }
                                                            Err(e) => toast.set(Some(if e.code == "link_join_disabled" { "Nhóm đang tắt tham gia bằng liên kết".into() } else { "Không lấy được liên kết mời".into() })),
                                                        }
                                                    });
                                                }
                                                _ => {}
                                            }
                                        },
                                    }
                                }
                            }

                            PinnedBar {
                                items: group_pins.clone(),
                                on_jump: move |id: String| jump_to_message(id),
                                on_unpin: {
                                    let gid = grp.id.clone();
                                    move |mid: String| {
                                        let g = gid.clone();
                                        spawn(async move {
                                            if let Ok(pinned) = groups::groups_pin_message(&g, &mid).await {
                                                group_messages.with_mut(|l| for m in l.iter_mut() { if m.id == mid { m.is_pinned = pinned; } });
                                            }
                                        });
                                    }
                                },
                            }

                            if group_channels.read().len() > 1 {
                                div { class: "sidebar-filters", style: "padding: var(--space-2) var(--space-4);", role: "tablist", aria_label: "Kênh của nhóm",
                                    for ch in group_channels() {
                                        {
                                            let is_cur = active_group_channel().as_ref().map(|c| c.id == ch.id).unwrap_or(false);
                                            let ch_clone = ch.clone();
                                            let gid = grp.id.clone();
                                            rsx! {
                                                button {
                                                    key: "{ch.id}",
                                                    r#type: "button",
                                                    role: "tab",
                                                    aria_selected: "{is_cur}",
                                                    class: if is_cur { "filter-chip active" } else { "filter-chip" },
                                                    onclick: move |_| {
                                                        let c = ch_clone.clone();
                                                        active_group_channel.set(Some(c.clone()));
                                                        let g = gid.clone();
                                                        spawn(async move {
                                                            if let Ok(msgs) = groups::groups_get_messages(&g, Some(&c.id), 50, None).await {
                                                                group_messages.set(msgs);
                                                            }
                                                            platform::scroll_chat_to_bottom(true);
                                                        });
                                                    },
                                                    "# {crate::components::navigation::sidebar::format_display_label(&ch.name)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            MessageList {
                                messages: group_view.clone(),
                                show_sender_names: true,
                                intro_title: grp.name.clone(),
                                intro_sub: format!("Nhóm trò chuyện với {} thành viên. Tin nhắn nhóm hiện được máy chủ lưu ở dạng thường, chưa mã hóa đầu cuối.", grp.member_count),
                                highlight_id: highlight_msg(),
                                on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                                on_profile_click: move |(uid, x, y): (String, f64, f64)| profile_popover.set(Some((uid, x, y))),
                                on_react: {
                                    let gid = grp.id.clone();
                                    move |(mid, emoji): (String, String)| {
                                        let g = gid.clone();
                                        spawn(async move {
                                            let _ = groups::groups_toggle_reaction(&g, &mid, &emoji).await;
                                        });
                                    }
                                },
                                on_reply: move |m: ViewMessage| {
                                    group_reply_to.set(Some((m.id.clone(), format!("{}: {}", m.sender_name, preview_text(&m.text)))));
                                    let _ = dioxus::document::eval("const el = document.getElementById('group-chat-input'); if (el) el.focus();");
                                },
                                my_id: current_user_id.clone(),
                                my_username: current_username.clone(),
                                double_click_reply: prefs.read().double_click_reply,
                                on_pin: if g_pin { Some(EventHandler::new({
                                    let gid = grp.id.clone();
                                    move |(mid, _pin): (String, bool)| {
                                        let g = gid.clone();
                                        spawn(async move {
                                            match groups::groups_pin_message(&g, &mid).await {
                                                Ok(pinned) => group_messages.with_mut(|l| for m in l.iter_mut() { if m.id == mid { m.is_pinned = pinned; } }),
                                                Err(_) => toast.set(Some("Bạn không có quyền ghim tin nhắn trong nhóm này".into())),
                                            }
                                        });
                                    }
                                })) } else { None },
                                on_send_raw: move |raw: String| send_group_raw.call(raw),
                                on_save_self: move |t: String| save_to_self.call(t),
                                on_open_invite: move |(kind, code): (String, String)| invite_open.set(Some((kind, code))),
                            }

                            Composer {
                                input_id: "group-chat-input".to_string(),
                                value: group_input(),
                                placeholder: tn("chat.message_to", &grp.name),
                                disabled: !g_send,
                                disabled_reason: if g_send { None } else { Some("Chỉ trưởng nhóm và phó nhóm được gửi tin nhắn trong nhóm này".to_string()) },
                                mentions: group_mentions.clone(),
                                quick_replies: quick_replies.clone(),
                                allow_poll: g_poll,
                                allow_note: g_note,
                                allow_reminder: g_reminder,
                                on_raw: move |raw: String| send_group_raw.call(raw),
                                on_image: move |img: (Vec<u8>, String)| send_group_image.call(img),
                                reply_to: group_reply_to().map(|(_, t)| t),
                                on_cancel_reply: move |_| group_reply_to.set(None),
                                on_input: move |v: String| group_input.set(v),
                                on_send: move |_| trigger_send_group_message.call(()),
                                on_sticker: {
                                    let gid = grp.id.clone();
                                    move |sticker_url: String| {
                                        let g = gid.clone();
                                        let cid = active_group_channel().map(|c| c.id);
                                        spawn(async move {
                                            if let Ok(msg) = groups::groups_send_message(&g, cid.as_deref(), &sticker_url, None).await {
                                                group_messages.with_mut(|list| {
                                                    if !list.iter().any(|m| m.id == msg.id) {
                                                        list.push(msg);
                                                    }
                                                });
                                                platform::scroll_chat_to_bottom(false);
                                            }
                                        });
                                    }
                                },
                            }
                        }

                        if is_group_info_open() {
                            button {
                                r#type: "button",
                                class: "drawer-backdrop",
                                aria_label: "Đóng thông tin nhóm",
                                tabindex: "-1",
                                onclick: move |_| is_group_info_open.set(false),
                            }
                            GroupInfoPanel {
                                group: grp.clone(),
                                current_user_id: current_user_id.clone(),
                                on_close: move |_| is_group_info_open.set(false),
                                on_open_invite: move |_| is_group_invite_open.set(true),
                                on_open_add_member: move |_| is_group_add_member_open.set(true),
                                on_group_deleted: move |_| {
                                    is_group_info_open.set(false);
                                    active_group.set(None);
                                    spawn(async move {
                                        if let Ok(grps) = groups::groups_list().await {
                                            groups.set(grps);
                                        }
                                    });
                                },
                                on_group_left: move |_| {
                                    is_group_info_open.set(false);
                                    active_group.set(None);
                                    spawn(async move {
                                        if let Ok(grps) = groups::groups_list().await {
                                            groups.set(grps);
                                        }
                                    });
                                },
                            }
                        }
                    }
                } else if let (Some(srv), Some(mod_key), Some(cfg)) = (selected_server(), active_ent_module(), enterprise_config()) {
                    if srv.is_enterprise {
                        EnterpriseWorkspace {
                            server_id: srv.id.clone(),
                            server_name: srv.name.clone(),
                            owner_id: srv.owner_id.clone(),
                            my_id: current_user_id.clone(),
                            module: mod_key,
                            cfg,
                            refresh_tick: ent_refresh_tick(),
                            members: server_members(),
                            roles: server_roles(),
                            members_open: is_server_members_open(),
                            on_toggle_members: move |_| is_server_members_open.set(!is_server_members_open()),
                            on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                            on_profile_click: move |(uid, x, y): (String, f64, f64)| profile_popover.set(Some((uid, x, y))),
                            on_jump_channel_msg: move |(cid, mid): (String, String)| {
                                if let Some(target_ch) = channels.peek().iter().find(|c| c.id == cid).cloned() {
                                    select_channel(target_ch);
                                    spawn(async move {
                                        platform::sleep_ms(140).await;
                                        jump_to_message(mid);
                                    });
                                }
                            },
                            on_select_module: move |m: String| active_ent_module.set(Some(m)),
                            on_close: move |_| active_ent_module.set(None),
                        }
                    } else {
                        div { class: "empty-state", div { class: "empty-state-title", "Chế độ doanh nghiệp hiện đang tắt" } }
                    }
                } else if let Some(ch) = selected_channel() {
                    div { class: "chat-main-container",
                        div { class: "chat-column wp-{prefs.read().wallpaper}",
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại danh sách kênh",
                                    onclick: move |_| selected_channel.set(None),
                                    BackIcon {}
                                }
                                div { class: "chat-header-identity",
                                    span { style: "color: var(--text-muted); display: flex;",
                                        if ch.channel_type == "voice" { VolumeIcon { size: 20 } } else { HashIcon { size: 20 } }
                                    }
                                    div { class: "chat-header-info",
                                        h2 { class: "chat-header-title", "{crate::components::navigation::sidebar::format_display_label(&ch.name)}" }
                                        if let Some(srv) = selected_server() {
                                            span { class: "chat-header-meta", "{srv.name}" }
                                        }
                                    }
                                }
                                div { class: "chat-header-actions",
                                    if ch.channel_type != "voice" {
                                        button { r#type: "button", class: "btn-ghost", title: "Sự kiện của cộng đồng", onclick: move |_| events_open.set(true),
                                            span { "📅" } span { class: "hide-mobile", "Sự kiện" } }
                                        button { r#type: "button", class: "btn-ghost", title: "Biệt danh, ảnh và giới thiệu riêng trong cộng đồng này", onclick: move |_| community_profile_open.set(true),
                                            crate::components::icons::UserIcon {} span { class: "hide-mobile", "Hồ sơ ở đây" } }
                                        button {
                                            r#type: "button",
                                            class: if popped_now { "btn-ghost active" } else { "btn-ghost" },
                                            title: if popped_now { "Bỏ kênh này khỏi mục Tin nhắn" } else { "Hiện kênh này trong mục Tin nhắn để đọc nhanh" },
                                            onclick: {
                                                let cid = ch.id.clone();
                                                move |_| {
                                                    apply_pref(("channel".to_string(), cid.clone(), serde_json::json!({ "in_messages": !popped_now })));
                                                    toast.set(Some(if popped_now { "Đã bỏ kênh khỏi Tin nhắn".into() } else { "Kênh đã xuất hiện trong mục Tin nhắn".into() }));
                                                }
                                            },
                                            MessageIcon { size: 16 }
                                            span { class: "hide-mobile", if popped_now { "Bỏ khỏi Tin nhắn" } else { "Đưa ra Tin nhắn" } }
                                        }
                                    }
                                    if let Some(srv) = selected_server() {
                                        ServerHeaderTools {
                                            server_id: srv.id.clone(),
                                            server_name: srv.name.clone(),
                                            is_enterprise: srv.is_enterprise,
                                            members: server_members(),
                                            members_open: is_server_members_open(),
                                            on_toggle_members: move |_| is_server_members_open.set(!is_server_members_open()),
                                            on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                                            on_jump_channel_msg: move |(cid, mid): (String, String)| {
                                                let same = selected_channel.peek().as_ref().map(|c| c.id == cid).unwrap_or(false);
                                                if same {
                                                    jump_to_message(mid);
                                                } else if let Some(target_ch) = channels.peek().iter().find(|c| c.id == cid).cloned() {
                                                    select_channel(target_ch);
                                                    spawn(async move {
                                                        platform::sleep_ms(140).await;
                                                        jump_to_message(mid);
                                                    });
                                                }
                                            },
                                            on_open_ent_module: if srv.is_enterprise {
                                                Some(EventHandler::new(move |m: String| active_ent_module.set(Some(m))))
                                            } else {
                                                None
                                            },
                                        }
                                    }
                                }
                            }

                            if ch.channel_type == "forum" {
                                crate::components::community_ui::ForumView { channel_id: ch.id.clone(), can_manage: can_manage_channels() }
                            } else if ch.channel_type == "event" {
                                if let Some(srv) = selected_server() {
                                    crate::components::community_ui::EventsPanel { server_id: srv.id.clone(), can_manage: can_manage_channels() }
                                }
                            } else if ch.channel_type != "voice" {
                                {
                                    let on_task_handler = if selected_server().is_some_and(|s| s.is_enterprise)
                                        && enterprise_config().is_some_and(|c| c.has(enterprise::ent_perm::CREATE_TASKS))
                                    {
                                        let cid = ch.id.clone();
                                        Some(EventHandler::new(move |(mid, txt): (String, String)| {
                                            create_task_from.set(Some((cid.clone(), mid, txt)));
                                        }))
                                    } else {
                                        None
                                    };
                                    rsx! {
                                        MessageList {
                                            messages: channel_view.clone(),
                                            show_sender_names: true,
                                            intro_title: format!("Chào mừng đến #{}", crate::components::navigation::sidebar::format_display_label(&ch.name)),
                                            intro_sub: "Đây là khởi đầu của kênh. Tin nhắn kênh cộng đồng được máy chủ lưu ở dạng thường, chưa mã hóa đầu cuối.".to_string(),
                                            highlight_id: highlight_msg(),
                                            on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                                            on_profile_click: move |(uid, x, y): (String, f64, f64)| profile_popover.set(Some((uid, x, y))),
                                            on_create_task: on_task_handler,
                                        }
                                    }
                                }
                                Composer {
                                    input_id: "channel-chat-input".to_string(),
                                    value: channel_input(),
                                    placeholder: if selected_server().is_some_and(|s| s.is_enterprise) {
                                        format!("Nhắn tin tới #{} (gõ / để dùng lệnh doanh nghiệp)", crate::components::navigation::sidebar::format_display_label(&ch.name))
                                    } else {
                                        format!("Nhắn tin tới #{}", crate::components::navigation::sidebar::format_display_label(&ch.name))
                                    },
                                    extra_commands: if selected_server().is_some_and(|s| s.is_enterprise) {
                                        vec![
                                            ("".into(), "svg:workspace".into(), "/workspace".into(), "Mở tổng quan không gian doanh nghiệp".into()),
                                            ("".into(), "svg:task".into(), "/task".into(), "Tạo công việc mới: /task <tiêu đề công việc>".into()),
                                            ("".into(), "svg:checkin".into(), "/checkin".into(), "Chấm công vào ca làm việc: /checkin [ghi chú]".into()),
                                            ("".into(), "svg:checkout".into(), "/checkout".into(), "Chấm công tan ca làm việc: /checkout [ghi chú]".into()),
                                            ("".into(), "svg:leave".into(), "/leave".into(), "Gửi đơn xin nghỉ phép: /leave <lý do>".into()),
                                            ("".into(), "svg:meet".into(), "/meet".into(), "Đặt lịch họp nhanh: /meet <tiêu đề cuộc họp>".into()),
                                            ("".into(), "svg:approve".into(), "/approve".into(), "Gửi đề xuất phê duyệt: /approve <tiêu đề đề xuất>".into()),
                                            ("".into(), "svg:announce".into(), "/announce".into(), "Đăng thông báo nội bộ: /announce <nội dung>".into()),
                                        ]
                                    } else {
                                        Vec::new()
                                    },
                                    on_input: move |v: String| channel_input.set(v),
                                    on_send: move |_| trigger_send_channel_message.call(()),
                                    on_sticker: move |url: String| {
                                        channel_input.set(url);
                                        trigger_send_channel_message.call(());
                                    },
                                }
                            } else {
                                {
                                    let connected = active_voice_channel().as_ref().map(|v| v.id == ch.id).unwrap_or(false);
                                    let parts: Vec<VoiceStateItem> = voice_participants().into_iter().filter(|p| p.channel_id == ch.id).collect();
                                    let ch_join = ch.clone();
                                    rsx! {
                                        div { class: "voice-room-stage",
                                            div { class: "empty-state-icon", VolumeIcon { size: 28 } }
                                            div {
                                                div { class: "voice-room-title", "{crate::components::navigation::sidebar::format_display_label(&ch.name)}" }
                                                div { class: "voice-room-sub",
                                                    if connected { "Bạn đang ở trong kênh thoại" } else { "Bạn chưa tham gia kênh thoại này" }
                                                }
                                            }
                                            if !parts.is_empty() {
                                                div { class: "voice-participants-grid",
                                                    for p in parts {
                                                        div { key: "{p.user_id}", class: "voice-participant-card",
                                                            Avatar { name: p.username.clone(), avatar_url: p.avatar_url.clone(), size: 64 }
                                                            span { class: "voice-participant-name", "{p.username}" }
                                                            if p.self_mute {
                                                                span { class: "chat-item-tag", "Đã tắt mic" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "voice-controls-dock",
                                                if connected {
                                                    button {
                                                        r#type: "button",
                                                        class: if is_voice_muted() { "call-btn call-btn-danger" } else { "call-btn" },
                                                        title: if is_voice_muted() { "Bật micro" } else { "Tắt micro" },
                                                        aria_label: if is_voice_muted() { "Bật micro" } else { "Tắt micro" },
                                                        onclick: move |_| {
                                                            if let Some(vc) = active_voice_channel() {
                                                                let next = !is_voice_muted();
                                                                is_voice_muted.set(next);
                                                                spawn(async move {
                                                                    let _ = api::webrtc_update_voice_state(&vc.id, next).await;
                                                                });
                                                            }
                                                        },
                                                        if is_voice_muted() { MicOffIcon { size: 20 } } else { MicIcon { size: 20 } }
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "call-btn call-btn-danger",
                                                        title: "Rời kênh thoại",
                                                        aria_label: "Rời kênh thoại",
                                                        onclick: move |_| {
                                                            if let Some(vc) = active_voice_channel() {
                                                                spawn(async move {
                                                                    let _ = api::webrtc_leave_voice(&vc.id).await;
                                                                    active_voice_channel.set(None);
                                                                    voice_participants.set(Vec::new());
                                                                });
                                                            }
                                                        },
                                                        PhoneOffIcon { size: 20 }
                                                    }
                                                } else {
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-primary",
                                                        onclick: move |_| select_channel(ch_join.clone()),
                                                        VolumeIcon { size: 18 }
                                                        span { "Tham gia kênh thoại" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if is_server_members_open() {
                            if let Some(srv) = selected_server() {
                                div {
                                    class: "panel-resizer resizer-right",
                                    "data-resizer": "details",
                                    title: "Kéo để thay đổi kích thước danh sách thành viên",
                                }
                                ServerMembersPanel {
                                    owner_id: srv.owner_id.clone(),
                                    members: server_members(),
                                    roles: server_roles(),
                                    on_profile_click: move |(uid, x, y): (String, f64, f64)| profile_popover.set(Some((uid, x, y))),
                                    on_close: move |_| is_server_members_open.set(false),
                                }
                            }
                        }
                    }
                } else {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", MessageIcon { size: 28 } }
                        if matches!(section, NavSection::Communities) {
                            div { class: "empty-state-title", "Chọn một cộng đồng" }
                            div { class: "empty-state-sub", "Mỗi cộng đồng có nhiều kênh văn bản và kênh thoại theo danh mục. Tạo mới hoặc tham gia bằng mã mời." }
                            button {
                                r#type: "button",
                                class: "btn-primary",
                                onclick: move |_| is_community_modal_open.set(true),
                                span { "Tạo hoặc tham gia cộng đồng" }
                            }
                        } else if matches!(section, NavSection::Server(_)) {
                            div { class: "empty-state-title", "Chọn một kênh" }
                            div { class: "empty-state-sub", "Chọn kênh văn bản để đọc tin nhắn, hoặc kênh thoại để tham gia trò chuyện." }
                        } else {
                            div { class: "empty-state-title", "Chọn một cuộc trò chuyện" }
                            div { class: "empty-state-sub", "Tin nhắn trực tiếp được mã hóa đầu cuối. Thêm bạn bè hoặc tạo nhóm để bắt đầu." }
                            div { style: "display: flex; gap: var(--space-2); flex-wrap: wrap; justify-content: center;",
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: move |_| nav_section.set(NavSection::Friends),
                                    UserPlusIcon { size: 18 }
                                    span { "Thêm bạn" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| is_create_group_open.set(true),
                                    GroupIcon {}
                                    span { "Tạo nhóm" }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ---------------- Modals & overlays ----------------
        if is_community_modal_open() {
            CommunityModal {
                on_close: move |_| is_community_modal_open.set(false),
                on_server_ready: move |srv: ServerItem| {
                    is_community_modal_open.set(false);
                    let s2 = srv.clone();
                    spawn(async move {
                        if let Ok(list) = api::servers_list().await {
                            servers.set(list);
                        }
                    });
                    select_server(s2);
                },
            }
        }

        if is_create_category_open() {
            if let Some(srv) = selected_server() {
                CreateCategoryModal {
                    server_id: srv.id.clone(),
                    on_close: move |_| is_create_category_open.set(false),
                    on_created: move |cat: ServerCategoryItem| {
                        is_create_category_open.set(false);
                        server_categories.with_mut(|l| l.push(cat));
                    },
                }
            }
        }

        if let Some(cat) = create_channel_for() {
            if let Some(srv) = selected_server() {
                CreateChannelModal {
                    server_id: srv.id.clone(),
                    category_id: cat,
                    on_close: move |_| create_channel_for.set(None),
                    on_created: move |ch: ChannelItem| {
                        create_channel_for.set(None);
                        channels.with_mut(|l| {
                            if !l.iter().any(|c| c.id == ch.id) {
                                l.push(ch);
                            }
                        });
                    },
                }
            }
        }

        if is_server_invite_open() {
            if let Some(srv) = selected_server() {
                InviteFriendsDialog {
                    kind: "server".to_string(),
                    target_id: srv.id.clone(),
                    name: srv.name.clone(),
                    avatar_url: srv.icon_url.clone(),
                    member_count: srv.member_count,
                    friends: friends(),
                    my_id: current_user_id.clone(),
                    my_name: current_username.clone(),
                    on_close: move |_| is_server_invite_open.set(false),
                }
            }
        }

        if is_create_group_open() {
            CreateGroupModal {
                friends: friends(),
                on_close: move |_| is_create_group_open.set(false),
                on_created: move |gid| {
                    is_create_group_open.set(false);
                    spawn(async move {
                        if let Ok(grps) = groups::groups_list().await {
                            groups.set(grps.clone());
                            if let Some(created) = grps.into_iter().find(|g| g.id == gid) {
                                select_group.call(created);
                            }
                        }
                    });
                },
            }
        }

        if is_group_invite_open() {
            if let Some(grp) = active_group() {
                InviteFriendsDialog {
                    kind: "group".to_string(),
                    target_id: grp.id.clone(),
                    name: grp.name.clone(),
                    avatar_url: grp.avatar_url.clone(),
                    member_count: grp.member_count,
                    friends: friends(),
                    my_id: current_user_id.clone(),
                    my_name: current_username.clone(),
                    on_close: move |_| is_group_invite_open.set(false),
                }
            }
        }
        if let Some((kind, tid)) = invite_friends_for() {
            {
                let (name, avatar, members) = if kind == "server" {
                    servers().into_iter().find(|x| x.id == tid).map(|x| (x.name, x.icon_url, x.member_count)).unwrap_or_default()
                } else {
                    groups().into_iter().find(|x| x.id == tid).map(|x| (x.name, x.avatar_url, x.member_count)).unwrap_or_default()
                };
                rsx! {
                    InviteFriendsDialog {
                        kind: kind.clone(),
                        target_id: tid.clone(),
                        name,
                        avatar_url: avatar,
                        member_count: members,
                        friends: friends(),
                        my_id: current_user_id.clone(),
                        my_name: current_username.clone(),
                        on_close: move |_| invite_friends_for.set(None),
                    }
                }
            }
        }
        if let Some((kind, code)) = invite_open() {
            InviteDialog {
                key: "{kind}-{code}",
                kind: kind.clone(),
                code: code.clone(),
                on_close: move |_| invite_open.set(None),
                on_joined: move |(k, id): (String, String)| {
                    invite_open.set(None);
                    spawn(async move {
                        if k == "server" {
                            if let Ok(list) = api::servers_list().await {
                                servers.set(list.clone());
                                if let Some(srv) = list.into_iter().find(|x| x.id == id) {
                                    select_server(srv);
                                }
                            }
                        } else if let Ok(grps) = groups::groups_list().await {
                            groups.set(grps.clone());
                            if let Some(g) = grps.into_iter().find(|x| x.id == id) {
                                select_group.call(g);
                            }
                        }
                    });
                },
            }
        }
        if let Some((cid, mid, preview)) = create_task_from() {
            if let Some(srv) = selected_server() {
                CreateTaskFromMsgDialog {
                    server_id: srv.id.clone(),
                    channel_id: cid,
                    message_id: mid,
                    preview,
                    on_close: move |_| create_task_from.set(None),
                    on_created: move |_| {
                        create_task_from.set(None);
                        ent_refresh_tick.set(ent_refresh_tick() + 1);
                        toast.set(Some("Đã tạo công việc từ tin nhắn".into()));
                    },
                }
            }
        }
        if group_manage_open() {
            if let Some(grp) = active_group() {
                GroupManageDialog {
                    group: grp.clone(),
                    my_id: current_user_id.clone(),
                    settings: group_settings(),
                    members: group_members(),
                    on_close: move |_| group_manage_open.set(false),
                    on_settings: move |st: GroupSettings| group_settings.set(Some(st)),
                    on_members_changed: {
                        let gid = grp.id.clone();
                        move |_| {
                            let g = gid.clone();
                            spawn(async move {
                                if let Ok(m) = groups::groups_get_members(&g).await {
                                    group_members.set(m);
                                }
                                if let Ok(st) = social::group_settings(&g).await {
                                    group_settings.set(Some(st));
                                }
                                if let Ok(grps) = groups::groups_list().await {
                                    groups.set(grps);
                                }
                            });
                        }
                    },
                    on_blocked: move |_| {
                        group_manage_open.set(false);
                        active_group.set(None);
                        toast.set(Some("Đã chặn và rời nhóm".into()));
                        spawn(async move {
                            if let Ok(grps) = groups::groups_list().await {
                                groups.set(grps);
                            }
                        });
                    },
                }
            }
        }
        if wallpaper_picker_open() {
            WallpaperPicker {
                current: chat_wallpaper(),
                shared: !self_mode,
                on_close: move |_| wallpaper_picker_open.set(false),
                on_pick: move |v: String| {
                    wallpaper_picker_open.set(false);
                    if let Some(f) = active_friend() {
                        if f.id == *my_id_sig.peek() {
                            platform::storage_set(&format!("chatw_selfwp_{}", f.id), &v);
                            chat_wallpaper.set(Some(v));
                        } else {
                            spawn(async move {
                                match social::wallpaper_set("dm", &f.id, &v).await {
                                    Ok(_) => chat_wallpaper.set(Some(v)),
                                    Err(_) => toast.set(Some("Không đổi được hình nền".into())),
                                }
                            });
                        }
                    } else if let Some(g) = active_group() {
                        spawn(async move {
                            match social::wallpaper_set("group", &g.id, &v).await {
                                Ok(_) => chat_wallpaper.set(Some(v)),
                                Err(_) => toast.set(Some("Không đổi được hình nền".into())),
                            }
                        });
                    }
                },
            }
        }
        if let Some((sid, label)) = rename_device() {
            RenameDeviceDialog {
                current: label.clone(),
                on_close: move |_| rename_device.set(None),
                on_save: move |n: String| {
                    rename_device.set(None);
                    let sid = sid.clone();
                    spawn(async move {
                        match social::session_rename(&sid, &n).await {
                            Ok(_) => {
                                if let Ok(list) = library::sessions_list().await {
                                    my_sessions.set(list);
                                }
                            }
                            Err(_) => toast.set(Some("Không đổi được tên thiết bị".into())),
                        }
                    });
                },
            }
        }
        if confirm_clear_self() {
            div { class: "modal-overlay", onclick: move |_| confirm_clear_self.set(false),
                div { class: "modal-content compact", role: "alertdialog", aria_modal: "true", onclick: move |e| e.stop_propagation(),
                    h3 { class: "modal-title", "Xóa \"{t(\"chat.self\")}\"?" }
                    p { class: "modal-sub", "Toàn bộ ghi chú trong cuộc trò chuyện với chính mình bị xóa vĩnh viễn trên mọi thiết bị. Bạn có thể tạo lại sau, nhưng không khôi phục được nội dung cũ." }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_clear_self.set(false), "{t(\"act.cancel\")}" }
                        button { r#type: "button", class: "btn-danger",
                            onclick: move |_| {
                                confirm_clear_self.set(false);
                                spawn(async move {
                                    match social::self_chat_delete().await {
                                        Ok(_) => {
                                            privacy.with_mut(|p| p.self_chat = false);
                                            let me = my_id_sig.peek().clone();
                                            if active_friend.peek().as_ref().is_some_and(|f| f.id == me) {
                                                active_friend.set(None);
                                            }
                                            last_previews.with_mut(|m| { m.remove(&me); });
                                            toast.set(Some("Đã xóa \"Chính mình\". Bạn có thể tạo lại ở đầu danh sách tin nhắn.".into()));
                                        }
                                        Err(_) => toast.set(Some("Không xóa được".into())),
                                    }
                                });
                            },
                            "Xóa vĩnh viễn"
                        }
                    }
                }
            }
        }
        if let Some((sid, w)) = welcome_for() {
            crate::components::community_ui::WelcomeDialog {
                server_id: sid, welcome: w,
                on_close: move |_| welcome_for.set(None),
                on_open_channel: move |cid: String| {
                    if let Some(ch) = channels.peek().iter().find(|c| c.id == cid).cloned() { select_channel(ch); }
                },
            }
        }
        if community_profile_open() {
            if let Some(srv) = selected_server() {
                crate::components::community_ui::CommunityProfileDialog { server_id: srv.id.clone(), server_name: srv.name.clone(), my_name: current_username.clone(), on_close: move |_| community_profile_open.set(false) }
            }
        }
        if events_open() {
            if let Some(srv) = selected_server() {
                div { class: "modal-overlay", onclick: move |_| events_open.set(false),
                    div { class: "modal-content events-modal", role: "dialog", aria_modal: "true", aria_label: "Sự kiện", onclick: move |e| e.stop_propagation(),
                        onkeydown: move |e| if e.key() == Key::Escape { events_open.set(false) },
                        crate::components::community_ui::EventsPanel { server_id: srv.id.clone(), can_manage: can_manage_channels(), on_close: move |_| events_open.set(false) }
                    }
                }
            }
        }
        if let Some(msg) = toast() {
            div { class: "app-toast", role: "status", aria_live: "polite",
                span { "{msg}" }
                button { r#type: "button", class: "icon-btn", aria_label: "Đóng thông báo", onclick: move |_| toast.set(None), crate::components::icons::CloseIcon { size: 14 } }
            }
        }

        if is_group_add_member_open() {
            if let Some(grp) = active_group() {
                AddMemberModal {
                    group_id: grp.id.clone(),
                    on_close: move |_| is_group_add_member_open.set(false),
                    on_member_added: move |_| is_group_add_member_open.set(false),
                }
            }
        }

        if let Some(incoming) = incoming_call_session() {
            {
                let inc_accept = incoming.clone();
                let inc_reject = incoming.clone();
                rsx! {
                    IncomingCallModal {
                        session: incoming,
                        on_accept: move |_| {
                            let s = inc_accept.clone();
                            spawn(async move {
                                if let Ok(accepted) = calls::calls_accept(&s.id).await {
                                    incoming_call_session.set(None);
                                    active_call_session.set(Some(accepted));
                                }
                            });
                        },
                        on_reject: move |_| {
                            let s = inc_reject.clone();
                            spawn(async move {
                                let _ = calls::calls_reject(&s.id).await;
                                incoming_call_session.set(None);
                            });
                        },
                    }
                }
            }
        }

        if let Some(session) = active_call_session() {
            ActiveCallOverlay {
                session: session.clone(),
                current_user_id: current_user_id.clone(),
                on_end_call: move |_| {
                    let s = session.clone();
                    spawn(async move {
                        let _ = calls::calls_end(&s.id).await;
                        active_call_session.set(None);
                        media::stop_all_media();
                    });
                },
            }
        }

        if let Some((uid, x, y)) = profile_popover() {
            ProfilePopover {
                key: "{uid}",
                user_id: uid.clone(),
                x: x,
                y: y,
                is_self: uid == current_user_id,
                effects_enabled: prefs.read().profile_effects,
                on_close: move |_| profile_popover.set(None),
                on_view_full: move |id: String| {
                    profile_popover.set(None);
                    viewing_profile_user_id.set(Some(id));
                },
                on_message: move |id: String| {
                    profile_popover.set(None);
                    if let Some(f) = friends.read().iter().find(|f| f.id == id).cloned() {
                        select_friend.call(f);
                    }
                },
            }
        }

        if locked() {
            crate::components::pin_lock::PinLock {
                username: current_username.clone(),
                on_unlock: move |_| locked.set(false),
                on_forgot: move |_| {
                    prefs_ctx.update(|p| {
                        p.pin_hash.clear();
                        p.pin_salt.clear();
                        p.lock_enabled = false;
                    });
                    locked.set(false);
                    on_logout.call(());
                },
            }
        }
        if tour_open() {
            WelcomeTour {
                on_close: { let k = tour_key.clone(); move |_| { platform::storage_set(&k, "done"); tour_open.set(false); } },
            }
        }

        if quick_switch_open() {
            QuickSwitcher {
                friends: friends(),
                groups: groups(),
                servers: servers(),
                on_close: move |_| quick_switch_open.set(false),
                on_pick_friend: move |f: FriendItem| { quick_switch_open.set(false); select_friend.call(f); },
                on_pick_group: move |g: GroupItem| { quick_switch_open.set(false); select_group.call(g); },
                on_pick_server: move |srv: ServerItem| { quick_switch_open.set(false); select_server(srv); },
                on_open_settings: move |_| { quick_switch_open.set(false); server_settings_open.set(false); is_settings_open.set(true); },
            }
        }

        if let Some(uid) = viewing_profile_user_id() {
            {
                let is_me = uid == current_user_id;
                rsx! {
                    ProfileModal {
                        user_id: uid.clone(),
                        is_self: is_me,
                        effects_enabled: prefs.read().profile_effects,
                        on_close: move |_| viewing_profile_user_id.set(None),
                        on_profile_updated: move |p: crate::models::UserProfileResponse| {
                            if let Some(av) = p.avatar_url {
                                my_avatar_url.set(Some(av));
                            }
                        },
                        on_message: move |id: String| {
                            viewing_profile_user_id.set(None);
                            if let Some(f) = friends.read().iter().find(|f| f.id == id).cloned() {
                                select_friend.call(f);
                            }
                        },
                    }
                }
            }
        }
    }
}
