use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{
            CheckIcon, CopyIcon, MicIcon, MicOffIcon, MonitorIcon, PhoneOffIcon, VideoIcon,
            VideoOffIcon,
        },
    },
    media::{
        SfuUiMediaState, attach_local_stream_to_element, attach_remote_stream_to_element,
        init_webrtc_manager, poll_sfu_ui_state, start_local_media, toggle_camera_async, toggle_mic,
        toggle_screen_share,
    },
    models::{CallParticipantItem, CallSessionItem},
    platform,
    services::calls::calls_update_state,
};

#[component]
pub fn ActiveCallOverlay(
    session: CallSessionItem,
    current_user_id: String,
    on_end_call: EventHandler<()>,
) -> Element {
    let mut is_muted = use_signal(|| false);
    let mut is_camera_off = use_signal(|| session.call_type == "audio");
    let mut is_screen_sharing = use_signal(|| false);
    let mut has_local_cam = use_signal(|| false);
    let mut remote_screen_user_id = use_signal(|| Option::<String>::None);
    let mut remote_cam_user_ids = use_signal(Vec::<String>::new);
    let mut ice_state = use_signal(|| "connecting".to_string());
    let mut copied_link = use_signal(|| false);
    let mut origin = use_signal(|| "http://127.0.0.1:8085".to_string());
    let mut active_speaker_id = use_signal(|| Option::<String>::None);
    let call_seconds = use_signal(|| 0u64);
    let mut sfu_booted_for_id = use_signal(String::new);

    let is_group = session.group_id.is_some();
    let call_id = session.id.clone();

    // Ensure SFU WebRTC session is initialized cleanly for ANY entry point (DM, Group, Meet, Accept)
    {
        let cid_init = call_id.clone();
        let me_init = current_user_id.clone();
        let want_video = session.call_type == "video";
        use_effect(move || {
            if sfu_booted_for_id() != cid_init {
                sfu_booted_for_id.set(cid_init.clone());
                let cid_run = cid_init.clone();
                let me_run = me_init.clone();
                spawn(async move {
                    // Only initialize if not already initialized for this callId
                    let mut eval = dioxus::document::eval(&format!(
                        "dioxus.send(Boolean(window.__chatwWebRTC && window.__chatwWebRTC.callId === {} && !window.__chatwWebRTC.disposed));",
                        serde_json::to_string(&cid_run).unwrap_or_default()
                    ));
                    let already = eval.recv::<bool>().await.unwrap_or(false);
                    if !already {
                        init_webrtc_manager(&cid_run, want_video, &me_run);
                        let _ = start_local_media(want_video).await;
                    }
                });
            }
        });
    }

    use_effect(move || {
        spawn(async move {
            let org = platform::app_origin().await;
            if !org.is_empty() {
                origin.set(org);
            }
        });
    });

    // Call duration timer
    use_effect(move || {
        let mut sec = call_seconds;
        spawn(async move {
            loop {
                crate::platform::sleep_ms(1000).await;
                sec.with_mut(|s| *s += 1);
            }
        });
    });

    // Continuous SFU UI sync + stream element re-attachment + active speaker polling
    use_effect(move || {
        spawn(async move {
            loop {
                crate::platform::sleep_ms(220).await;
                let st: SfuUiMediaState = poll_sfu_ui_state().await;
                if is_screen_sharing() != st.local_screen {
                    is_screen_sharing.set(st.local_screen);
                }
                if has_local_cam() != st.local_cam {
                    has_local_cam.set(st.local_cam);
                }
                if remote_screen_user_id() != st.remote_screen_user_id {
                    remote_screen_user_id.set(st.remote_screen_user_id);
                }
                if remote_cam_user_ids() != st.remote_cam_user_ids {
                    remote_cam_user_ids.set(st.remote_cam_user_ids);
                }
                if ice_state() != st.ice_state && !st.ice_state.is_empty() {
                    ice_state.set(st.ice_state);
                }
                if active_speaker_id() != st.active_speaker {
                    active_speaker_id.set(st.active_speaker);
                }
            }
        });
    });

    // Ensure local stream is attached on updates
    use_effect(move || {
        let _ = (is_screen_sharing(), has_local_cam(), is_camera_off());
        spawn(async move {
            crate::platform::sleep_ms(50).await;
            attach_local_stream_to_element("webrtc-local-video");
            attach_local_stream_to_element("webrtc-local-screen-video");
        });
    });

    let peers: Vec<CallParticipantItem> = session
        .participants
        .iter()
        .filter(|p| p.user_id != current_user_id && p.left_at.is_none())
        .cloned()
        .collect();
    let total_participants = peers.len() + 1;

    use_effect({
        let peers = peers.clone();
        move || {
            let peers = peers.clone();
            let r_scr = remote_screen_user_id();
            let _ = remote_cam_user_ids();
            spawn(async move {
                crate::platform::sleep_ms(60).await;
                if let Some(ref presenter_uid) = r_scr {
                    attach_remote_stream_to_element("webrtc-remote-screen-video", presenter_uid);
                }
                for p in peers {
                    let vid_id = format!("webrtc-remote-video-{}", p.user_id);
                    let aud_id = format!("webrtc-remote-audio-{}", p.user_id);
                    attach_remote_stream_to_element(&vid_id, &p.user_id);
                    attach_remote_stream_to_element(&aud_id, &p.user_id);
                }
            });
        }
    });

    let format_duration = |sec: u64| {
        let mins = sec / 60;
        let s = sec % 60;
        format!("{:02}:{:02}", mins, s)
    };

    let handle_toggle_mic = {
        let cid = call_id.clone();
        move |_| {
            let new_muted = !is_muted();
            is_muted.set(new_muted);
            toggle_mic(new_muted);
            let cid = cid.clone();
            spawn(async move {
                let _ = calls_update_state(&cid, Some(new_muted), None).await;
            });
        }
    };

    let handle_toggle_camera = {
        let cid = call_id.clone();
        move |_| {
            let want_off = !is_camera_off();
            is_camera_off.set(want_off);
            let cid = cid.clone();
            spawn(async move {
                let actual_off = toggle_camera_async(want_off).await;
                is_camera_off.set(actual_off);
                let _ = calls_update_state(&cid, None, Some(actual_off)).await;
            });
        }
    };

    let handle_toggle_screen = move |_| {
        let want = !is_screen_sharing();
        spawn(async move {
            let active = toggle_screen_share(want).await;
            is_screen_sharing.set(active);
            if active {
                crate::platform::sleep_ms(60).await;
                attach_local_stream_to_element("webrtc-local-screen-video");
            }
        });
    };

    let call_title = if let Some(t) = session.meet_title.as_ref().filter(|s| !s.trim().is_empty()) {
        t.clone()
    } else if is_group {
        session.group_name.clone().unwrap_or_else(|| "Cuộc gọi nhóm".to_string())
    } else if session.caller_id == current_user_id {
        session.receiver_name.clone().unwrap_or_else(|| "Cuộc gọi".to_string())
    } else {
        session.caller_name.clone()
    };

    let meet_code_opt = session.meet_code.clone();
    let remote_presenter_name = remote_screen_user_id().and_then(|uid| {
        peers
            .iter()
            .find(|p| p.user_id == uid)
            .map(|p| p.username.clone())
    });
    let show_stage = is_screen_sharing() || remote_screen_user_id().is_some();
    let sfu_connected = matches!(ice_state().as_str(), "connected" | "completed");
    let show_local_cam = has_local_cam() && !is_camera_off();
    let active_remote_cams = remote_cam_user_ids();

    rsx! {
        div {
            class: "settings-modal-backdrop active-call-overlay",
            style: "background: #0B0F17; z-index: 9999; display: flex; flex-direction: column; padding: 16px; box-sizing: border-box;",
            // Top Header
            div { style: "display: flex; align-items: center; justify-content: space-between; width: 100%; max-width: 1280px; margin: 0 auto 12px auto; flex-wrap: wrap; gap: 12px;",
                div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                    div { style: "background: rgba(255, 255, 255, 0.1); padding: 6px 12px; border-radius: 20px; font-size: 13px; font-weight: 500; color: #10B981; display: flex; align-items: center; gap: 6px;",
                        span { style: "width: 8px; height: 8px; border-radius: 50%; background: #10B981; display: inline-block;" }
                        "{format_duration(call_seconds())}"
                    }
                    span { style: "font-weight: 600; font-size: 16px; color: #F8FAFC;", "{call_title}" }
                    span { style: "background: rgba(255, 255, 255, 0.08); padding: 4px 10px; border-radius: 12px; font-size: 12px; color: #94A3B8;",
                        "{total_participants} người tham gia"
                    }
                    span {
                        class: "sfu-status-pill",
                        style: if sfu_connected {
                            "background: rgba(16, 185, 129, 0.15); color: #34D399; padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 500;"
                        } else {
                            "background: rgba(59, 130, 246, 0.15); color: #93C5FD; padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 500;"
                        },
                        if sfu_connected { "Máy chủ SFU · Đã kết nối" } else { "Máy chủ SFU · Đang kết nối" }
                    }
                }
                if let Some(code) = meet_code_opt {
                    {
                        let c_copy = code.clone();
                        rsx! {
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                span {
                                    class: "call-meet-code",
                                    style: "background: rgba(255, 255, 255, 0.08); color: #E2E8F0; padding: 6px 12px; border-radius: 8px; font-size: 13px; font-family: monospace;",
                                    "Mã: {code}"
                                }
                                button {
                                    r#type: "button",
                                    class: "call-copy-link-btn",
                                    style: "background: #2563EB; color: #FFFFFF; border: none; padding: 6px 14px; border-radius: 8px; font-size: 13px; font-weight: 500; display: inline-flex; align-items: center; gap: 6px; cursor: pointer;",
                                    onclick: move |_| {
                                        let link = format!("{}/?meet={}", origin().trim_end_matches('/'), c_copy);
                                        platform::copy_to_clipboard(&link);
                                        copied_link.set(true);
                                        spawn(async move {
                                            platform::sleep_ms(2000).await;
                                            copied_link.set(false);
                                        });
                                    },
                                    if copied_link() {
                                        CheckIcon { size: 15 }
                                        "Đã sao chép link"
                                    } else {
                                        CopyIcon { size: 15 }
                                        "Sao chép link mời"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Main Stage Area: Google Meet / Zoom Presentation Stage + Participant Strip
            div {
                style: "flex: 1; min-height: 0; width: 100%; max-width: 1280px; margin: 0 auto; display: flex; flex-direction: column; gap: 12px; overflow: hidden; border-radius: 16px; background: #131B2E; padding: 12px; box-sizing: border-box;",

                // Screen Share Stage (shown whenever anyone presents)
                if show_stage {
                    div {
                        class: "call-presentation-stage",
                        style: "flex: 1; min-height: 240px; width: 100%; display: flex; gap: 12px; overflow: hidden;",
                        if remote_screen_user_id().is_some() {
                            div {
                                style: "flex: 1; min-width: 0; position: relative; background: #0B0F17; border-radius: 12px; overflow: hidden; display: flex; align-items: center; justify-content: center;",
                                video {
                                    id: "webrtc-remote-screen-video",
                                    autoplay: true,
                                    playsinline: true,
                                    style: "width: 100%; height: 100%; object-fit: contain; display: block;"
                                }
                                div { style: "position: absolute; top: 10px; left: 10px; background: rgba(15, 23, 42, 0.78); color: #F8FAFC; padding: 4px 10px; border-radius: 8px; font-size: 12px; font-weight: 500; display: inline-flex; align-items: center; gap: 6px;",
                                    MonitorIcon { size: 14 }
                                    if let Some(ref pname) = remote_presenter_name {
                                        "{pname} đang chia sẻ màn hình"
                                    } else {
                                        "Màn hình đang được chia sẻ"
                                    }
                                }
                            }
                        }
                        if is_screen_sharing() {
                            div {
                                style: "flex: 1; min-width: 0; position: relative; background: #0B0F17; border-radius: 12px; overflow: hidden; display: flex; align-items: center; justify-content: center;",
                                video {
                                    id: "webrtc-local-screen-video",
                                    autoplay: true,
                                    playsinline: true,
                                    muted: true,
                                    style: "width: 100%; height: 100%; object-fit: contain; display: block;"
                                }
                                div { style: "position: absolute; top: 10px; left: 10px; background: rgba(37, 99, 235, 0.88); color: #FFFFFF; padding: 4px 10px; border-radius: 8px; font-size: 12px; font-weight: 500; display: inline-flex; align-items: center; gap: 6px;",
                                    MonitorIcon { size: 14 }
                                    "Bạn đang chia sẻ màn hình"
                                }
                            }
                        }
                    }
                }

                // Participant Tiles Grid (full area when no screen share, compact strip at bottom when presenting)
                div {
                    style: if show_stage {
                        "width: 100%; display: grid; gap: 10px; grid-template-columns: repeat(auto-fit, minmax(180px, 240px)); justify-content: center; flex-shrink: 0;"
                    } else {
                        "flex: 1; min-height: 0; width: 100%; display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); align-content: center;"
                    },
                    // Local tile
                    div {
                        style: "position: relative; background: #1E293B; border-radius: 12px; overflow: hidden; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center;",
                        video {
                            id: "webrtc-local-video",
                            autoplay: true,
                            playsinline: true,
                            muted: true,
                            style: if show_local_cam { "width: 100%; height: 100%; object-fit: cover; display: block;" } else { "display: none;" }
                        }
                        if !show_local_cam {
                            div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px;",
                                Avatar {
                                    user_id: current_user_id.clone(),
                                    username: "Bạn".to_string(),
                                    avatar_url: None,
                                    size: if show_stage { 44 } else { 64 },
                                }
                                if !show_stage {
                                    span { style: "color: #94A3B8; font-size: 13px;", "Camera đang tắt" }
                                }
                            }
                        }
                        div { style: "position: absolute; bottom: 8px; left: 8px; background: rgba(0,0,0,0.6); padding: 2px 8px; border-radius: 4px; font-size: 12px; color: white;",
                            "Bạn"
                            if is_muted() { " · Đã tắt tiếng" }
                        }
                    }

                    // Remote participant tiles
                    for p in peers.iter() {
                        {
                            let uid = p.user_id.clone();
                            let is_speaking = active_speaker_id().as_deref() == Some(&uid);
                            let peer_has_cam = active_remote_cams.contains(&uid);
                            let tile_bg = if is_speaking { "#173A2C" } else { "#1E293B" };
                            rsx! {
                                div {
                                    key: "{p.user_id}",
                                    style: "position: relative; background: {tile_bg}; border-radius: 12px; overflow: hidden; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center;",
                                    video {
                                        id: "webrtc-remote-video-{p.user_id}",
                                        autoplay: true,
                                        playsinline: true,
                                        style: if peer_has_cam { "width: 100%; height: 100%; object-fit: cover; display: block;" } else { "display: none;" }
                                    }
                                    audio {
                                        id: "webrtc-remote-audio-{p.user_id}",
                                        autoplay: true,
                                    }
                                    if !peer_has_cam {
                                        div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px;",
                                            Avatar {
                                                user_id: p.user_id.clone(),
                                                username: p.username.clone(),
                                                avatar_url: p.avatar_url.clone(),
                                                size: if show_stage { 44 } else { 64 },
                                            }
                                            if !show_stage {
                                                span { style: "color: #94A3B8; font-size: 13px;", "{p.username}" }
                                            }
                                        }
                                    }
                                    div { style: "position: absolute; bottom: 8px; left: 8px; background: rgba(0,0,0,0.6); padding: 2px 8px; border-radius: 4px; font-size: 12px; color: white;",
                                        "{p.username}"
                                        if p.muted { " · Đã tắt tiếng" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Bottom Floating Controls
            div { style: "display: flex; align-items: center; justify-content: center; gap: 20px; padding: 16px 0 4px 0; margin-top: auto;",
                // Mic Button
                button {
                    r#type: "button",
                    class: "partner-action-btn",
                    title: if is_muted() { "Bật micro" } else { "Tắt micro" },
                    aria_label: if is_muted() { "Bật micro" } else { "Tắt micro" },
                    style: if is_muted() { "background: #EF4444; color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" } else { "background: rgba(255,255,255,0.15); color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" },
                    onclick: handle_toggle_mic,
                    if is_muted() {
                        MicOffIcon { size: 22 }
                    } else {
                        MicIcon { size: 22 }
                    }
                }

                // Camera Button
                button {
                    r#type: "button",
                    class: "partner-action-btn",
                    title: if is_camera_off() { "Bật camera" } else { "Tắt camera" },
                    aria_label: if is_camera_off() { "Bật camera" } else { "Tắt camera" },
                    style: if is_camera_off() { "background: #EF4444; color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" } else { "background: rgba(255,255,255,0.15); color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" },
                    onclick: handle_toggle_camera,
                    if is_camera_off() {
                        VideoOffIcon { size: 22 }
                    } else {
                        VideoIcon { size: 22 }
                    }
                }

                // Screen Share Button
                button {
                    r#type: "button",
                    class: "partner-action-btn",
                    title: if is_screen_sharing() { "Dừng chia sẻ màn hình" } else { "Chia sẻ màn hình" },
                    aria_label: if is_screen_sharing() { "Dừng chia sẻ màn hình" } else { "Chia sẻ màn hình" },
                    style: if is_screen_sharing() { "background: #2563EB; color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" } else { "background: rgba(255,255,255,0.15); color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" },
                    onclick: handle_toggle_screen,
                    MonitorIcon { size: 22 }
                }

                // End Call Button
                button {
                    r#type: "button",
                    class: "call-btn call-btn-danger",
                    title: "Rời cuộc gọi",
                    aria_label: "Rời cuộc gọi",
                    style: "background: #EF4444; color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;",
                    onclick: move |_| on_end_call.call(()),
                    PhoneOffIcon { size: 24 }
                }
            }
        }
    }
}
