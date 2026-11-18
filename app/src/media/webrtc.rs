/// WebRTC Media Controller for Dioxus (Web WASM & Desktop WebView)
/// Manages real media devices (mic, camera), RTCPeerConnections, tracks, and audio analyser.

pub fn init_webrtc_manager(call_id: &str, is_video: bool, local_user_id: &str) {
    let call_id_escaped = serde_json::to_string(call_id).unwrap_or_default();
    let local_id_escaped = serde_json::to_string(local_user_id).unwrap_or_default();
    let video_bool = if is_video { "true" } else { "false" };

    let js = format!(
        r#"
        (() => {{
            window.__chatwWebRTC = window.__chatwWebRTC || {{}};
            const mgr = window.__chatwWebRTC;
            mgr.callId = {call_id_escaped};
            mgr.localUserId = {local_id_escaped};
            mgr.isVideo = {video_bool};
            mgr.localStream = null;
            mgr.peerConnections = new Map();
            mgr.remoteStreams = new Map();
            mgr.audioCtx = null;
            mgr.analyser = null;
            mgr.activeSpeaker = null;

            mgr.rtcConfig = {{
                iceServers: [
                    {{ urls: 'stun:stun.l.google.com:19302' }},
                    {{ urls: 'stun:stun1.l.google.com:19302' }}
                ]
            }};

            console.log('[WebRTC] Manager initialized for call:', mgr.callId, 'video:', mgr.isVideo);
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Request real camera/mic permission and start local media stream
pub async fn start_local_media(is_video: bool) -> Result<(), String> {
    let video_bool = if is_video { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            const mgr = window.__chatwWebRTC || {{}};
            try {{
                if (mgr.localStream) {{
                    mgr.localStream.getTracks().forEach(t => t.stop());
                }}

                const constraints = {{
                    audio: {{ echoCancellation: true, noiseSuppression: true }},
                    video: {video_bool} ? {{ width: {{ ideal: 1280 }}, height: {{ ideal: 720 }} }} : false
                }};

                const stream = await navigator.mediaDevices.getUserMedia(constraints);
                mgr.localStream = stream;

                // Attach to local preview video element if present
                const localVideo = document.getElementById('webrtc-local-video');
                if (localVideo) {{
                    localVideo.srcObject = stream;
                    localVideo.muted = true;
                    localVideo.play().catch(() => {{}});
                }}

                // Setup audio analyser for active speaker detection
                try {{
                    const AC = window.AudioContext || window.webkitAudioContext;
                    if (AC) {{
                        mgr.audioCtx = new AC();
                        mgr.analyser = mgr.audioCtx.createAnalyser();
                        mgr.analyser.fftSize = 256;
                        const source = mgr.audioCtx.createMediaStreamSource(stream);
                        source.connect(mgr.analyser);
                    }}
                }} catch (e) {{
                    console.warn('[WebRTC] AudioContext failed:', e);
                }}

                console.log('[WebRTC] Local media stream captured successfully. Tracks:', stream.getTracks().length);
                dioxus.send({{ ok: true }});
            }} catch (err) {{
                console.error('[WebRTC] getUserMedia error:', err);
                dioxus.send({{ error: String(err && err.message ? err.message : err) }});
            }}
        }})();
        "#
    );

    let mut eval = dioxus::document::eval(&js);
    let res: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if let Some(err) = res.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    Ok(())
}

/// Initiate peer connection and generate SDP Offer to send to remote peer
pub async fn create_offer_signal(call_id: &str, target_peer_id: &str) -> Result<serde_json::Value, String> {
    let call_id_escaped = serde_json::to_string(call_id).unwrap_or_default();
    let peer_id_escaped = serde_json::to_string(target_peer_id).unwrap_or_default();

    let js = format!(
        r#"
        (async () => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr) {{ dioxus.send({{ error: 'no_manager' }}); return; }}

            const peerId = {peer_id_escaped};
            let pc = mgr.peerConnections.get(peerId);
            if (!pc) {{
                pc = new RTCPeerConnection(mgr.rtcConfig);
                mgr.peerConnections.set(peerId, pc);

                // Add local tracks
                if (mgr.localStream) {{
                    mgr.localStream.getTracks().forEach(track => pc.addTrack(track, mgr.localStream));
                }}

                // ICE candidates
                pc.onicecandidate = (event) => {{
                    if (event.candidate) {{
                        fetch('/api/calls/' + {call_id_escaped} + '/signal', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify({{
                                target_user_id: peerId,
                                signal_type: 'ice_candidate',
                                data: event.candidate
                            }})
                        }}).catch(() => {{}});
                    }}
                }};

                // Remote tracks
                pc.ontrack = (event) => {{
                    console.log('[WebRTC] Received remote track from peer:', peerId, event.track.kind);
                    let remoteStream = mgr.remoteStreams.get(peerId);
                    if (!remoteStream) {{
                        remoteStream = new MediaStream();
                        mgr.remoteStreams.set(peerId, remoteStream);
                    }}
                    remoteStream.addTrack(event.track);

                    // Attach to DOM element
                    const elVideo = document.getElementById('webrtc-remote-video-' + peerId);
                    if (elVideo) {{
                        elVideo.srcObject = remoteStream;
                        elVideo.play().catch(() => {{}});
                    }}
                    const elAudio = document.getElementById('webrtc-remote-audio-' + peerId);
                    if (elAudio) {{
                        elAudio.srcObject = remoteStream;
                        elAudio.play().catch(() => {{}});
                    }}
                }};
            }}

            try {{
                const offer = await pc.createOffer();
                await pc.setLocalDescription(offer);
                dioxus.send({{ ok: true, sdp: offer }});
            }} catch (e) {{
                dioxus.send({{ error: String(e) }});
            }}
        }})();
        "#
    );

    let mut eval = dioxus::document::eval(&js);
    let res: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if let Some(err) = res.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    res.get("sdp").cloned().ok_or_else(|| "missing_sdp".to_string())
}

/// Handle incoming signal (offer, answer, ice_candidate) from remote peer
pub async fn handle_remote_signal(
    call_id: &str,
    sender_id: &str,
    signal_type: &str,
    data: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let call_id_escaped = serde_json::to_string(call_id).unwrap_or_default();
    let sender_id_escaped = serde_json::to_string(sender_id).unwrap_or_default();
    let type_escaped = serde_json::to_string(signal_type).unwrap_or_default();
    let data_escaped = serde_json::to_string(&data).unwrap_or_default();

    let js = format!(
        r#"
        (async () => {{
            const mgr = window.__chatwWebRTC;
            if (!mgr) {{ dioxus.send({{ ok: false, error: 'no_manager' }}); return; }}

            const peerId = {sender_id_escaped};
            const sigType = {type_escaped};
            const sigData = {data_escaped};

            let pc = mgr.peerConnections.get(peerId);
            if (!pc) {{
                pc = new RTCPeerConnection(mgr.rtcConfig);
                mgr.peerConnections.set(peerId, pc);

                if (mgr.localStream) {{
                    mgr.localStream.getTracks().forEach(track => pc.addTrack(track, mgr.localStream));
                }}

                pc.onicecandidate = (event) => {{
                    if (event.candidate) {{
                        fetch('/api/calls/' + {call_id_escaped} + '/signal', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify({{
                                target_user_id: peerId,
                                signal_type: 'ice_candidate',
                                data: event.candidate
                            }})
                        }}).catch(() => {{}});
                    }}
                }};

                pc.ontrack = (event) => {{
                    console.log('[WebRTC] Received remote track from peer:', peerId, event.track.kind);
                    let remoteStream = mgr.remoteStreams.get(peerId);
                    if (!remoteStream) {{
                        remoteStream = new MediaStream();
                        mgr.remoteStreams.set(peerId, remoteStream);
                    }}
                    remoteStream.addTrack(event.track);

                    const elVideo = document.getElementById('webrtc-remote-video-' + peerId);
                    if (elVideo) {{
                        elVideo.srcObject = remoteStream;
                        elVideo.play().catch(() => {{}});
                    }}
                    const elAudio = document.getElementById('webrtc-remote-audio-' + peerId);
                    if (elAudio) {{
                        elAudio.srcObject = remoteStream;
                        elAudio.play().catch(() => {{}});
                    }}
                }};
            }}

            try {{
                if (sigType === 'offer') {{
                    await pc.setRemoteDescription(new RTCSessionDescription(sigData));
                    const answer = await pc.createAnswer();
                    await pc.setLocalDescription(answer);
                    dioxus.send({{ ok: true, answer: answer }});
                }} else if (sigType === 'answer') {{
                    await pc.setRemoteDescription(new RTCSessionDescription(sigData));
                    dioxus.send({{ ok: true }});
                }} else if (sigType === 'ice_candidate') {{
                    if (sigData) {{
                        await pc.addIceCandidate(new RTCIceCandidate(sigData));
                    }}
                    dioxus.send({{ ok: true }});
                }} else {{
                    dioxus.send({{ ok: true }});
                }}
            }} catch (err) {{
                console.error('[WebRTC] handle_remote_signal error:', err);
                dioxus.send({{ ok: false, error: String(err) }});
            }}
        }})();
        "#
    );

    let mut eval = dioxus::document::eval(&js);
    let res: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if let Some(err) = res.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    Ok(res.get("answer").cloned())
}

/// Toggle microphone track (mute/unmute)
pub fn toggle_mic(muted: bool) {
    let muted_bool = if muted { "true" } else { "false" };
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (mgr && mgr.localStream) {{
                mgr.localStream.getAudioTracks().forEach(t => {{
                    t.enabled = !({muted_bool});
                }});
                console.log('[WebRTC] Mic muted:', {muted_bool});
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Toggle camera track (on/off)
pub fn toggle_camera(camera_off: bool) {
    let off_bool = if camera_off { "true" } else { "false" };
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (mgr && mgr.localStream) {{
                mgr.localStream.getVideoTracks().forEach(t => {{
                    t.enabled = !({off_bool});
                }});
                console.log('[WebRTC] Camera off:', {off_bool});
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Clean up and stop all tracks and close peer connections
pub fn stop_all_media() {
    let js = r#"
        (() => {
            const mgr = window.__chatwWebRTC;
            if (mgr) {
                if (mgr.localStream) {
                    mgr.localStream.getTracks().forEach(t => {
                        try { t.stop(); } catch {}
                    });
                    mgr.localStream = null;
                }
                if (mgr.peerConnections) {
                    mgr.peerConnections.forEach(pc => {
                        try { pc.close(); } catch {}
                    });
                    mgr.peerConnections.clear();
                }
                if (mgr.remoteStreams) {
                    mgr.remoteStreams.forEach(stream => {
                        try { stream.getTracks().forEach(t => t.stop()); } catch {}
                    });
                    mgr.remoteStreams.clear();
                }
                if (mgr.audioCtx) {
                    try { mgr.audioCtx.close(); } catch {}
                    mgr.audioCtx = null;
                }
                console.log('[WebRTC] Cleaned up all media and connections.');
            }
        })();
    "#;
    let _ = dioxus::document::eval(js);
}

/// Attach local stream to element by ID
pub fn attach_local_stream_to_element(element_id: &str) {
    let el_escaped = serde_json::to_string(element_id).unwrap_or_default();
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (mgr && mgr.localStream) {{
                const el = document.getElementById({el_escaped});
                if (el) {{
                    el.srcObject = mgr.localStream;
                    el.muted = true;
                    el.play().catch(() => {{}});
                }}
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Attach remote peer stream to element by ID
pub fn attach_remote_stream_to_element(element_id: &str, peer_id: &str) {
    let el_escaped = serde_json::to_string(element_id).unwrap_or_default();
    let peer_escaped = serde_json::to_string(peer_id).unwrap_or_default();
    let js = format!(
        r#"
        (() => {{
            const mgr = window.__chatwWebRTC;
            if (mgr && mgr.remoteStreams) {{
                const stream = mgr.remoteStreams.get({peer_escaped});
                const el = document.getElementById({el_escaped});
                if (el && stream) {{
                    el.srcObject = stream;
                    el.play().catch(() => {{}});
                }}
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

/// Poll real audio RMS from AudioContext AnalyserNode to detect active speaker
pub async fn check_active_speaker() -> Option<String> {
    let js = r#"
        (() => {
            const mgr = window.__chatwWebRTC;
            if (!mgr || !mgr.analyser) { dioxus.send(null); return; }
            try {
                const data = new Uint8Array(mgr.analyser.frequencyBinCount);
                mgr.analyser.getByteFrequencyData(data);
                let sum = 0;
                for (let i = 0; i < data.length; i++) sum += data[i];
                const avg = sum / data.length;
                if (avg > 25) {
                    dioxus.send(mgr.localUserId);
                } else {
                    dioxus.send(null);
                }
            } catch {
                dioxus.send(null);
            }
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    eval.recv::<Option<String>>().await.unwrap_or(None)
}

