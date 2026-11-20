//! WebRTC SFU Media Controller for Dioxus (Web WASM & Desktop WebView)
//! Connects each participant to the backend Rust (`str0m`) SFU server over a single
//! RTCPeerConnection + DataChannel ("offer/answer"), publishing microphone, camera,
//! and screen share independently and receiving server-forwarded streams for all peers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SfuUiMediaState {
    pub local_screen: bool,
    pub local_cam: bool,
    pub remote_screen_user_id: Option<String>,
    pub remote_cam_user_ids: Vec<String>,
    pub active_speaker: Option<String>,
    pub ice_state: String,
}

/// Initialize the per-call SFU WebRTC manager and start connecting to `/api/calls/{id}/sfu/join`.
pub fn init_webrtc_manager(call_id: &str, is_video: bool, local_user_id: &str) {
    let call_id_escaped = serde_json::to_string(call_id).unwrap_or_default();
    let local_id_escaped = serde_json::to_string(local_user_id).unwrap_or_default();
    let video_bool = if is_video { "true" } else { "false" };

    let js = format!(
        r#"
        (() => {{
            if (window.__chatwWebRTC && window.__chatwWebRTC.cleanup) {{
                try {{ window.__chatwWebRTC.cleanup(false); }} catch {{}}
            }}

            const mgr = {{
                callId: {call_id_escaped},
                localUserId: {local_id_escaped},
                isVideo: {video_bool},
                pc: null,
                dataChannel: null,
                localAudioStream: null,
                localCamStream: null,
                screenStream: null,
                audioTransceiver: null,
                camTransceiver: null,
                screenTransceiver: null,
                pendingTagsByTransceiver: [],
                remoteMetaByMid: {{}},
                remoteAudioStreams: new Map(),
                remoteCamStreams: new Map(),
                remoteScreenStreams: new Map(),
                activeRemoteScreenUserId: null,
                audioCtx: null,
                analyser: null,
                isScreenSharing: false,
                hasLocalCam: false,
                micMuted: false,
                camOff: !({video_bool}),
                iceState: 'new',
                negotiating: false,
                negotiateQueued: false,
                nextOfferId: 1,
                pendingOffer: null,
                sdpQueue: Promise.resolve(),
                channelOpenWaiters: [],
                disposed: false,
            }};
            window.__chatwWebRTC = mgr;

            mgr.enqueueSdp = (fn) => {{
                mgr.sdpQueue = mgr.sdpQueue.then(fn, fn);
                return mgr.sdpQueue;
            }};

            mgr.waitForChannelOpen = () => {{
                if (mgr.disposed) return Promise.resolve(false);
                if (mgr.dataChannel && mgr.dataChannel.readyState === 'open') {{
                    return Promise.resolve(true);
                }}
                return new Promise((resolve) => {{
                    const timer = setTimeout(() => resolve(false), 12000);
                    mgr.channelOpenWaiters.push((ok) => {{
                        clearTimeout(timer);
                        resolve(ok);
                    }});
                }});
            }};

            mgr.collectMeta = () => {{
                const meta = {{}};
                if (mgr.audioTransceiver && mgr.audioTransceiver.mid) {{
                    meta[mgr.audioTransceiver.mid] = 'audio';
                }}
                if (mgr.camTransceiver && mgr.camTransceiver.mid) {{
                    meta[mgr.camTransceiver.mid] = 'cam';
                }}
                if (mgr.screenTransceiver && mgr.screenTransceiver.mid) {{
                    meta[mgr.screenTransceiver.mid] = 'screen';
                }}
                return meta;
            }};

            mgr.attachElement = (elId, stream, muted) => {{
                if (!elId || !stream) return;
                const el = document.getElementById(elId);
                if (!el) return;
                if (el.srcObject !== stream) {{
                    el.srcObject = stream;
                }}
                if (muted) el.muted = true;
                el.play().catch(() => {{}});
            }};

            mgr.syncAllDomElements = () => {{
                if (mgr.localCamStream && !mgr.camOff) {{
                    mgr.attachElement('webrtc-local-video', mgr.localCamStream, true);
                }}
                if (mgr.screenStream && mgr.isScreenSharing) {{
                    mgr.attachElement('webrtc-local-screen-video', mgr.screenStream, true);
                }}
                if (mgr.activeRemoteScreenUserId) {{
                    const rs = mgr.remoteScreenStreams.get(mgr.activeRemoteScreenUserId);
                    if (rs) {{
                        mgr.attachElement('webrtc-remote-screen-video', rs, false);
                    }}
                }}
                mgr.remoteCamStreams.forEach((stream, uid) => {{
                    mgr.attachElement('webrtc-remote-video-' + uid, stream, false);
                }});
                mgr.remoteAudioStreams.forEach((stream, uid) => {{
                    mgr.attachElement('webrtc-remote-audio-' + uid, stream, false);
                }});
            }};

            mgr.negotiateLocal = async () => {{
                if (mgr.disposed || !mgr.pc) return;
                if (mgr.negotiating) {{
                    mgr.negotiateQueued = true;
                    return;
                }}
                const ready = await mgr.waitForChannelOpen();
                if (!ready || mgr.disposed || !mgr.pc || !mgr.dataChannel || mgr.dataChannel.readyState !== 'open') {{
                    return;
                }}
                mgr.negotiating = true;
                let shouldRetry = false;
                try {{
                    await mgr.enqueueSdp(async () => {{
                        if (mgr.disposed || !mgr.pc || !mgr.dataChannel || mgr.dataChannel.readyState !== 'open') return;
                        if (mgr.pc.signalingState !== 'stable') {{
                            shouldRetry = true;
                            return;
                        }}
                        const oid = mgr.nextOfferId++;
                        const offer = await mgr.pc.createOffer();
                        await mgr.pc.setLocalDescription(offer);
                        const meta = mgr.collectMeta();
                        const answerPromise = new Promise((resolve) => {{
                            const t = setTimeout(() => {{
                                if (mgr.pendingOffer && mgr.pendingOffer.id === oid) {{
                                    mgr.pendingOffer = null;
                                }}
                                resolve(null);
                            }}, 8000);
                            mgr.pendingOffer = {{
                                id: oid,
                                resolve: (res) => {{
                                    clearTimeout(t);
                                    resolve(res);
                                }}
                            }};
                        }});
                        mgr.dataChannel.send(JSON.stringify({{
                            type: 'offer',
                            offer_id: oid,
                            sdp: offer.sdp,
                            meta
                        }}));
                        const res = await answerPromise;
                        if (res && res.retry) {{
                            if (mgr.pc && mgr.pc.signalingState === 'have-local-offer') {{
                                try {{ await mgr.pc.setLocalDescription({{ type: 'rollback' }}); }} catch {{}}
                            }}
                            shouldRetry = true;
                        }} else if (res && res.sdp && !mgr.disposed && mgr.pc) {{
                            await mgr.pc.setRemoteDescription({{ type: 'answer', sdp: res.sdp }});
                        }}
                    }});
                }} catch (e) {{
                    console.warn('[SFU] negotiateLocal error:', e);
                }} finally {{
                    mgr.negotiating = false;
                    if ((shouldRetry || mgr.negotiateQueued) && !mgr.disposed) {{
                        mgr.negotiateQueued = false;
                        setTimeout(() => mgr.negotiateLocal(), shouldRetry ? 60 : 10);
                    }}
                }}
            }};

            mgr.handleServerOffer = async (msg) => {{
                if (mgr.disposed || !mgr.pc || !msg.sdp) return;
                if (msg.meta) {{
                    Object.assign(mgr.remoteMetaByMid, msg.meta);
                }}
                await mgr.enqueueSdp(async () => {{
                    if (mgr.disposed || !mgr.pc) return;
                    try {{
                        if (mgr.pc.signalingState !== 'stable') {{
                            try {{ await mgr.pc.setLocalDescription({{ type: 'rollback' }}); }} catch {{}}
                            mgr.negotiateQueued = true;
                        }}
                        await mgr.pc.setRemoteDescription({{ type: 'offer', sdp: msg.sdp }});
                        const answer = await mgr.pc.createAnswer();
                        await mgr.pc.setLocalDescription(answer);
                        if (mgr.dataChannel && mgr.dataChannel.readyState === 'open') {{
                            mgr.dataChannel.send(JSON.stringify({{
                                type: 'answer',
                                offer_id: msg.offer_id,
                                sdp: answer.sdp
                            }}));
                        }}
                    }} catch (e) {{
                        console.warn('[SFU] handleServerOffer error:', e);
                    }}
                }});
            }};

            mgr.stopScreenShare = async (skipNegotiate) => {{
                const mid = mgr.screenTransceiver ? mgr.screenTransceiver.mid : null;
                if (mgr.screenStream) {{
                    mgr.screenStream.getTracks().forEach(t => {{
                        t.onended = null;
                        try {{ t.stop(); }} catch {{}}
                    }});
                    mgr.screenStream = null;
                }}
                mgr.isScreenSharing = false;
                if (mgr.screenTransceiver) {{
                    try {{
                        await mgr.screenTransceiver.sender.replaceTrack(null);
                    }} catch {{}}
                    try {{
                        mgr.screenTransceiver.direction = 'inactive';
                        if (typeof mgr.screenTransceiver.stop === 'function') {{
                            mgr.screenTransceiver.stop();
                        }}
                    }} catch {{}}
                    mgr.screenTransceiver = null;
                }}
                if (mid && mgr.dataChannel && mgr.dataChannel.readyState === 'open') {{
                    try {{
                        mgr.dataChannel.send(JSON.stringify({{ type: 'track_stop', mid }}));
                    }} catch {{}}
                }}
                if (!skipNegotiate && !mgr.disposed) {{
                    await mgr.negotiateLocal();
                }}
            }};

            mgr.cleanup = (notifyServer) => {{
                mgr.disposed = true;
                while (mgr.channelOpenWaiters.length) {{
                    try {{ mgr.channelOpenWaiters.shift()(false); }} catch {{}}
                }}
                if (mgr.pendingOffer) {{
                    try {{ mgr.pendingOffer.resolve(null); }} catch {{}}
                    mgr.pendingOffer = null;
                }}
                if (mgr.screenStream) {{
                    mgr.screenStream.getTracks().forEach(t => {{
                        t.onended = null;
                        try {{ t.stop(); }} catch {{}}
                    }});
                    mgr.screenStream = null;
                }}
                if (mgr.localAudioStream) {{
                    mgr.localAudioStream.getTracks().forEach(t => {{ try {{ t.stop(); }} catch {{}} }});
                    mgr.localAudioStream = null;
                }}
                if (mgr.localCamStream) {{
                    mgr.localCamStream.getTracks().forEach(t => {{ try {{ t.stop(); }} catch {{}} }});
                    mgr.localCamStream = null;
                }}
                if (mgr.pc) {{
                    try {{ mgr.pc.close(); }} catch {{}}
                    mgr.pc = null;
                }}
                if (mgr.audioCtx) {{
                    try {{ mgr.audioCtx.close(); }} catch {{}}
                    mgr.audioCtx = null;
                }}
                mgr.remoteAudioStreams.clear();
                mgr.remoteCamStreams.clear();
                mgr.remoteScreenStreams.clear();
                mgr.activeRemoteScreenUserId = null;
                if (notifyServer && mgr.callId) {{
                    fetch('/api/calls/' + encodeURIComponent(mgr.callId) + '/sfu/leave', {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: '{{}}'
                    }}).catch(() => {{}});
                }}
            }};

            const pc = new RTCPeerConnection();
            mgr.pc = pc;

            pc.oniceconnectionstatechange = () => {{
                mgr.iceState = pc.iceConnectionState || 'new';
                console.log('[SFU] ICE state:', mgr.iceState);
            }};

            const resolveTrackOrigin = (event) => {{
                const mid = event.transceiver ? event.transceiver.mid : null;
                if (mid && mgr.remoteMetaByMid[mid]) {{
                    return mgr.remoteMetaByMid[mid];
                }}
                const s = event.streams && event.streams[0];
                if (s && typeof s.id === 'string' && s.id.startsWith('sfu_')) {{
                    const rest = s.id.slice(4);
                    const lastUs = rest.lastIndexOf('_');
                    if (lastUs > 0) {{
                        return {{
                            user_id: rest.slice(0, lastUs),
                            tag: rest.slice(lastUs + 1)
                        }};
                    }}
                }}
                return null;
            }};

            pc.ontrack = (event) => {{
                const track = event.track;
                const info = resolveTrackOrigin(event);
                if (!info || !info.user_id) return;
                const uid = info.user_id;
                const tag = info.tag || (track.kind === 'audio' ? 'audio' : 'cam');
                const stream = (event.streams && event.streams[0]) ? event.streams[0] : new MediaStream([track]);
                if (!stream.getTracks().includes(track)) {{
                    stream.addTrack(track);
                }}

                console.log('[SFU] Remote track received:', uid, tag, track.kind);

                if (tag === 'screen') {{
                    mgr.remoteScreenStreams.set(uid, stream);
                    mgr.activeRemoteScreenUserId = uid;
                    setTimeout(() => mgr.syncAllDomElements(), 20);
                    track.addEventListener('unmute', () => mgr.syncAllDomElements());
                    track.addEventListener('ended', () => {{
                        if (mgr.remoteScreenStreams.get(uid) === stream) {{
                            mgr.remoteScreenStreams.delete(uid);
                            if (mgr.activeRemoteScreenUserId === uid) {{
                                const next = mgr.remoteScreenStreams.keys().next();
                                mgr.activeRemoteScreenUserId = next.done ? null : next.value;
                            }}
                        }}
                    }});
                }} else if (track.kind === 'video') {{
                    mgr.remoteCamStreams.set(uid, stream);
                    setTimeout(() => mgr.syncAllDomElements(), 20);
                    track.addEventListener('unmute', () => mgr.syncAllDomElements());
                    track.addEventListener('ended', () => {{
                        if (mgr.remoteCamStreams.get(uid) === stream) {{
                            mgr.remoteCamStreams.delete(uid);
                        }}
                    }});
                }} else {{
                    mgr.remoteAudioStreams.set(uid, stream);
                    setTimeout(() => mgr.syncAllDomElements(), 20);
                }}
            }};

            const dc = pc.createDataChannel('offer/answer');
            mgr.dataChannel = dc;
            dc.onopen = () => {{
                console.log('[SFU] DataChannel open for call:', mgr.callId);
                while (mgr.channelOpenWaiters.length) {{
                    try {{ mgr.channelOpenWaiters.shift()(true); }} catch {{}}
                }}
                mgr.syncAllDomElements();
            }};
            dc.onmessage = async (ev) => {{
                let msg;
                try {{ msg = JSON.parse(ev.data); }} catch {{ return; }}
                if (msg.type === 'offer') {{
                    if (mgr.pendingOffer) {{
                        const p = mgr.pendingOffer;
                        mgr.pendingOffer = null;
                        p.resolve({{ retry: true }});
                    }}
                    await mgr.handleServerOffer(msg);
                }} else if (msg.type === 'retry_offer') {{
                    if (mgr.pendingOffer && (!msg.offer_id || msg.offer_id === mgr.pendingOffer.id)) {{
                        const p = mgr.pendingOffer;
                        mgr.pendingOffer = null;
                        p.resolve({{ retry: true }});
                    }}
                }} else if (msg.type === 'answer') {{
                    if (mgr.pendingOffer && (!msg.offer_id || msg.offer_id === mgr.pendingOffer.id)) {{
                        const p = mgr.pendingOffer;
                        mgr.pendingOffer = null;
                        p.resolve({{ sdp: msg.sdp }});
                    }}
                }} else if (msg.type === 'track_stopped' && msg.user_id) {{
                    if (msg.tag === 'screen') {{
                        mgr.remoteScreenStreams.delete(msg.user_id);
                        if (mgr.activeRemoteScreenUserId === msg.user_id) {{
                            const next = mgr.remoteScreenStreams.keys().next();
                            mgr.activeRemoteScreenUserId = next.done ? null : next.value;
                        }}
                    }} else if (msg.tag === 'cam') {{
                        mgr.remoteCamStreams.delete(msg.user_id);
                    }} else if (msg.tag === 'audio') {{
                        mgr.remoteAudioStreams.delete(msg.user_id);
                    }}
                }}
            }};

            // Start SFU handshake immediately (works even with zero local hardware devices)
            (async () => {{
                try {{
                    const offer = await pc.createOffer();
                    await pc.setLocalDescription(offer);
                    const res = await fetch('/api/calls/' + encodeURIComponent(mgr.callId) + '/sfu/join', {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ sdp: offer.sdp, meta: {{}} }})
                    }});
                    if (!res.ok) {{
                        console.warn('[SFU] join HTTP error:', res.status);
                        return;
                    }}
                    const ans = await res.json();
                    if (!mgr.disposed && ans && ans.sdp) {{
                        await pc.setRemoteDescription({{ type: 'answer', sdp: ans.sdp }});
                    }}
                }} catch (e) {{
                    console.warn('[SFU] initial join error:', e);
                }}
            }})();
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Request camera/mic permission (graceful fallback if camera/mic hardware unavailable) and publish to SFU
pub async fn start_local_media(is_video: bool) -> Result<(), String> {
    let video_bool = if is_video { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr || mgr.disposed || !mgr.pc) {{
                dioxus.send({{ ok: false }});
                return;
            }}
            if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {{
                dioxus.send({{ ok: true, warning: 'no_media_devices' }});
                return;
            }}

            let stream = null;
            try {{
                stream = await navigator.mediaDevices.getUserMedia({{
                    audio: {{ echoCancellation: true, noiseSuppression: true }},
                    video: {video_bool} ? {{ width: {{ ideal: 1280 }}, height: {{ ideal: 720 }} }} : false
                }});
            }} catch (err) {{
                // Fallback to audio-only if camera failed/missing
                if ({video_bool}) {{
                    try {{
                        stream = await navigator.mediaDevices.getUserMedia({{
                            audio: {{ echoCancellation: true, noiseSuppression: true }},
                            video: false
                        }});
                    }} catch {{}}
                }}
            }}

            if (!stream || mgr.disposed || !mgr.pc) {{
                dioxus.send({{ ok: true, no_hardware: true }});
                return;
            }}

            const audioTracks = stream.getAudioTracks();
            const videoTracks = stream.getVideoTracks();

            if (audioTracks.length > 0) {{
                const aStream = new MediaStream([audioTracks[0]]);
                mgr.localAudioStream = aStream;
                audioTracks[0].enabled = !mgr.micMuted;
                if (!mgr.audioTransceiver) {{
                    mgr.audioTransceiver = mgr.pc.addTransceiver(audioTracks[0], {{
                        direction: 'sendonly',
                        streams: [aStream]
                    }});
                }} else {{
                    await mgr.audioTransceiver.sender.replaceTrack(audioTracks[0]).catch(() => {{}});
                }}

                try {{
                    const AC = window.AudioContext || window.webkitAudioContext;
                    if (AC) {{
                        mgr.audioCtx = new AC();
                        mgr.analyser = mgr.audioCtx.createAnalyser();
                        mgr.analyser.fftSize = 256;
                        const source = mgr.audioCtx.createMediaStreamSource(aStream);
                        source.connect(mgr.analyser);
                    }}
                }} catch {{}}
            }}

            if (videoTracks.length > 0) {{
                const vStream = new MediaStream([videoTracks[0]]);
                mgr.localCamStream = vStream;
                mgr.hasLocalCam = true;
                mgr.camOff = false;
                videoTracks[0].enabled = true;
                if (!mgr.camTransceiver) {{
                    mgr.camTransceiver = mgr.pc.addTransceiver(videoTracks[0], {{
                        direction: 'sendonly',
                        streams: [vStream]
                    }});
                }} else {{
                    await mgr.camTransceiver.sender.replaceTrack(videoTracks[0]).catch(() => {{}});
                }}
                mgr.syncAllDomElements();
            }}

            if (audioTracks.length > 0 || videoTracks.length > 0) {{
                await mgr.negotiateLocal();
            }}
            dioxus.send({{ ok: true }});
        }})();
        "#
    );

    let mut eval = dioxus::document::eval(&js);
    let _ = eval.recv::<serde_json::Value>().await;
    Ok(())
}

/// Legacy P2P signal no-op / compat stub (SFU handles media & DataChannel negotiation)
pub async fn create_offer_signal(_call_id: &str, _target_peer_id: &str) -> Result<serde_json::Value, String> {
    Err("sfu_mode".into())
}

pub async fn handle_remote_signal(
    _call_id: &str,
    _sender_id: &str,
    _signal_type: &str,
    _data: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    Ok(None)
}

/// Toggle microphone track (mute/unmute)
pub fn toggle_mic(muted: bool) {
    let muted_bool = if muted { "true" } else { "false" };
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (mgr) {{
                mgr.micMuted = {muted_bool};
                if (mgr.localAudioStream) {{
                    mgr.localAudioStream.getAudioTracks().forEach(t => {{
                        t.enabled = !({muted_bool});
                    }});
                }}
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Toggle camera track (on/off); acquires webcam on demand if call began as audio-only
pub async fn toggle_camera_async(camera_off: bool) -> bool {
    let off_bool = if camera_off { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr || mgr.disposed || !mgr.pc) {{ dioxus.send(true); return; }}
            if ({off_bool}) {{
                mgr.camOff = true;
                if (mgr.localCamStream) {{
                    mgr.localCamStream.getVideoTracks().forEach(t => {{ t.enabled = false; }});
                }}
                dioxus.send(true);
                return;
            }}
            mgr.camOff = false;
            if (mgr.localCamStream && mgr.localCamStream.getVideoTracks().some(t => t.readyState === 'live')) {{
                mgr.localCamStream.getVideoTracks().forEach(t => {{ t.enabled = true; }});
                mgr.hasLocalCam = true;
                mgr.syncAllDomElements();
                dioxus.send(false);
                return;
            }}
            try {{
                const camStream = await navigator.mediaDevices.getUserMedia({{
                    video: {{ width: {{ ideal: 1280 }}, height: {{ ideal: 720 }} }}
                }});
                const vTrack = camStream.getVideoTracks()[0];
                if (!vTrack) {{
                    mgr.camOff = true;
                    dioxus.send(true);
                    return;
                }}
                mgr.localCamStream = camStream;
                mgr.hasLocalCam = true;
                mgr.camOff = false;
                if (!mgr.camTransceiver) {{
                    mgr.camTransceiver = mgr.pc.addTransceiver(vTrack, {{
                        direction: 'sendonly',
                        streams: [camStream]
                    }});
                }} else {{
                    await mgr.camTransceiver.sender.replaceTrack(vTrack).catch(() => {{}});
                }}
                mgr.syncAllDomElements();
                await mgr.negotiateLocal();
                dioxus.send(false);
            }} catch (e) {{
                mgr.camOff = true;
                dioxus.send(true);
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    eval.recv::<bool>().await.unwrap_or(camera_off)
}

pub fn toggle_camera(camera_off: bool) {
    let off_bool = if camera_off { "true" } else { "false" };
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (mgr) {{
                mgr.camOff = {off_bool};
                if (mgr.localCamStream) {{
                    mgr.localCamStream.getVideoTracks().forEach(t => {{
                        t.enabled = !({off_bool});
                    }});
                }}
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Clean up and stop all tracks and close SFU peer connection
pub fn stop_all_media() {
    let js = r#"
        (() => {
            const mgr = window.__chatwWebRTC;
            if (mgr && mgr.cleanup) {
                mgr.cleanup(true);
            }
            window.__chatwWebRTC = null;
        })();
    "#;
    let _ = dioxus::document::eval(js);
}

/// Attach local camera and screen streams if mounted
pub fn attach_local_stream_to_element(element_id: &str) {
    let el_escaped = serde_json::to_string(element_id).unwrap_or_default();
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr) return;
            const elId = {el_escaped};
            if (elId === 'webrtc-local-screen-video' && mgr.screenStream) {{
                mgr.attachElement(elId, mgr.screenStream, true);
            }} else if (mgr.localCamStream && !mgr.camOff) {{
                mgr.attachElement(elId, mgr.localCamStream, true);
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Attach remote camera, screen, and audio streams if mounted
pub fn attach_remote_stream_to_element(element_id: &str, peer_id: &str) {
    let el_escaped = serde_json::to_string(element_id).unwrap_or_default();
    let peer_escaped = serde_json::to_string(peer_id).unwrap_or_default();
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr) return;
            const elId = {el_escaped};
            const pid = {peer_escaped};
            if (elId.startsWith('webrtc-remote-screen')) {{
                const s = mgr.remoteScreenStreams.get(pid);
                if (s) mgr.attachElement(elId, s, false);
            }} else if (elId.startsWith('webrtc-remote-audio')) {{
                const s = mgr.remoteAudioStreams.get(pid);
                if (s) mgr.attachElement(elId, s, false);
            }} else {{
                const s = mgr.remoteCamStreams.get(pid);
                if (s) mgr.attachElement(elId, s, false);
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Sync all SFU media DOM elements + return live media state & active speaker
pub async fn poll_sfu_ui_state() -> SfuUiMediaState {
    let js = r#"
        (() => {
            const mgr = window.__chatwWebRTC;
            if (!mgr || mgr.disposed) {
                dioxus.send({
                    local_screen: false,
                    local_cam: false,
                    remote_screen_user_id: null,
                    remote_cam_user_ids: [],
                    active_speaker: null,
                    ice_state: 'closed'
                });
                return;
            }
            mgr.syncAllDomElements();
            let speaker = null;
            if (mgr.analyser) {
                try {
                    const data = new Uint8Array(mgr.analyser.frequencyBinCount);
                    mgr.analyser.getByteFrequencyData(data);
                    let sum = 0;
                    for (let i = 0; i < data.length; i++) sum += data[i];
                    if (sum / data.length > 25) speaker = mgr.localUserId;
                } catch {}
            }
            dioxus.send({
                local_screen: Boolean(mgr.isScreenSharing && mgr.screenStream),
                local_cam: Boolean(mgr.hasLocalCam && !mgr.camOff && mgr.localCamStream),
                remote_screen_user_id: mgr.activeRemoteScreenUserId || null,
                remote_cam_user_ids: Array.from(mgr.remoteCamStreams.keys()),
                active_speaker: speaker,
                ice_state: mgr.iceState || 'new'
            });
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    eval.recv::<SfuUiMediaState>().await.unwrap_or_default()
}

pub async fn check_active_speaker() -> Option<String> {
    poll_sfu_ui_state().await.active_speaker
}

/// Start or stop Screen Sharing and push stream to backend Rust SFU for all participants
pub async fn toggle_screen_share(enable: bool) -> bool {
    let en = if enable { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr || mgr.disposed || !mgr.pc) {{
                dioxus.send(false);
                return;
            }}
            try {{
                if ({en}) {{
                    if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) {{
                        dioxus.send(false);
                        return;
                    }}
                    let displayStream = null;
                    try {{
                        displayStream = await navigator.mediaDevices.getDisplayMedia({{
                            video: {{ cursor: 'always', frameRate: {{ ideal: 30, max: 60 }} }},
                            audio: false
                        }});
                    }} catch {{
                        displayStream = await navigator.mediaDevices.getDisplayMedia({{ video: true }});
                    }}
                    const screenTrack = displayStream.getVideoTracks()[0];
                    if (!screenTrack) {{
                        dioxus.send(false);
                        return;
                    }}
                    if (mgr.screenStream) {{
                        await mgr.stopScreenShare(true);
                    }}
                    mgr.screenStream = displayStream;
                    mgr.isScreenSharing = true;

                    screenTrack.onended = () => {{
                        if (window.__chatwWebRTC === mgr && !mgr.disposed) {{
                            mgr.stopScreenShare(false);
                        }}
                    }};

                    mgr.screenTransceiver = mgr.pc.addTransceiver(screenTrack, {{
                        direction: 'sendonly',
                        streams: [displayStream]
                    }});

                    // Immediately attach preview & also after React/Dioxus DOM mounts the stage
                    mgr.syncAllDomElements();
                    setTimeout(() => mgr.syncAllDomElements(), 80);

                    await mgr.negotiateLocal();
                    dioxus.send(true);
                }} else {{
                    await mgr.stopScreenShare(false);
                    dioxus.send(false);
                }}
            }} catch (e) {{
                console.warn('[SFU] toggle_screen_share error:', e);
                dioxus.send(false);
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    eval.recv::<bool>().await.unwrap_or(false)
}
