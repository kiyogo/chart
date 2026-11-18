use dioxus::prelude::*;
use futures_util::StreamExt;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    components::{
        avatar::Avatar,
        calls::{ActiveCallOverlay, IncomingCallModal},
        emoji_picker::EmojiPicker,
        groups::{AddMemberModal, CreateGroupModal, GroupInfoPanel, GroupInviteModal},
        icons::{
            BackIcon, CloseIcon, EmojiIcon, FriendsIcon, GroupIcon, InfoIcon, LogoIcon,
            LogoutIcon, MenuIcon, PhoneIcon, PlusIcon, SendIcon, ServerIcon, SettingsIcon,
            VideoIcon,
        },
    },
    crypto::e2ee::{
        PeerKeys, decrypt_chat_message, encrypt_chat_message, get_or_verify_peer_keys,
        get_user_keys,
    },
    media,
    models::{
        CallSessionItem, ChannelItem, ChannelMessageItem, DisplayMessage, FriendItem, GroupItem,
        GroupMessageItem, MessageStatus, PendingRequestItem, ServerItem, UserSearchResult, WsEvent,
    },
    platform,
    services::{api, calls, groups, ws},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Chats,
    Groups,
    Friends,
    Servers,
}

#[derive(Clone, PartialEq)]
struct AddFriendStatus {
    is_error: bool,
    message: String,
}

fn format_badge(count: usize) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    }
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
) -> Element {
    let preview_storage_key = format!("chatw_previews_{current_user_id}");

    let mut user_keys = use_signal(|| get_user_keys(Some(&current_user_id)));
    let mut friends = use_signal(Vec::<FriendItem>::new);
    let mut active_friend = use_signal(|| Option::<FriendItem>::None);
    let mut messages = use_signal(Vec::<DisplayMessage>::new);
    let mut input_text = use_signal(String::new);
    let mut is_emoji_open = use_signal(|| false);
    let mut is_friend_typing = use_signal(|| false);
    let mut is_sending = use_signal(|| false);
    let mut is_settings_open = use_signal(|| false);

    // Avatar state
    let mut my_avatar_url = use_signal(|| current_user_avatar_url.clone());
    let mut is_uploading_avatar = use_signal(|| false);
    let mut avatar_upload_error = use_signal(|| Option::<String>::None);

    // Partner Info Sidebar (responsive auto-open on >= 1024px)
    let mut is_partner_info_open = use_signal(|| platform::initial_window_width() >= 1024);
    let mut partner_fingerprint = use_signal(|| Option::<String>::None);

    // Sidebar Tabs
    let mut sidebar_tab = use_signal(|| SidebarTab::Chats);
    let mut servers = use_signal(Vec::<ServerItem>::new);
    let mut selected_server = use_signal(|| Option::<ServerItem>::None);
    let mut channels = use_signal(Vec::<ChannelItem>::new);
    let mut selected_channel = use_signal(|| Option::<ChannelItem>::None);
    let mut channel_messages = use_signal(Vec::<ChannelMessageItem>::new);
    let mut channel_input = use_signal(String::new);
    let mut new_server_name = use_signal(String::new);
    let mut invite_code_input = use_signal(String::new);
    let mut active_voice_channel = use_signal(|| Option::<ChannelItem>::None);
    let mut is_voice_muted = use_signal(|| false);

    // Groups State
    let mut groups = use_signal(Vec::<GroupItem>::new);
    let mut active_group = use_signal(|| Option::<GroupItem>::None);
    let mut group_messages = use_signal(Vec::<GroupMessageItem>::new);
    let mut group_input = use_signal(String::new);
    let mut is_create_group_open = use_signal(|| false);
    let mut is_group_info_open = use_signal(|| false);
    let mut is_group_invite_open = use_signal(|| false);
    let mut is_group_add_member_open = use_signal(|| false);

    // Calls State
    let mut active_call_session = use_signal(|| Option::<CallSessionItem>::None);
    let mut incoming_call_session = use_signal(|| Option::<CallSessionItem>::None);

    // Friends & Search
    let mut friend_input = use_signal(String::new);
    let mut search_results = use_signal(Vec::<UserSearchResult>::new);
    let mut add_friend_status = use_signal(|| Option::<AddFriendStatus>::None);
    let mut is_adding_friend = use_signal(|| false);
    let mut pending_requests = use_signal(Vec::<PendingRequestItem>::new);

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
            platform::set_document_title(&format!("({combined}) chatw"));
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

    // Initial mount: load keys, friends, requests, servers, install resize/audio listeners, connect WS
    let uid_for_mount = current_user_id.clone();
    let pkey_for_mount = preview_storage_key.clone();
    use_effect(move || {
        platform::install_audio_unlock();

        // Check actual viewport width & subscribe to resize events
        spawn(async move {
            let w = platform::get_viewport_width().await;
            if w >= 1024 {
                is_partner_info_open.set(true);
            }
            let mut eval = dioxus::document::eval(
                r#"
                window.addEventListener('resize', () => {
                    dioxus.send(window.innerWidth || 0);
                });
                "#,
            );
            while let Ok(width) = eval.recv::<u32>().await {
                if width >= 1024 && !is_partner_info_open() {
                    is_partner_info_open.set(true);
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

                        if !friends.read().iter().any(|f| f.id == partner_id)
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
                                            });
                                        }
                                    });
                                    platform::scroll_chat_to_bottom(false);

                                    if !is_from_me
                                        && (sidebar_tab() != SidebarTab::Chats
                                            || is_settings_open())
                                    {
                                        unread_counts.with_mut(|m| {
                                            *m.entry(partner_id.clone()).or_insert(0) += 1;
                                        });
                                        platform::play_notification_chime();
                                    }
                                } else if !is_from_me {
                                    unread_counts.with_mut(|m| {
                                        *m.entry(partner_id.clone()).or_insert(0) += 1;
                                    });
                                    platform::play_notification_chime();
                                }
                            }
                            Err(_) => {
                                if is_for_active {
                                    messages.with_mut(|list| {
                                        list.push(DisplayMessage {
                                            id: id.clone(),
                                            seq,
                                            sender_id: sender_id.clone(),
                                            recipient_id: recipient_id.clone(),
                                            text: "Không thể giải mã tin nhắn".to_string(),
                                            created_at: created_at.clone(),
                                            is_self: false,
                                            status: MessageStatus::Error,
                                            reply_to_id: None,
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
                        platform::play_notification_chime();
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
                        if active_friend()
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
                        if active_group()
                            .as_ref()
                            .map(|g| g.id == msg.group_id)
                            .unwrap_or(false)
                        {
                            group_messages.with_mut(|list| {
                                if !list.iter().any(|m| m.id == msg.id) {
                                    list.push(msg.clone());
                                }
                            });
                            platform::scroll_chat_to_bottom(false);
                        }
                        if msg.sender_id != uid {
                            platform::play_notification_chime();
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
                    WsEvent::CallIncoming(payload) => {
                        let sess = payload.into_item();
                        incoming_call_session.set(Some(sess));
                        platform::play_notification_chime();
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
                    _ => {}
                }
            }
        });
    });

    // Helper: select a channel
    let mut select_channel = move |ch: ChannelItem| {
        selected_channel.set(Some(ch.clone()));
        active_friend.set(None);
        active_group.set(None);
        if ch.channel_type == "text" {
            let cid = ch.id.clone();
            spawn(async move {
                if let Ok(msgs) = api::channels_get_messages(&cid, 50).await {
                    channel_messages.set(msgs);
                }
            });
        } else if ch.channel_type == "voice" {
            let ch_clone = ch.clone();
            spawn(async move {
                if api::webrtc_join_voice(&ch_clone.id).await.is_ok() {
                    active_voice_channel.set(Some(ch_clone));
                }
            });
        }
    };

    // Helper: select a group
    let select_group = use_callback(move |grp: GroupItem| {
        is_settings_open.set(false);
        selected_channel.set(None);
        active_friend.set(None);
        active_group.set(Some(grp.clone()));
        group_messages.set(Vec::new());

        let gid = grp.id.clone();
        spawn(async move {
            if let Ok(msgs) = groups::groups_get_messages(&gid, 50, None).await {
                group_messages.set(msgs);
            }
            platform::scroll_chat_to_bottom(false);
        });
    });

    // Helper: select a server
    let mut select_server = move |srv: ServerItem| {
        selected_server.set(Some(srv.clone()));
        spawn(async move {
            if let Ok(chs) = api::servers_get_channels(&srv.id).await {
                let first_text = chs.iter().find(|c| c.channel_type == "text").cloned();
                channels.set(chs);
                if let Some(first) = first_text {
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
        selected_channel.set(None);
        active_group.set(None);
        active_friend.set(Some(friend.clone()));
        is_friend_typing.set(false);
        messages.set(Vec::new());
        partner_fingerprint.set(None);

        unread_counts.with_mut(|m| {
            m.remove(&friend.id);
        });

        let uid = uid_for_select.clone();
        let pkey = pkey_for_select.clone();
        spawn(async move {
            let w = platform::get_viewport_width().await;
            if w >= 1024 {
                is_partner_info_open.set(true);
            }

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
                        }),
                        Err(_) => decrypted_list.push(DisplayMessage {
                            id: wire.id,
                            seq: wire.seq,
                            sender_id: wire.sender_id,
                            recipient_id: wire.recipient_id,
                            text: "Không thể giải mã tin nhắn".to_string(),
                            created_at: wire.created_at,
                            is_self,
                            status: MessageStatus::Error,
                            reply_to_id: None,
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
                messages.set(decrypted_list);
                platform::scroll_chat_to_bottom(true);
            }
        });
    });

    // Send DM message
    let uid_for_send = current_user_id.clone();
    let pkey_for_send = preview_storage_key.clone();
    let trigger_send_message = use_callback(move |_: ()| {
        let Some(active) = active_friend() else {
            return;
        };
        let text = input_text().trim().to_string();
        let Some(keys) = user_keys() else {
            return;
        };
        if text.is_empty() || is_sending() {
            return;
        }

        if is_currently_typing() {
            is_currently_typing.set(false);
            let next_seq = typing_seq() + 1;
            typing_seq.set(next_seq);
            let aid = active.id.clone();
            spawn(async move {
                let _ = api::typing_send(&aid, false).await;
            });
        }

        is_sending.set(true);
        input_text.set(String::new());
        is_emoji_open.set(false);

        let message_id = Uuid::new_v4().to_string();
        let uid = uid_for_send.clone();
        let pkey = pkey_for_send.clone();

        spawn(async move {
            let result: Result<(i64, String), String> = async {
                let cached_pk = peer_keys_cache.read().get(&active.id).cloned();
                let peer_keys = match cached_pk {
                    Some(pk) => pk,
                    None => {
                        let pk_resp = api::keys_get(&active.id).await.map_err(|e| e.code)?;
                        let pk = get_or_verify_peer_keys(
                            &pk_resp.user_id,
                            &pk_resp.ecdh_public,
                            &pk_resp.ecdsa_public,
                            &pk_resp.fingerprint,
                        )?;
                        peer_keys_cache.write().insert(active.id.clone(), pk.clone());
                        pk
                    }
                };

                let (recip_env, sender_env) = encrypt_chat_message(
                    &message_id,
                    &uid,
                    &active.id,
                    &keys,
                    &peer_keys,
                    &text,
                )?;

                let resp = api::messages_send(&message_id, &active.id, &recip_env, &sender_env)
                    .await
                    .map_err(|e| e.code)?;
                Ok((resp.seq, resp.created_at))
            }
            .await;

            match result {
                Ok((seq, created_at)) => {
                    messages.with_mut(|list| {
                        list.push(DisplayMessage {
                            id: message_id.clone(),
                            seq,
                            sender_id: uid.clone(),
                            recipient_id: active.id.clone(),
                            text: text.clone(),
                            created_at,
                            is_self: true,
                            status: MessageStatus::Sent,
                            reply_to_id: None,
                        });
                    });
                    last_previews.with_mut(|m| {
                        m.insert(active.id.clone(), text.clone());
                        persist_previews(&pkey, m);
                    });
                    platform::scroll_chat_to_bottom(false);
                }
                Err(_) => {
                    messages.with_mut(|list| {
                        list.push(DisplayMessage {
                            id: message_id.clone(),
                            seq: 0,
                            sender_id: uid.clone(),
                            recipient_id: active.id.clone(),
                            text: "Không gửi được".to_string(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            is_self: true,
                            status: MessageStatus::Error,
                            reply_to_id: None,
                        });
                    });
                }
            }
            is_sending.set(false);
            platform::focus_chat_input();
        });
    });

    // Send channel message
    let trigger_send_channel_message = use_callback(move |_: ()| {
        let Some(ch) = selected_channel() else {
            return;
        };
        let content = channel_input().trim().to_string();
        if content.is_empty() {
            return;
        }
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
    let trigger_send_group_message = use_callback(move |_: ()| {
        let Some(grp) = active_group() else {
            return;
        };
        let text = group_input().trim().to_string();
        if text.is_empty() {
            return;
        }
        group_input.set(String::new());
        let gid = grp.id.clone();
        spawn(async move {
            if let Ok(msg) = groups::groups_send_message(&gid, &text, None).await {
                group_messages.with_mut(|list| {
                    if !list.iter().any(|m| m.id == msg.id) {
                        list.push(msg);
                    }
                });
                platform::scroll_chat_to_bottom(false);
            }
        });
    });

    // Computed counts
    let total_unread: usize = unread_counts.read().values().copied().sum();
    let pending_len = pending_requests.read().len();

    let is_chat_selected = active_friend().is_some() || active_group().is_some() || selected_channel().is_some() || is_settings_open();

    let sidebar_class = if is_chat_selected {
        "sidebar hidden-mobile"
    } else {
        "sidebar "
    };

    let chat_view_class =
        if !is_chat_selected {
            "chat-view hidden-mobile"
        } else {
            "chat-view "
        };

    let current_username_for_add = current_username.clone();

    rsx! {
        div { class: "app-layout",
            div { class: "app-content",
                // Sidebar
                aside { class: "{sidebar_class}",
                div { class: "sidebar-header",
                    div { class: "app-brand",
                        Avatar {
                            name: current_username.clone(),
                            avatar_url: my_avatar_url(),
                            size: 28,
                        }
                        span { "{current_username}" }
                    }
                    div { class: "sidebar-actions",
                        button {
                            r#type: "button",
                            class: if sidebar_tab() == SidebarTab::Chats && !is_settings_open() { "icon-btn active" } else { "icon-btn " },
                            aria_label: "Danh sách chat",
                            onclick: move |_| {
                                sidebar_tab.set(SidebarTab::Chats);
                                is_settings_open.set(false);
                                if let Some(active) = active_friend() {
                                    unread_counts.with_mut(|m| {
                                        m.remove(&active.id);
                                    });
                                }
                            },
                            LogoIcon { size: 18 }
                            if total_unread > 0 {
                                span { class: "tab-badge", "{format_badge(total_unread)}" }
                            }
                        }
                        button {
                            r#type: "button",
                            class: if sidebar_tab() == SidebarTab::Groups && !is_settings_open() { "icon-btn active" } else { "icon-btn " },
                            aria_label: "Nhóm trò chuyện",
                            onclick: move |_| {
                                sidebar_tab.set(SidebarTab::Groups);
                                is_settings_open.set(false);
                                spawn(async move {
                                    if let Ok(list) = groups::groups_list().await {
                                        groups.set(list);
                                    }
                                });
                            },
                            GroupIcon {}
                        }
                        button {
                            r#type: "button",
                            class: if sidebar_tab() == SidebarTab::Friends && !is_settings_open() { "icon-btn active" } else { "icon-btn " },
                            aria_label: "Kết bạn",
                            onclick: move |_| {
                                sidebar_tab.set(SidebarTab::Friends);
                                is_settings_open.set(false);
                            },
                            FriendsIcon {}
                            if pending_len > 0 {
                                span { class: "tab-badge", "{format_badge(pending_len)}" }
                            }
                        }
                        button {
                            r#type: "button",
                            class: if sidebar_tab() == SidebarTab::Servers && !is_settings_open() { "icon-btn active" } else { "icon-btn " },
                            aria_label: "Máy chủ",
                            onclick: move |_| {
                                sidebar_tab.set(SidebarTab::Servers);
                                is_settings_open.set(false);
                                spawn(async move {
                                    if let Ok(list) = api::servers_list().await {
                                        servers.set(list);
                                    }
                                });
                            },
                            ServerIcon {}
                        }
                        button {
                            r#type: "button",
                            class: if is_settings_open() { "icon-btn active" } else { "icon-btn " },
                            aria_label: "Cài đặt",
                            onclick: move |_| {
                                let next_open = !is_settings_open();
                                is_settings_open.set(next_open);
                                if next_open {
                                    active_friend.set(None);
                                    selected_channel.set(None);
                                }
                            },
                            SettingsIcon {}
                        }
                        button {
                            r#type: "button",
                            class: "icon-btn btn-danger",
                            aria_label: "Đăng xuất",
                            onclick: move |_| on_logout.call(()),
                            LogoutIcon {}
                        }
                    }
                }

                // Tab: Friends / Direct Add & Requests
                if sidebar_tab() == SidebarTab::Friends {
                    div { class: "add-friend-container",
                        form {
                            class: "add-friend-form",
                            onsubmit: move |evt| {
                                evt.prevent_default();
                                let target = friend_input().trim().to_string();
                                if target.is_empty() || is_adding_friend() {
                                    return;
                                }
                                if target.to_lowercase() == current_username_for_add.to_lowercase() {
                                    add_friend_status.set(Some(AddFriendStatus {
                                        is_error: true,
                                        message: "Không thể tự kết bạn".to_string(),
                                    }));
                                    return;
                                }
                                is_adding_friend.set(true);
                                add_friend_status.set(None);
                                spawn(async move {
                                    match api::friends_send_request_by_username(&target).await {
                                        Ok(_) => {
                                            friend_input.set(String::new());
                                            add_friend_status.set(Some(AddFriendStatus {
                                                is_error: false,
                                                message: "Đã gửi lời mời kết bạn".to_string(),
                                            }));
                                            if let Ok(list) = api::friends_list().await {
                                                friends.set(list);
                                            }
                                            if let Ok(reqs) = api::friends_get_requests().await {
                                                pending_requests.set(reqs);
                                            }
                                        }
                                        Err(err) => {
                                            let msg = match err.code.as_str() {
                                                "receiver_not_found" => "Không tìm thấy người dùng",
                                                "already_friends" => "Hai người đã là bạn bè",
                                                "request_already_pending" => "Đã gửi lời mời trước đó",
                                                _ => "Không thể gửi lời mời",
                                            };
                                            add_friend_status.set(Some(AddFriendStatus {
                                                is_error: true,
                                                message: msg.to_string(),
                                            }));
                                        }
                                    }
                                    is_adding_friend.set(false);
                                });
                            },
                            input {
                                r#type: "text",
                                placeholder: "Tên tài khoản",
                                aria_label: "Tên tài khoản kết bạn",
                                value: "{friend_input}",
                                oninput: move |evt| {
                                    friend_input.set(evt.value());
                                    add_friend_status.set(None);
                                },
                            }
                            button {
                                r#type: "submit",
                                class: "btn-small-primary",
                                disabled: friend_input().trim().is_empty() || is_adding_friend(),
                                "Kết bạn"
                            }
                        }
                        if let Some(st) = add_friend_status() {
                            div {
                                class: if st.is_error { "status-badge error" } else { "status-badge success" },
                                "{st.message}"
                            }
                        }
                    }

                    div { class: "sidebar-scrollable",
                        if !friend_input().trim().is_empty() {
                            if search_results.read().is_empty() {
                                div { class: "empty-chat", "Không tìm thấy người dùng" }
                            } else {
                                for user in search_results() {
                                    div { class: "request-card", key: "{user.id}",
                                        div { class: "request-user",
                                            Avatar {
                                                name: user.username.clone(),
                                                avatar_url: user.avatar_url.clone(),
                                                size: 32,
                                            }
                                            span { class: "chat-item-name", "{user.username}" }
                                        }
                                        if user.relationship == "friend" {
                                            span { class: "btn-small-disabled", "Bạn bè" }
                                        } else if user.relationship == "pending_sent" {
                                            span { class: "btn-small-disabled", "Đã gửi" }
                                        } else if user.relationship == "pending_received" {
                                            button {
                                                r#type: "button",
                                                class: "btn-small-primary",
                                                onclick: {
                                                    let uid = user.id.clone();
                                                    move |_| {
                                                        let req_id = pending_requests
                                                            .read()
                                                            .iter()
                                                            .find(|r| r.sender_id == uid)
                                                            .map(|r| r.id.clone());
                                                        if let Some(rid) = req_id {
                                                            spawn(async move {
                                                                if api::friends_accept_request(&rid).await.is_ok() {
                                                                    if let Ok(list) = api::friends_list().await {
                                                                        friends.set(list);
                                                                    }
                                                                    if let Ok(reqs) = api::friends_get_requests().await {
                                                                        pending_requests.set(reqs);
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    }
                                                },
                                                "Chấp nhận"
                                            }
                                        } else {
                                            button {
                                                r#type: "button",
                                                class: "btn-small-primary",
                                                onclick: {
                                                    let rid = user.id.clone();
                                                    move |_| {
                                                        let rid_c = rid.clone();
                                                        spawn(async move {
                                                            if api::friends_send_request_by_id(&rid_c).await.is_ok() {
                                                                search_results.with_mut(|list| {
                                                                    for u in list.iter_mut() {
                                                                        if u.id == rid_c {
                                                                            u.relationship = "pending_sent".to_string();
                                                                        }
                                                                    }
                                                                });
                                                                if let Ok(list) = api::friends_list().await {
                                                                    friends.set(list);
                                                                }
                                                                if let Ok(reqs) = api::friends_get_requests().await {
                                                                    pending_requests.set(reqs);
                                                                }
                                                            }
                                                        });
                                                    }
                                                },
                                                "Kết bạn"
                                            }
                                        }
                                    }
                                }
                            }
                        } else if pending_requests.read().is_empty() {
                            if add_friend_status().is_none() {
                                div { class: "empty-chat", "Không có lời mời nào" }
                            }
                        } else {
                            for req in pending_requests() {
                                div { class: "request-card", key: "{req.id}",
                                    div { class: "request-user",
                                        Avatar {
                                            name: req.sender_username.clone(),
                                            avatar_url: req.sender_avatar_url.clone(),
                                            size: 32,
                                        }
                                        span { class: "chat-item-name", "{req.sender_username}" }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-small-primary",
                                        onclick: {
                                            let rid = req.id.clone();
                                            move |_| {
                                                let rid_c = rid.clone();
                                                spawn(async move {
                                                    if api::friends_accept_request(&rid_c).await.is_ok() {
                                                        if let Ok(list) = api::friends_list().await {
                                                            friends.set(list);
                                                        }
                                                        if let Ok(reqs) = api::friends_get_requests().await {
                                                            pending_requests.set(reqs);
                                                        }
                                                    }
                                                });
                                            }
                                        },
                                        "Chấp nhận"
                                    }
                                }
                            }
                        }
                    }
                }

                // Tab: Chats (Default)
                if sidebar_tab() == SidebarTab::Chats {
                    div { class: "sidebar-scrollable",
                        if friends.read().is_empty() {
                            div { class: "empty-chat", "Chưa có bạn bè nào" }
                        } else {
                            for friend in friends() {
                                {
                                    let is_selected = active_friend()
                                        .as_ref()
                                        .map(|a| a.id == friend.id)
                                        .unwrap_or(false);
                                    let unread = unread_counts
                                        .read()
                                        .get(&friend.id)
                                        .copied()
                                        .unwrap_or(0);
                                    let preview = last_previews
                                        .read()
                                        .get(&friend.id)
                                        .cloned()
                                        .unwrap_or_else(|| "Bắt đầu trò chuyện".to_string());
                                    let f_click = friend.clone();
                                    rsx! {
                                        button {
                                            key: "{friend.id}",
                                            r#type: "button",
                                            class: if is_selected { "chat-item selected" } else { "chat-item " },
                                            onclick: move |_| select_friend.call(f_click.clone()),
                                            Avatar {
                                                name: friend.username.clone(),
                                                avatar_url: friend.avatar_url.clone(),
                                                size: 38,
                                            }
                                            div { class: "chat-item-info",
                                                div { class: "chat-item-header",
                                                    span {
                                                        class: if unread > 0 { "chat-item-name unread" } else { "chat-item-name " },
                                                        "{friend.username}"
                                                    }
                                                    if unread > 0 {
                                                        span { class: "chat-item-badge", "{format_badge(unread)}" }
                                                    }
                                                }
                                                div {
                                                    class: if unread > 0 { "chat-item-preview unread" } else { "chat-item-preview " },
                                                    "{preview}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Tab: Groups
                if sidebar_tab() == SidebarTab::Groups {
                    div { class: "group-tab-container",
                        div { class: "group-actions-bar",
                            button {
                                r#type: "button",
                                class: "group-create-btn",
                                style: "display: flex; align-items: center; gap: 6px; padding: 6px 12px; background: var(--msg-sent); color: white; border: none; border-radius: var(--radius-sm); font-size: 13px; font-weight: 500; cursor: pointer;",
                                onclick: move |_| is_create_group_open.set(true),
                                PlusIcon { size: 16 }
                                span { "Tạo nhóm mới" }
                            }
                        }
                        div { class: "sidebar-scrollable",
                            if groups.read().is_empty() {
                                div { class: "empty-chat", "Chưa tham gia nhóm nào" }
                            } else {
                                for grp in groups() {
                                    {
                                        let is_selected = active_group()
                                            .as_ref()
                                            .map(|g| g.id == grp.id)
                                            .unwrap_or(false);
                                        let g_click = grp.clone();
                                        rsx! {
                                            button {
                                                key: "{grp.id}",
                                                r#type: "button",
                                                class: if is_selected { "chat-item selected" } else { "chat-item " },
                                                onclick: move |_| select_group.call(g_click.clone()),
                                                Avatar {
                                                    name: grp.name.clone(),
                                                    avatar_url: grp.avatar_url.clone(),
                                                    size: 38,
                                                }
                                                div { class: "chat-item-info",
                                                    div { class: "chat-item-header",
                                                        span { class: "chat-item-name", "{grp.name}" }
                                                        if let Some(role) = grp.role.as_ref() {
                                                            span {
                                                                style: "font-size: 10px; padding: 2px 6px; border-radius: 4px; background: var(--bg-hover); color: var(--text-secondary); text-transform: uppercase; font-weight: 600;",
                                                                "{role}"
                                                            }
                                                        }
                                                    }
                                                    div { class: "chat-item-preview",
                                                        "{grp.member_count} thành viên"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Tab: Servers (Discord-style)
                if sidebar_tab() == SidebarTab::Servers {
                    div { class: "server-tab-container",
                        div { class: "server-actions-bar",
                            form {
                                style: "display: flex; gap: 6px;",
                                onsubmit: move |evt| {
                                    evt.prevent_default();
                                    let name = new_server_name().trim().to_string();
                                    if name.is_empty() {
                                        return;
                                    }
                                    spawn(async move {
                                        if let Ok(created) = api::servers_create(&name).await {
                                            new_server_name.set(String::new());
                                            if let Ok(list) = api::servers_list().await {
                                                servers.set(list);
                                            }
                                            select_server(created);
                                        }
                                    });
                                },
                                input {
                                    r#type: "text",
                                    placeholder: "Tên server mới",
                                    value: "{new_server_name}",
                                    oninput: move |evt| new_server_name.set(evt.value()),
                                }
                                button {
                                    r#type: "submit",
                                    disabled: new_server_name().trim().is_empty(),
                                    "Tạo"
                                }
                            }
                            form {
                                style: "display: flex; gap: 6px;",
                                onsubmit: move |evt| {
                                    evt.prevent_default();
                                    let code = invite_code_input().trim().to_string();
                                    if code.is_empty() {
                                        return;
                                    }
                                    spawn(async move {
                                        if let Ok(srv) = api::servers_join(&code).await {
                                            invite_code_input.set(String::new());
                                            if let Ok(list) = api::servers_list().await {
                                                servers.set(list);
                                            }
                                            select_server(srv);
                                        }
                                    });
                                },
                                input {
                                    r#type: "text",
                                    placeholder: "Mã mời tham gia",
                                    value: "{invite_code_input}",
                                    oninput: move |evt| invite_code_input.set(evt.value()),
                                }
                                button {
                                    r#type: "submit",
                                    disabled: invite_code_input().trim().is_empty(),
                                    "Vào"
                                }
                            }
                        }

                        div { class: "server-list-header", "Danh sách máy chủ" }
                        if servers.read().is_empty() {
                            div { class: "empty-chat", "Chưa tham gia server nào" }
                        } else {
                            for srv in servers() {
                                {
                                    let is_sel = selected_server()
                                        .as_ref()
                                        .map(|s| s.id == srv.id)
                                        .unwrap_or(false);
                                    let s_click = srv.clone();
                                    rsx! {
                                        button {
                                            key: "{srv.id}",
                                            r#type: "button",
                                            class: if is_sel { "server-item selected" } else { "server-item " },
                                            onclick: move |_| select_server(s_click.clone()),
                                            Avatar {
                                                name: srv.name.clone(),
                                                avatar_url: srv.icon_url.clone(),
                                                size: 32,
                                            }
                                            span { "{srv.name}" }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(sel_srv) = selected_server() {
                            div {
                                class: "server-list-header",
                                style: "margin-top: 12px;",
                                "Kênh ("
                                "{sel_srv.name}"
                                ")"
                            }
                            for ch in channels() {
                                {
                                    let is_sel = selected_channel()
                                        .as_ref()
                                        .map(|c| c.id == ch.id)
                                        .unwrap_or(false);
                                    let icon_str = if ch.channel_type == "voice" { "🔊" } else { "#" };
                                    let c_click = ch.clone();
                                    rsx! {
                                        button {
                                            key: "{ch.id}",
                                            r#type: "button",
                                            class: if is_sel { "channel-item selected" } else { "channel-item " },
                                            onclick: move |_| select_channel(c_click.clone()),
                                            span { "{icon_str}" }
                                            span { "{ch.name}" }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(vc) = active_voice_channel() {
                            div {
                                class: "voice-control-bar",
                                style: "margin-top: auto;",
                                span { "🔊 {vc.name}" }
                                div { style: "display: flex; gap: 6px;",
                                    button {
                                        r#type: "button",
                                        class: if is_voice_muted() { "voice-btn mute" } else { "voice-btn " },
                                        onclick: move |_| {
                                            if let Some(ch) = active_voice_channel() {
                                                let next = !is_voice_muted();
                                                is_voice_muted.set(next);
                                                spawn(async move {
                                                    let _ = api::webrtc_update_voice_state(&ch.id, next).await;
                                                });
                                            }
                                        },
                                        if is_voice_muted() { "Bật mic" } else { "Tắt mic" }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "voice-btn disconnect",
                                        onclick: move |_| {
                                            if let Some(ch) = active_voice_channel() {
                                                spawn(async move {
                                                    let _ = api::webrtc_leave_voice(&ch.id).await;
                                                    active_voice_channel.set(None);
                                                });
                                            }
                                        },
                                        "Rời phòng"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Main Chat View
            main { class: "{chat_view_class}",
                if let Some(active) = active_friend() {
                    div { class: "chat-main-container",
                        div { class: "chat-column",
                            // DM Header
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại",
                                    onclick: move |_| active_friend.set(None),
                                    BackIcon {}
                                }
                                Avatar {
                                    name: active.username.clone(),
                                    avatar_url: active.avatar_url.clone(),
                                    size: 34,
                                }
                                h2 { class: "chat-header-title", "{active.username}" }
                                div { class: "chat-header-spacer" }
                                {
                                    let active_uid_audio = active.id.clone();
                                    let active_uid_video = active.id.clone();
                                    rsx! {
                                        button {
                                            r#type: "button",
                                            class: "icon-btn call-action-btn",
                                            aria_label: "Gọi thoại",
                                            title: "Gọi thoại",
                                            onclick: move |_| {
                                                let target = active_uid_audio.clone();
                                                spawn(async move {
                                                    if let Ok(sess) = calls::calls_start("audio", Some(target), None).await {
                                                        active_call_session.set(Some(sess));
                                                    }
                                                });
                                            },
                                            PhoneIcon {}
                                        }
                                        button {
                                            r#type: "button",
                                            class: "icon-btn call-action-btn",
                                            aria_label: "Gọi video",
                                            title: "Gọi video",
                                            onclick: move |_| {
                                                let target = active_uid_video.clone();
                                                spawn(async move {
                                                    if let Ok(sess) = calls::calls_start("video", Some(target), None).await {
                                                        active_call_session.set(Some(sess));
                                                    }
                                                });
                                            },
                                            VideoIcon {}
                                        }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: if is_partner_info_open() { "icon-btn active" } else { "icon-btn " },
                                    aria_label: "Thông tin tài khoản",
                                    title: "Thông tin tài khoản",
                                    onclick: move |_| is_partner_info_open.set(!is_partner_info_open()),
                                    MenuIcon {}
                                }
                            }

                            // Messages — column-reverse: newest at bottom, scroll up for older
                            div {
                                id: "chat-messages-scroll",
                                class: "chat-messages",
                                if messages.read().is_empty() {
                                    div { class: "empty-chat-inner", "Chưa có tin nhắn" }
                                } else {
                                    for msg in messages().into_iter().rev() {
                                        {
                                            let row_cls = if msg.is_self { "message-row sent" } else { "message-row received" };
                                            let reply_text = msg.reply_to_id.as_ref().map(|rid| {
                                                messages
                                                    .read()
                                                    .iter()
                                                    .find(|m| &m.id == rid)
                                                    .map(|m| m.text.clone())
                                                    .unwrap_or_else(|| "Tin nhắn gốc".to_string())
                                            });
                                            let time_str = if msg.created_at.is_empty() {
                                                String::new()
                                            } else {
                                                platform::format_local_time(&msg.created_at)
                                            };
                                            rsx! {
                                                div { class: "{row_cls}", key: "{msg.id}",
                                                    div { class: "message-content-wrapper",
                                                        if !msg.is_self {
                                                            Avatar {
                                                                name: active.username.clone(),
                                                                avatar_url: active.avatar_url.clone(),
                                                                size: 28,
                                                            }
                                                        }
                                                        div { class: "message-bubble",
                                                            if let Some(rt) = reply_text {
                                                                div {
                                                                    style: "font-size: 11.5px; border-left: 2px solid currentColor; padding-left: 6px; margin-bottom: 4px; opacity: 0.75;",
                                                                    "{rt}"
                                                                }
                                                            }
                                                            "{msg.text}"
                                                        }
                                                    }
                                                    if !msg.created_at.is_empty() {
                                                        span { class: "message-time", "{time_str}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Typing Indicator
                            if is_friend_typing() {
                                div { class: "typing-indicator", "Đang nhập" }
                            }

                            // Input Bar
                            form {
                                class: "chat-input-bar",
                                onsubmit: move |evt| {
                                    evt.prevent_default();
                                    trigger_send_message.call(());
                                },
                                if is_emoji_open() {
                                    EmojiPicker {
                                        on_select: move |emoji: String| {
                                            let cur = input_text();
                                            spawn(async move {
                                                let updated = platform::insert_emoji_in_dm_input(&cur, &emoji).await;
                                                input_text.set(updated);
                                            });
                                        },
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    aria_label: "Chọn emoji",
                                    onclick: move |evt| {
                                        evt.stop_propagation();
                                        is_emoji_open.set(!is_emoji_open());
                                    },
                                    EmojiIcon {}
                                }
                                input {
                                    id: "dm-chat-input",
                                    r#type: "text",
                                    class: "chat-input",
                                    placeholder: "Nhập tin nhắn",
                                    aria_label: "Nội dung tin nhắn",
                                    value: "{input_text}",
                                    oninput: move |evt| {
                                        let val = evt.value();
                                        input_text.set(val.clone());
                                        if let Some(af) = active_friend() {
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
                                    onkeydown: move |evt| {
                                        if evt.key() == Key::Enter && !evt.modifiers().shift() {
                                            evt.prevent_default();
                                            trigger_send_message.call(());
                                        } else if evt.key() == Key::Escape {
                                            is_emoji_open.set(false);
                                        }
                                    },
                                }
                                button {
                                    r#type: "submit",
                                    class: "send-btn",
                                    disabled: is_sending() || input_text().trim().is_empty(),
                                    aria_label: "Gửi",
                                    SendIcon {}
                                }
                            }
                        }

                        // Partner Info Panel
                        if is_partner_info_open() {
                            aside { class: "partner-info-sidebar",
                                header { class: "partner-info-header",
                                    h3 { class: "partner-info-title", "Thông tin" }
                                    button {
                                        r#type: "button",
                                        class: "icon-btn",
                                        aria_label: "Đóng",
                                        onclick: move |_| is_partner_info_open.set(false),
                                        CloseIcon {}
                                    }
                                }
                                div { class: "partner-info-content",
                                    div { class: "partner-info-card",
                                        Avatar {
                                            name: active.username.clone(),
                                            avatar_url: active.avatar_url.clone(),
                                            size: 64,
                                        }
                                        div { class: "partner-info-name", "{active.username}" }
                                        div { class: "partner-info-sub", "Tài khoản bạn bè" }
                                    }

                                    div { class: "settings-group",
                                        div { class: "settings-group-title", "Tài khoản" }
                                        div { class: "settings-row",
                                            span { class: "settings-label", "Tên người dùng" }
                                            span { class: "settings-value", "{active.username}" }
                                        }
                                        div { class: "settings-row column-layout",
                                            span { class: "settings-label", "Mã định danh" }
                                            span { class: "settings-value mono", "{active.id}" }
                                        }
                                    }

                                    div { class: "settings-group",
                                        div { class: "settings-group-title", "Mã hóa đầu cuối" }
                                        div { class: "settings-row",
                                            span { class: "settings-label", "Giao thức" }
                                            span { class: "settings-value nowrap", "ECDH P-256 / AES-256-GCM" }
                                        }
                                        div { class: "settings-row",
                                            span { class: "settings-label", "Chữ ký số" }
                                            span { class: "settings-value nowrap", "ECDSA P-256 / SHA-256" }
                                        }
                                        div { class: "settings-row",
                                            span { class: "settings-label", "Trạng thái khóa" }
                                            span { class: "settings-value tag-active",
                                                if partner_fingerprint().is_some() {
                                                    "Đã kích hoạt trên thiết bị"
                                                } else {
                                                    "Đang tải"
                                                }
                                            }
                                        }
                                        if let Some(fp) = partner_fingerprint() {
                                            div { class: "settings-row column-layout",
                                                span { class: "settings-label", "Vân tay khóa" }
                                                span { class: "settings-value mono", "{fp}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(grp) = active_group() {
                    // Group View
                    div { class: "chat-main-container",
                        div { class: "chat-column",
                            // Group Header
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại",
                                    onclick: move |_| active_group.set(None),
                                    BackIcon {}
                                }
                                Avatar {
                                    name: grp.name.clone(),
                                    avatar_url: grp.avatar_url.clone(),
                                    size: 34,
                                }
                                div { class: "chat-header-info", style: "display: flex; flex-direction: column; min-width: 0;",
                                    h2 { class: "chat-header-title", style: "margin: 0; line-height: 1.2;", "{grp.name}" }
                                    span { style: "font-size: 11px; color: var(--text-secondary);", "{grp.member_count} thành viên" }
                                }
                                div { class: "chat-header-spacer" }
                                {
                                    let g_id_audio = grp.id.clone();
                                    let g_id_video = grp.id.clone();
                                    rsx! {
                                        button {
                                            r#type: "button",
                                            class: "icon-btn call-action-btn",
                                            aria_label: "Gọi thoại nhóm",
                                            title: "Gọi thoại nhóm",
                                            onclick: move |_| {
                                                let target = g_id_audio.clone();
                                                spawn(async move {
                                                    if let Ok(sess) = calls::calls_start("audio", None, Some(target)).await {
                                                        active_call_session.set(Some(sess));
                                                    }
                                                });
                                            },
                                            PhoneIcon {}
                                        }
                                        button {
                                            r#type: "button",
                                            class: "icon-btn call-action-btn",
                                            aria_label: "Gọi video nhóm",
                                            title: "Gọi video nhóm",
                                            onclick: move |_| {
                                                let target = g_id_video.clone();
                                                spawn(async move {
                                                    if let Ok(sess) = calls::calls_start("video", None, Some(target)).await {
                                                        active_call_session.set(Some(sess));
                                                    }
                                                });
                                            },
                                            VideoIcon {}
                                        }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: if is_group_info_open() { "icon-btn active" } else { "icon-btn " },
                                    aria_label: "Thông tin nhóm",
                                    title: "Thông tin nhóm",
                                    onclick: move |_| is_group_info_open.set(!is_group_info_open()),
                                    InfoIcon { size: 18 }
                                }
                            }

                            // Group Messages List
                            div {
                                id: "chat-messages-scroll",
                                class: "chat-messages",
                                style: "overflow-y: auto; display: flex; flex-direction: column;",
                                if group_messages.read().is_empty() {
                                    div { class: "empty-chat-inner", "Chưa có tin nhắn trong nhóm này" }
                                } else {
                                    for msg in group_messages() {
                                        {
                                            let is_self = msg.sender_id == current_user_id;
                                            let row_cls = if is_self { "message-row sent" } else { "message-row received" };
                                            let time_str = platform::format_local_time(&msg.created_at);
                                            rsx! {
                                                div { class: "{row_cls}", key: "{msg.id}",
                                                    div { class: "message-content-wrapper",
                                                        if !is_self {
                                                            Avatar {
                                                                name: msg.sender_username.clone(),
                                                                avatar_url: msg.sender_avatar_url.clone(),
                                                                size: 28,
                                                            }
                                                        }
                                                        div { class: "message-bubble",
                                                            if !is_self {
                                                                div {
                                                                    style: "font-size: 11px; font-weight: 600; margin-bottom: 2px; opacity: 0.85;",
                                                                    "{msg.sender_username}"
                                                                }
                                                            }
                                                            "{msg.content}"
                                                        }
                                                    }
                                                    if !msg.created_at.is_empty() {
                                                        span { class: "message-time", "{time_str}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Group Input Bar
                            form {
                                class: "chat-input-bar",
                                onsubmit: move |evt| {
                                    evt.prevent_default();
                                    trigger_send_group_message.call(());
                                },
                                input {
                                    r#type: "text",
                                    class: "chat-input",
                                    placeholder: "Nhắn tin tại {grp.name}",
                                    aria_label: "Nội dung tin nhắn nhóm",
                                    value: "{group_input}",
                                    oninput: move |evt| group_input.set(evt.value()),
                                    onkeydown: move |evt| {
                                        if evt.key() == Key::Enter && !evt.modifiers().shift() {
                                            evt.prevent_default();
                                            trigger_send_group_message.call(());
                                        }
                                    },
                                }
                                button {
                                    r#type: "submit",
                                    class: "send-btn",
                                    disabled: group_input().trim().is_empty(),
                                    aria_label: "Gửi",
                                    SendIcon {}
                                }
                            }
                        }

                        // Group Info Sidebar (if opened)
                        if is_group_info_open() {
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
                } else if let Some(ch) = selected_channel() {
                    // Channel View
                    div { class: "chat-main-container",
                        div { class: "chat-column",
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại",
                                    onclick: move |_| selected_channel.set(None),
                                    BackIcon {}
                                }
                                div {
                                    style: "font-size: 17px; font-weight: 600;",
                                    if ch.channel_type == "voice" { "🔊 {ch.name}" } else { "# {ch.name}" }
                                }
                                div { class: "chat-header-spacer" }
                                if let Some(srv) = selected_server() {
                                    span {
                                        style: "font-size: 12px; color: var(--text-muted);",
                                        "{srv.name}"
                                    }
                                }
                            }

                            if ch.channel_type == "text" {
                                // Channel Messages List
                                div {
                                    class: "chat-messages",
                                    style: "overflow-y: auto; display: flex; flex-direction: column;",
                                    if channel_messages.read().is_empty() {
                                        div { class: "empty-chat-inner", "Chưa có tin nhắn trong kênh này" }
                                    } else {
                                        for msg in channel_messages() {
                                            {
                                                let is_self = msg.sender_id == current_user_id;
                                                let row_cls = if is_self { "message-row sent" } else { "message-row received" };
                                                let time_str = platform::format_local_time(&msg.created_at);
                                                rsx! {
                                                    div { class: "{row_cls}", key: "{msg.id}",
                                                        div { class: "message-content-wrapper",
                                                            if !is_self {
                                                                Avatar {
                                                                    name: msg.sender_username.clone(),
                                                                    avatar_url: msg.sender_avatar_url.clone(),
                                                                    size: 28,
                                                                }
                                                            }
                                                            div { class: "message-bubble",
                                                                if !is_self {
                                                                    div {
                                                                        style: "font-size: 11px; font-weight: 600; margin-bottom: 2px; opacity: 0.85;",
                                                                        "{msg.sender_username}"
                                                                    }
                                                                }
                                                                "{msg.content}"
                                                            }
                                                        }
                                                        if !msg.created_at.is_empty() {
                                                            span { class: "message-time", "{time_str}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Channel Input Bar
                                form {
                                    class: "chat-input-bar",
                                    onsubmit: move |evt| {
                                        evt.prevent_default();
                                        trigger_send_channel_message.call(());
                                    },
                                    input {
                                        r#type: "text",
                                        class: "chat-input",
                                        placeholder: "Nhắn tin tại #{ch.name}",
                                        aria_label: "Nội dung tin nhắn kênh",
                                        value: "{channel_input}",
                                        oninput: move |evt| channel_input.set(evt.value()),
                                        onkeydown: move |evt| {
                                            if evt.key() == Key::Enter && !evt.modifiers().shift() {
                                                evt.prevent_default();
                                                trigger_send_channel_message.call(());
                                            }
                                        },
                                    }
                                    button {
                                        r#type: "submit",
                                        class: "send-btn",
                                        disabled: channel_input().trim().is_empty(),
                                        aria_label: "Gửi",
                                        SendIcon {}
                                    }
                                }
                            } else {
                                // Voice Channel Room View
                                div {
                                    class: "empty-chat",
                                    style: "flex-direction: column; gap: 12px;",
                                    div { style: "font-size: 42px;", "🔊" }
                                    div { style: "font-size: 18px; font-weight: 600;", "Kênh thoại: {ch.name}" }
                                    div {
                                        style: "font-size: 13px; color: var(--text-muted);",
                                        if active_voice_channel().as_ref().map(|v| v.id == ch.id).unwrap_or(false) {
                                            "Đang kết nối phòng thoại WebRTC"
                                        } else {
                                            "Nhấn vào kênh để kết nối"
                                        }
                                    }
                                    if active_voice_channel().as_ref().map(|v| v.id == ch.id).unwrap_or(false) {
                                        div { style: "display: flex; gap: 10px; margin-top: 8px;",
                                            button {
                                                r#type: "button",
                                                class: if is_voice_muted() { "voice-btn mute" } else { "voice-btn " },
                                                onclick: move |_| {
                                                    if let Some(vc) = active_voice_channel() {
                                                        let next = !is_voice_muted();
                                                        is_voice_muted.set(next);
                                                        spawn(async move {
                                                            let _ = api::webrtc_update_voice_state(&vc.id, next).await;
                                                        });
                                                    }
                                                },
                                                if is_voice_muted() { "Bật mic" } else { "Tắt mic (Mute)" }
                                            }
                                            button {
                                                r#type: "button",
                                                class: "voice-btn disconnect",
                                                onclick: move |_| {
                                                    if let Some(vc) = active_voice_channel() {
                                                        spawn(async move {
                                                            let _ = api::webrtc_leave_voice(&vc.id).await;
                                                            active_voice_channel.set(None);
                                                        });
                                                    }
                                                },
                                                "Rời phòng"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if is_settings_open() {
                    // Settings View
                    header { class: "chat-header",
                        button {
                            r#type: "button",
                            class: "icon-btn",
                            aria_label: "Đóng cài đặt",
                            onclick: move |_| is_settings_open.set(false),
                            BackIcon {}
                        }
                        h2 { class: "chat-header-title", "Cài đặt" }
                    }
                    div { class: "settings-content",
                        div { class: "settings-profile-card",
                            div { class: "settings-avatar-wrapper",
                                Avatar {
                                    name: current_username.clone(),
                                    avatar_url: my_avatar_url(),
                                    size: 56,
                                }
                            }
                            div { class: "settings-profile-info",
                                div { class: "settings-profile-name", "{current_username}" }
                                div { class: "settings-profile-sub", "Tài khoản cá nhân" }
                            }
                            div { class: "settings-profile-action",
                                label { class: "btn-avatar-upload",
                                    input {
                                        id: "avatar-file-input",
                                        r#type: "file",
                                        accept: "image/*",
                                        disabled: is_uploading_avatar(),
                                        style: "display: none;",
                                        onchange: move |_| {
                                            is_uploading_avatar.set(true);
                                            avatar_upload_error.set(None);
                                            spawn(async move {
                                                match platform::process_selected_avatar_bytes().await {
                                                    Ok(bytes) => match api::users_upload_avatar(bytes).await {
                                                        Ok(resp) => {
                                                            my_avatar_url.set(Some(resp.avatar_url.clone()));
                                                            on_avatar_updated.call(resp.avatar_url);
                                                        }
                                                        Err(_) => {
                                                            avatar_upload_error.set(Some(
                                                                "Không thể cập nhật ảnh đại diện".to_string(),
                                                            ));
                                                        }
                                                    },
                                                    Err(err) if err == "no_file" => {}
                                                    Err(_) => {
                                                        avatar_upload_error.set(Some(
                                                            "Không thể cập nhật ảnh đại diện".to_string(),
                                                        ));
                                                    }
                                                }
                                                is_uploading_avatar.set(false);
                                            });
                                        },
                                    }
                                    if is_uploading_avatar() { "Đang tải" } else { "Đổi ảnh đại diện" }
                                }
                                if let Some(err) = avatar_upload_error() {
                                    span { class: "error-text", "{err}" }
                                }
                            }
                        }

                        div { class: "settings-group",
                            div { class: "settings-group-title", "Tài khoản" }
                            div { class: "settings-row",
                                span { class: "settings-label", "Tên người dùng" }
                                span { class: "settings-value", "{current_username}" }
                            }
                            div { class: "settings-row column-layout",
                                span { class: "settings-label", "Mã định danh" }
                                span { class: "settings-value mono", "{current_user_id}" }
                            }
                        }

                        div { class: "settings-group",
                            div { class: "settings-group-title", "Mã hóa đầu cuối" }
                            div { class: "settings-row",
                                span { class: "settings-label", "Giao thức" }
                                span { class: "settings-value nowrap", "ECDH P-256 / AES-256-GCM" }
                            }
                            div { class: "settings-row",
                                span { class: "settings-label", "Chữ ký số" }
                                span { class: "settings-value nowrap", "ECDSA P-256 / SHA-256" }
                            }
                            div { class: "settings-row",
                                span { class: "settings-label", "Trạng thái khóa" }
                                span { class: "settings-value tag-active",
                                    if user_keys().is_some() {
                                        "Đã kích hoạt trên thiết bị"
                                    } else {
                                        "Đang tải"
                                    }
                                }
                            }
                            if let Some(keys) = user_keys() {
                                div { class: "settings-row column-layout",
                                    span { class: "settings-label", "Vân tay khóa" }
                                    span { class: "settings-value mono", "{keys.fingerprint_b64}" }
                                }
                            }
                        }

                        div { class: "settings-group",
                            div { class: "settings-group-title", "Phiên đăng nhập" }
                            button {
                                r#type: "button",
                                class: "btn-danger-outline",
                                onclick: move |_| on_logout.call(()),
                                "Đăng xuất"
                            }
                        }
                    }
                } else {
                    div { class: "empty-chat",
                        span { "Chọn một cuộc trò chuyện để bắt đầu" }
                    }
                }
            }

            // Modals
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
                    GroupInviteModal {
                        group_id: grp.id.clone(),
                        on_close: move |_| is_group_invite_open.set(false),
                    }
                }
            }

            if is_group_add_member_open() {
                if let Some(grp) = active_group() {
                    AddMemberModal {
                        group_id: grp.id.clone(),
                        on_close: move |_| is_group_add_member_open.set(false),
                        on_member_added: move |_| {
                            is_group_add_member_open.set(false);
                        },
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
        }
    }
}
}
