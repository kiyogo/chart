use crate::{crypto::blob, platform, services::storage_sync};

use super::{
    LocalObjectMeta, extract_ref_id, get_local_object, put_local_object, sha256_hex,
};

#[derive(Clone, Debug, PartialEq)]
pub enum MediaFetchOutcome {
    Ready {
        bytes: Vec<u8>,
        mime: String,
        source: String, // "local_cache" | "server" | "peer_restored"
    },
    MissingNeedsRecovery {
        ref_id: String,
        availability_state: String,
        reason: String,
    },
    Error(String),
}

/// Local-first fetch with transparent SHA-256 verification, E2EE decryption, and automatic cache hydration + ack.
pub async fn load_or_fetch_media(
    url: &str,
    key_b64: &str,
    nonce_b64: &str,
    fallback_mime: &str,
    scope_type: &str,
    variant: &str,
) -> MediaFetchOutcome {
    let ref_id = extract_ref_id(url);

    // 1. Check local OPFS / IndexedDB / Native CAS first! Zero network traffic on hit.
    if let Some(ref rid) = ref_id {
        if let Some((plain_bytes, cached_mime, _sha)) = get_local_object(rid).await {
            let mime = if cached_mime != "application/octet-stream" {
                cached_mime
            } else {
                fallback_mime.to_string()
            };
            return MediaFetchOutcome::Ready {
                bytes: plain_bytes,
                mime,
                source: "local_cache".into(),
            };
        }
    }

    // 2. Fallback to network (`/api/attachments/{id}?variant=...` or legacy `/uploads/media/...`).
    let fetch_url = if let Some(ref rid) = ref_id {
        if variant != "original" {
            format!("/api/attachments/{rid}?variant={variant}")
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };

    match platform::fetch_bytes(&fetch_url).await {
        Ok(raw_bytes) => {
            let plain_bytes = if !key_b64.is_empty() {
                match blob::open(&raw_bytes, key_b64, nonce_b64) {
                    Ok(p) => p,
                    Err(e) => return MediaFetchOutcome::Error(e),
                }
            } else {
                raw_bytes
            };

            let sha = sha256_hex(&plain_bytes);
            if let Some(ref rid) = ref_id {
                let meta = LocalObjectMeta {
                    ref_id: rid.clone(),
                    scope_type: scope_type.to_string(),
                    file_name: format!("media_{rid}"),
                    mime_type: fallback_mime.to_string(),
                    sha256_plain: Some(sha.clone()),
                    availability_state: if scope_type == "dm" { "replicated_multi_device".into() } else { "server_durable".into() },
                };
                if let Ok(local_tier) = put_local_object(&meta, &plain_bytes).await {
                    let _ = storage_sync::ack_relay_copy(rid, &local_tier, &sha, variant == "original").await;
                }
            }

            MediaFetchOutcome::Ready {
                bytes: plain_bytes,
                mime: fallback_mime.to_string(),
                source: "server".into(),
            }
        }
        Err(err) => {
            if let Some(ref rid) = ref_id {
                if let Ok(meta) = storage_sync::get_ref_meta(rid).await {
                    let avail = meta.get("availability_state").and_then(|s| s.as_str()).unwrap_or("local_only").to_string();
                    let deleted = meta.get("deleted_for_all").and_then(|b| b.as_bool()).unwrap_or(false);
                    let reason = if deleted {
                        "Tệp đã bị người gửi xóa với mọi người".to_string()
                    } else if avail == "no_remaining_copies" {
                        "Không còn bản sao nào tồn tại trên máy bạn hoặc đối phương".to_string()
                    } else {
                        "Chưa có bản sao cục bộ; có thể yêu cầu khôi phục từ thiết bị khác".to_string()
                    };
                    return MediaFetchOutcome::MissingNeedsRecovery {
                        ref_id: rid.clone(),
                        availability_state: avail,
                        reason,
                    };
                }
            }
            MediaFetchOutcome::Error(err)
        }
    }
}

/// Handle an incoming WebRTC / Encrypted Relay recovery request from peer/own device.
pub async fn respond_to_incoming_recovery(request_id: &str, ref_id: &str, transport_mode: &str) {
    // Check if THIS device holds a verified local copy inside OPFS / IndexedDB / Native CAS!
    let Some((plain_bytes, mime, _sha)) = get_local_object(ref_id).await else {
        // Honestly report when this device does NOT have the file; never pretend!
        let _ = storage_sync::respond_recovery_request(
            request_id,
            "no_copy",
            None,
            None,
            Some("Thiết bị này cũng không còn bản lưu cục bộ"),
        )
        .await;
        return;
    };

    if transport_mode == "webrtc_p2p" {
        let rid_json = serde_json::to_string(request_id).unwrap_or_default();
        let ref_json = serde_json::to_string(ref_id).unwrap_or_default();
        let js = format!(
            r#"
            (async () => {{
                if (window.__chatwPeerTransfer) {{
                    await window.__chatwPeerTransfer.sendObjectOverPeer({rid_json}, {ref_json});
                }}
            }})();
            "#
        );
        let _ = dioxus::document::eval(&js);
        let _ = storage_sync::respond_recovery_request(
            request_id,
            "accept",
            Some("webrtc_p2p"),
            None,
            None,
        )
        .await;
        return;
    }

    // Encrypted Server Relay fallback: seal fresh E2EE relay payload or re-upload ciphertext & hand ref back.
    if let Ok(res) = storage_sync::upload_attachment_scoped(
        plain_bytes,
        &mime,
        "dm",
        None,
        Some(&format!("recovery_{ref_id}")),
        Some(&mime),
        None,
        false,
        true,
    )
    .await
    {
        if let Some(relay_id) = res.get("ref_id").and_then(|s| s.as_str()) {
            let _ = storage_sync::respond_recovery_request(
                request_id,
                "accept",
                Some("encrypted_relay"),
                Some(relay_id),
                None,
            )
            .await;
        }
    }
}

pub async fn relay_recovery_p2p_signal(request_id: &str, signal_type: &str, data: &serde_json::Value) {
    let rid_json = serde_json::to_string(request_id).unwrap_or_default();
    let st_json = serde_json::to_string(signal_type).unwrap_or_default();
    let d_json = serde_json::to_string(data).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (window.__chatwPeerTransfer) {{
                await window.__chatwPeerTransfer.handleSignal({rid_json}, {st_json}, {d_json});
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}

pub async fn initiate_p2p_recovery_offer(request_id: &str) {
    let rid_json = serde_json::to_string(request_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (window.__chatwPeerTransfer) {{
                await window.__chatwPeerTransfer.initiateOffer({rid_json});
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}
