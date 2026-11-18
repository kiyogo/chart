use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{MicIcon, MicOffIcon, PhoneOffIcon, VideoIcon, VideoOffIcon},
    },
    media::{
        attach_local_stream_to_element, attach_remote_stream_to_element, check_active_speaker,
        toggle_camera, toggle_mic,
    },
    models::{CallParticipantItem, CallSessionItem},
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
    let mut active_speaker_id = use_signal(|| Option::<String>::None);
    let call_seconds = use_signal(|| 0u64);

    let is_video = session.call_type == "video";
    let is_group = session.group_id.is_some();
    let call_id = session.id.clone();

    // Call duration timer
    use_effect({
        move || {
            let mut sec = call_seconds;
            spawn(async move {
                loop {
                    crate::platform::sleep_ms(1000).await;
                    sec.with_mut(|s| *s += 1);
                }
            });
        }
    });

    // Active speaker detection polling
    use_effect(move || {
        spawn(async move {
            loop {
                crate::platform::sleep_ms(300).await;
                if let Some(speaker) = check_active_speaker().await {
                    active_speaker_id.set(Some(speaker));
                } else {
                    active_speaker_id.set(None);
                }
            }
        });
    });

    // Ensure local stream is attached to local video
    use_effect(move || {
        spawn(async move {
            crate::platform::sleep_ms(150).await;
            attach_local_stream_to_element("webrtc-local-video");
        });
    });

    // Ensure remote streams are attached to remote videos
    let peers: Vec<CallParticipantItem> = session
        .participants
        .iter()
        .filter(|p| p.user_id != current_user_id)
        .cloned()
        .collect();

    use_effect({
        let peers = peers.clone();
        move || {
            let peers = peers.clone();
            spawn(async move {
                crate::platform::sleep_ms(200).await;
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
            let new_off = !is_camera_off();
            is_camera_off.set(new_off);
            toggle_camera(new_off);
            let cid = cid.clone();
            spawn(async move {
                let _ = calls_update_state(&cid, None, Some(new_off)).await;
            });
        }
    };

    let call_title = if is_group {
        session.group_name.clone().unwrap_or_else(|| "Cuộc gọi nhóm".to_string())
    } else if session.caller_id == current_user_id {
        session.receiver_name.clone().unwrap_or_else(|| "Cuộc gọi".to_string())
    } else {
        session.caller_name.clone()
    };

    rsx! {
        div {
            class: "settings-modal-backdrop",
            style: "background: #0B0F17; z-index: 9999; display: flex; flex-direction: column; padding: 16px; box-sizing: border-box;",
            // Top Header
            div { style: "display: flex; align-items: center; justify-content: space-between; width: 100%; max-width: 1200px; margin: 0 auto 16px auto;",
                div { style: "display: flex; align-items: center; gap: 12px;",
                    div { style: "background: rgba(255, 255, 255, 0.1); padding: 6px 12px; border-radius: 20px; font-size: 13px; font-weight: 500; color: #10B981; display: flex; align-items: center; gap: 6px;",
                        span { style: "width: 8px; height: 8px; border-radius: 50%; background: #10B981; display: inline-block;" }
                        "{format_duration(call_seconds())}"
                    }
                    span { style: "font-weight: 600; font-size: 16px; color: #F8FAFC;", "{call_title}" }
                }
            }

            // Media Grid View
            div {
                style: "flex: 1; width: 100%; max-width: 1200px; margin: 0 auto; display: flex; align-items: center; justify-content: center; position: relative; overflow: hidden; border-radius: 16px; background: #131B2E;",
                if is_video {
                    // Video View
                    div {
                        style: "width: 100%; height: 100%; display: grid; gap: 12px; padding: 12px; box-sizing: border-box; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); align-content: center;",
                        // Local video tile
                        div {
                            style: "position: relative; background: #1E293B; border-radius: 12px; overflow: hidden; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center;",
                            video {
                                id: "webrtc-local-video",
                                autoplay: true,
                                playsinline: true,
                                muted: true,
                                style: if is_camera_off() { "display: none;" } else { "width: 100%; height: 100%; object-fit: cover;" }
                            }
                            if is_camera_off() {
                                div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px;",
                                    Avatar {
                                        user_id: current_user_id.clone(),
                                        username: "Bạn".to_string(),
                                        avatar_url: None,
                                        size: 64,
                                    }
                                    span { style: "color: #94A3B8; font-size: 13px;", "Camera đang tắt" }
                                }
                            }
                            div { style: "position: absolute; bottom: 8px; left: 8px; background: rgba(0,0,0,0.6); padding: 2px 8px; border-radius: 4px; font-size: 12px; color: white;",
                                "Bạn"
                                if is_muted() { " (Muted)" }
                            }
                        }

                        // Remote video tiles
                        for p in peers.iter() {
                            {
                                let uid = p.user_id.clone();
                                let is_speaking = active_speaker_id().as_deref() == Some(&uid);
                                let border_style = if is_speaking { "2px solid #10B981" } else { "1px solid rgba(255,255,255,0.05)" };
                                rsx! {
                                    div {
                                        key: "{p.user_id}",
                                        style: "position: relative; background: #1E293B; border-radius: 12px; overflow: hidden; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center; border: {border_style};",
                                        video {
                                            id: "webrtc-remote-video-{p.user_id}",
                                            autoplay: true,
                                            playsinline: true,
                                            style: if p.camera_off { "display: none;" } else { "width: 100%; height: 100%; object-fit: cover;" }
                                        }
                                        audio {
                                            id: "webrtc-remote-audio-{p.user_id}",
                                            autoplay: true,
                                        }
                                        if p.camera_off {
                                            div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px;",
                                                Avatar {
                                                    user_id: p.user_id.clone(),
                                                    username: p.username.clone(),
                                                    avatar_url: p.avatar_url.clone(),
                                                    size: 64,
                                                }
                                                span { style: "color: #94A3B8; font-size: 13px;", "{p.username}" }
                                            }
                                        }
                                        div { style: "position: absolute; bottom: 8px; left: 8px; background: rgba(0,0,0,0.6); padding: 2px 8px; border-radius: 4px; font-size: 12px; color: white;",
                                            "{p.username}"
                                            if p.muted { " (Muted)" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Audio View: Clean tiles with active speaker aura
                    div {
                        style: "display: flex; flex-wrap: wrap; gap: 24px; align-items: center; justify-content: center; padding: 24px;",
                        // Local tile
                        div {
                            style: "background: #1E293B; border-radius: 16px; padding: 24px 32px; display: flex; flex-direction: column; align-items: center; gap: 12px; min-width: 140px;",
                            Avatar {
                                user_id: current_user_id.clone(),
                                username: "Bạn".to_string(),
                                avatar_url: None,
                                size: 80,
                            }
                            span { style: "font-weight: 500; font-size: 14px; color: #F8FAFC;", "Bạn" }
                            if is_muted() {
                                span { style: "font-size: 11px; color: #EF4444; background: rgba(239, 68, 68, 0.15); padding: 2px 8px; border-radius: 10px;", "Tắt mic" }
                            }
                        }

                        // Remote peers
                        for p in peers.iter() {
                            {
                                let uid = p.user_id.clone();
                                let is_speaking = active_speaker_id().as_deref() == Some(&uid);
                                let glow = if is_speaking { "border: 2px solid var(--status-success);" } else { "" };
                                rsx! {
                                    div {
                                        key: "{p.user_id}",
                                        style: "background: #1E293B; border-radius: 16px; padding: 24px 32px; display: flex; flex-direction: column; align-items: center; gap: 12px; min-width: 140px; {glow}",
                                        audio {
                                            id: "webrtc-remote-audio-{p.user_id}",
                                            autoplay: true,
                                        }
                                        Avatar {
                                            user_id: p.user_id.clone(),
                                            username: p.username.clone(),
                                            avatar_url: p.avatar_url.clone(),
                                            size: 80,
                                        }
                                        span { style: "font-weight: 500; font-size: 14px; color: #F8FAFC;", "{p.username}" }
                                        if p.muted {
                                            span { style: "font-size: 11px; color: #EF4444; background: rgba(239, 68, 68, 0.15); padding: 2px 8px; border-radius: 10px;", "Tắt mic" }
                                        } else if is_speaking {
                                            span { style: "font-size: 11px; color: #10B981; background: rgba(16, 185, 129, 0.15); padding: 2px 8px; border-radius: 10px;", "Đang nói..." }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Bottom Floating Controls
            div { style: "display: flex; align-items: center; justify-content: center; gap: 20px; padding: 20px 0; margin-top: auto;",
                // Mic Button
                button {
                    class: "partner-action-btn",
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
                    class: "partner-action-btn",
                    style: if is_camera_off() { "background: #EF4444; color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" } else { "background: rgba(255,255,255,0.15); color: white; border: none; width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer;" },
                    onclick: handle_toggle_camera,
                    if is_camera_off() {
                        VideoOffIcon { size: 22 }
                    } else {
                        VideoIcon { size: 22 }
                    }
                }

                // End Call Button
                button {
                    class: "partner-action-btn",
                    class: "call-btn call-btn-danger",
                    onclick: move |_| on_end_call.call(()),
                    PhoneOffIcon { size: 24 }
                }
            }
        }
    }
}
