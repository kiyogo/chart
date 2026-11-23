use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use zeroize::Zeroize;

use crate::{
    crypto::{blob, e2ee},
    platform,
    services::{api, storage_sync},
};

use super::{
    LocalObjectMeta, extract_ref_id, get_local_object, put_local_object, sha256_hex,
};

const LOCAL_CACHE_MAGIC: &[u8; 5] = b"CWLC1";

#[allow(deprecated)]
fn wrap_local_e2ee_cache(ref_id: &str, plain_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let uid = api::auth_me_cached_id().ok_or_else(|| "no_active_user".to_string())?;
    let keys = e2ee::get_user_keys(Some(&uid)).ok_or_else(|| "e2ee_keys_locked".to_string())?;
    let mut subkey = keys.derive_subkey(&format!("local-media-cache-v1:{uid}:{ref_id}"))?;
    let cipher = Aes256Gcm::new_from_slice(&subkey).map_err(|_| "aes_init_failed".to_string());
    subkey.zeroize();
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let aad = format!("CWLC1:{uid}:{ref_id}");
    let ct = cipher?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plain_bytes,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "local_cache_encrypt_failed".to_string())?;
    let mut out = Vec::with_capacity(5 + 12 + ct.len());
    out.extend_from_slice(LOCAL_CACHE_MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

#[allow(deprecated)]
fn unwrap_local_e2ee_cache(ref_id: &str, stored_bytes: Vec<u8>) -> Option<Vec<u8>> {
    if stored_bytes.len() < 5 + 12 + 16 || &stored_bytes[0..5] != LOCAL_CACHE_MAGIC {
        // Legacy unencrypted cache item (non-E2EE or pre-Spec 058): return as-is.
        return Some(stored_bytes);
    }
    let uid = api::auth_me_cached_id()?;
    let keys = e2ee::get_user_keys(Some(&uid))?;
    let mut subkey = keys.derive_subkey(&format!("local-media-cache-v1:{uid}:{ref_id}")).ok()?;
    let cipher = Aes256Gcm::new_from_slice(&subkey).ok();
    subkey.zeroize();
    let nonce = &stored_bytes[5..17];
    let ct = &stored_bytes[17..];
    let aad = format!("CWLC1:{uid}:{ref_id}");
    cipher?
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ct,
                aad: aad.as_bytes(),
            },
        )
        .ok()
}

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
        if let Some((cached_bytes, cached_mime, _sha)) = get_local_object(rid).await {
            if let Some(plain_bytes) = unwrap_local_e2ee_cache(rid, cached_bytes) {
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
            let decrypted_bytes = if !key_b64.is_empty() {
                match blob::open(&raw_bytes, key_b64, nonce_b64) {
                    Ok(p) => p,
                    Err(e) => return MediaFetchOutcome::Error(e),
                }
            } else {
                raw_bytes
            };

            // Transparently decompress `CWCMP1` smart lossless file envelope (with BLAKE3 & bomb guard!)
            let plain_bytes: Vec<u8> = if decrypted_bytes.starts_with(chatw_media_core::files::COMP_ENVELOPE_MAGIC) {
                match chatw_media_core::files::decompress_file_verified(&decrypted_bytes, 256 * 1024 * 1024) {
                    Ok((p, _codec)) => p,
                    Err(e) => return MediaFetchOutcome::Error(e.to_string()),
                }
            } else {
                decrypted_bytes
            };

            let sha = sha256_hex(&plain_bytes);
            if let Some(ref rid) = ref_id {
                // Encrypt E2EE/DM media at rest in local OPFS/IndexedDB/Native CAS so plaintext never sits on disk.
                let bytes_to_cache = if !key_b64.is_empty() || scope_type == "dm" {
                    wrap_local_e2ee_cache(rid, &plain_bytes).ok()
                } else {
                    Some(plain_bytes.clone())
                };
                if let Some(to_store) = bytes_to_cache {
                    let stored_sha = sha256_hex(&to_store);
                    let meta = LocalObjectMeta {
                        ref_id: rid.clone(),
                        scope_type: scope_type.to_string(),
                        file_name: format!("media_{rid}"),
                        mime_type: fallback_mime.to_string(),
                        sha256_plain: Some(stored_sha),
                        availability_state: if scope_type == "dm" { "replicated_multi_device".into() } else { "server_durable".into() },
                    };
                    if let Ok(local_tier) = put_local_object(&meta, &to_store).await {
                        let _ = storage_sync::ack_relay_copy(rid, &local_tier, &sha, variant == "original").await;
                    }
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
                        "The file has been deleted by the sender for everyone".to_string()
                    } else if avail == "no_remaining_copies" {
                        "No more copies exist on your or your opponent's computer".to_string()
                    } else {
                        "There is no local copy yet; may require recovery from another device".to_string()
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
    let Some((plain_bytes, _mime, _sha)) = get_local_object(ref_id).await else {
        // Honestly report when this device does NOT have the file; never pretend!
        let _ = storage_sync::respond_recovery_request(
            request_id,
            "no_copy",
            None,
            None,
            Some("This device also no longer has a local backup"),
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

    // Encrypted Server Relay fallback: ALWAYS seal payload with AEAD (`is_e2ee = true`) before uploading.
    // Never leak plaintext bytes through the server relay.
    let plain_for_relay = unwrap_local_e2ee_cache(ref_id, plain_bytes).unwrap_or_default();
    if plain_for_relay.is_empty() {
        return;
    }
    let Ok(sealed) = blob::seal(&plain_for_relay) else {
        return;
    };
    if let Ok(res) = storage_sync::upload_attachment_scoped(
        sealed.cipher,
        "application/octet-stream",
        "dm",
        None,
        Some(&format!("recovery_{ref_id}.enc")),
        Some("application/octet-stream"),
        None,
        true,
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
