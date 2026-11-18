use dioxus::prelude::*;
use futures_util::StreamExt;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    components::{
        avatar::Avatar,
        calls::{ActiveCallOverlay, IncomingCallModal},
        details_panel::DmDetailsPanel,
        groups::{AddMemberModal, CreateGroupModal, GroupInfoPanel, GroupInviteModal},
        icons::{
            BackIcon, GroupIcon, HashIcon, MessageIcon, MicIcon, MicOffIcon, PanelRightIcon,
            PhoneIcon, PhoneOffIcon, ShieldIcon, UserPlusIcon, VideoIcon, VolumeIcon,
        },
        messages::{Composer, MessageList, ViewMessage},
        navigation::{
            modals::{CommunityModal, CreateCategoryModal, CreateChannelModal, ServerInviteModal},
            rail::NavigationRail,
            sidebar::NavigationSidebar,
            NavSection,
        },
        profile_modal::ProfileModal,
        settings_view::SettingsView,
    },
    crypto::e2ee::{
        PeerKeys, decrypt_chat_message, encrypt_chat_message, get_or_verify_peer_keys,
        get_user_keys,
    },
    media,
    models::{
        CallSessionItem, ChannelItem, ChannelMessageItem, DisplayMessage, FriendItem,
        GroupCategoryItem, GroupChannelItem, GroupItem, GroupMessageItem, GroupReactionItem,
        MessageStatus, PendingRequestItem, ServerCategoryItem, ServerItem, UserSearchResult,
        VoiceStateItem, WsEvent,
    },
    platform,
    services::{api, calls, groups, ws},
};

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
                                        && (nav_section() != NavSection::DirectMessages
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
                                if is_for_active && !messages.read().iter().any(|m| m.id == id) {
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
        nav_section.set(NavSection::DirectMessages);
        group_reply_to.set(None);
        selected_channel.set(None);
        active_friend.set(None);
        active_group.set(Some(grp.clone()));
        group_messages.set(Vec::new());

        let gid = grp.id.clone();
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
            platform::scroll_chat_to_bottom(false);
        });
    });

    // Helper: select a server
    let mut select_server = move |srv: ServerItem| {
        is_settings_open.set(false);
        nav_section.set(NavSection::Server(srv.id.clone()));
        selected_server.set(Some(srv.clone()));
        active_friend.set(None);
        active_group.set(None);
        selected_channel.set(None);
        channel_messages.set(Vec::new());
        server_categories.set(Vec::new());
        channels.set(Vec::new());
        spawn(async move {
            if let Ok(cats) = api::servers_get_categories(&srv.id).await {
                server_categories.set(cats);
            }
            if let Ok(chs) = api::servers_get_channels(&srv.id).await {
                let first_text = chs.iter().find(|c| c.channel_type == "text").cloned();
                channels.set(chs);
                // On wide screens open the first text channel; on mobile keep the channel list visible.
                if platform::get_viewport_width().await >= 768
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
        nav_section.set(NavSection::DirectMessages);
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
                        if list.iter().any(|m| m.id == message_id) {
                            return;
                        }
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

    // ---------------------------------------------------------------------
    // Derived view state
    // ---------------------------------------------------------------------
    let total_unread: usize = unread_counts.read().values().copied().sum();
    let pending_len = pending_requests.read().len();
    let is_chat_selected = active_friend().is_some()
        || active_group().is_some()
        || selected_channel().is_some()
        || is_settings_open();
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

    // DM messages → view model
    let dm_view: Vec<ViewMessage> = match active_friend() {
        Some(af) => {
            let list = messages.read();
            list.iter()
                .map(|m| ViewMessage {
                    id: m.id.clone(),
                    sender_id: m.sender_id.clone(),
                    sender_name: if m.is_self { current_username.clone() } else { af.username.clone() },
                    sender_avatar: if m.is_self { my_avatar_url() } else { af.avatar_url.clone() },
                    text: m.text.clone(),
                    created_at: m.created_at.clone(),
                    is_self: m.is_self,
                    is_error: m.status == MessageStatus::Error,
                    is_pending: false,
                    reply_text: m.reply_to_id.as_ref().map(|rid| {
                        list.iter().find(|x| &x.id == rid).map(|x| x.text.clone()).unwrap_or_else(|| "Tin nhắn gốc không còn".to_string())
                    }),
                    reactions: Vec::new(),
                })
                .collect()
        }
        None => Vec::new(),
    };

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
                }
            })
            .collect()
    } else {
        Vec::new()
    };

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
        })
        .collect();

    let my_fingerprint = user_keys().map(|k| k.fingerprint_b64);
    let feedback = add_friend_status().map(|s| (s.is_error, s.message));

    rsx! {
        div { class: "workspace-shell",
            NavigationRail {
                active_section: section.clone(),
                servers: servers(),
                unread_dm_count: total_unread,
                pending_friend_count: pending_len,
                current_username: current_username.clone(),
                current_avatar_url: my_avatar_url(),
                hidden_mobile: is_chat_selected,
                on_select_dm: move |_| {
                    is_settings_open.set(false);
                    nav_section.set(NavSection::DirectMessages);
                },
                on_select_friends: move |_| {
                    is_settings_open.set(false);
                    nav_section.set(NavSection::Friends);
                    active_friend.set(None);
                    active_group.set(None);
                    selected_channel.set(None);
                },
                on_select_server: move |srv: ServerItem| select_server(srv),
                on_open_create_server: move |_| is_community_modal_open.set(true),
                on_open_settings: move |_| is_settings_open.set(true),
                on_open_profile: {
                    let me = current_user_id.clone();
                    move |_| viewing_profile_user_id.set(Some(me.clone()))
                },
                on_logout: move |_| on_logout.call(()),
            }

            if section != NavSection::Settings {
                NavigationSidebar {
                    active_section: section.clone(),
                    current_username: current_username.clone(),
                    current_avatar_url: my_avatar_url(),
                    hidden_mobile: is_chat_selected,
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
                    selected_channel_id: selected_channel().map(|c| c.id),
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

            main { class: if is_chat_selected { "main-stage" } else { "main-stage hidden-mobile" },
                if is_settings_open() {
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
                        on_logout: move |_| on_logout.call(()),
                    }
                } else if let Some(active) = active_friend() {
                    div { class: "chat-main-container",
                        div { class: "chat-column",
                            header { class: "chat-header",
                                button {
                                    r#type: "button",
                                    class: "icon-btn back-btn",
                                    aria_label: "Quay lại danh sách",
                                    onclick: move |_| active_friend.set(None),
                                    BackIcon {}
                                }
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
                                        if is_friend_typing() {
                                            span { class: "chat-header-meta", style: "color: var(--accent-text);", "đang nhập" }
                                        } else {
                                            span { class: "chat-header-meta secure", ShieldIcon { size: 12 } "Mã hóa đầu cuối" }
                                        }
                                    }
                                }
                                div { class: "chat-header-actions",
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
                            }

                            MessageList {
                                messages: dm_view.clone(),
                                e2ee: true,
                                intro_title: active.username.clone(),
                                intro_sub: format!("Đây là khởi đầu cuộc trò chuyện trực tiếp giữa bạn và {}.", active.username),
                                highlight_id: highlight_msg(),
                                on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                            }

                            div { class: "typing-indicator", aria_live: "polite",
                                if is_friend_typing() {
                                    span { class: "typing-dots", span {} span {} span {} }
                                    span { "{active.username} đang nhập" }
                                }
                            }

                            Composer {
                                input_id: "dm-chat-input".to_string(),
                                value: input_text(),
                                placeholder: "Nhắn tin tới {active.username}",
                                sending: is_sending(),
                                disabled: user_keys().is_none(),
                                on_input: move |val: String| {
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
                                on_send: move |_| trigger_send_message.call(()),
                            }
                        }

                        if is_partner_info_open() {
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
                        div { class: "chat-column",
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
                                }
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
                                                    "# {ch.name}"
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
                                    group_reply_to.set(Some((m.id.clone(), format!("{}: {}", m.sender_name, m.text))));
                                    let _ = dioxus::document::eval("const el = document.getElementById('group-chat-input'); if (el) el.focus();");
                                },
                            }

                            Composer {
                                input_id: "group-chat-input".to_string(),
                                value: group_input(),
                                placeholder: "Nhắn tin tới {grp.name}",
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
                } else if let Some(ch) = selected_channel() {
                    div { class: "chat-main-container",
                        div { class: "chat-column",
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
                                        h2 { class: "chat-header-title", "{ch.name}" }
                                        if let Some(srv) = selected_server() {
                                            span { class: "chat-header-meta", "{srv.name}" }
                                        }
                                    }
                                }
                            }

                            if ch.channel_type != "voice" {
                                MessageList {
                                    messages: channel_view.clone(),
                                    show_sender_names: true,
                                    intro_title: format!("Chào mừng đến #{}", ch.name),
                                    intro_sub: "Đây là khởi đầu của kênh. Tin nhắn kênh cộng đồng được máy chủ lưu ở dạng thường, chưa mã hóa đầu cuối.".to_string(),
                                    on_open_profile: move |uid: String| viewing_profile_user_id.set(Some(uid)),
                                }
                                Composer {
                                    input_id: "channel-chat-input".to_string(),
                                    value: channel_input(),
                                    placeholder: "Nhắn tin tới #{ch.name}",
                                    on_input: move |v: String| channel_input.set(v),
                                    on_send: move |_| trigger_send_channel_message.call(()),
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
                                                div { class: "voice-room-title", "{ch.name}" }
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
                    }
                } else {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", MessageIcon { size: 28 } }
                        if matches!(section, NavSection::Server(_)) {
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
                ServerInviteModal {
                    server_id: srv.id.clone(),
                    server_name: srv.name.clone(),
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

        if let Some(uid) = viewing_profile_user_id() {
            {
                let is_me = uid == current_user_id;
                rsx! {
                    ProfileModal {
                        user_id: uid.clone(),
                        is_self: is_me,
                        on_close: move |_| viewing_profile_user_id.set(None),
                        on_profile_updated: move |p: crate::models::UserProfileResponse| {
                            if let Some(av) = p.avatar_url {
                                my_avatar_url.set(Some(av));
                            }
                        },
                    }
                }
            }
        }
    }
}
