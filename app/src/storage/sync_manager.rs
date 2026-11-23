use std::collections::{HashMap, HashSet};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
#[allow(unused_imports)]
use dioxus::prelude::{ReadableExt, WritableExt};
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::{
    crypto::e2ee::{
        PeerKeys, UserKeys, build_signed_prekey_bundle, decrypt_chat_message, derive_argon2id_key,
        export_user_keys_json, get_or_create_device_transport_pubkey,
        get_or_verify_peer_keys_scoped, get_user_keys, import_user_keys_json,
        unwrap_sync_key_on_device, wrap_sync_key_for_device,
    },
    models::{DisplayMessage, MessageStatus},
    platform,
    services::{api, storage_sync},
};

use super::{
    DEVICE_TRUST_STATE, DeviceTrustInfo, E2EE_RECOVERY_REQUIRED, IncomingDeviceApproval,
    PENDING_APPROVAL_ALERT, SYNC_PROGRESS, get_client_device_info, init_local_storage,
    list_local_locations, load_cached_messages, load_sync_cursors, save_cached_messages,
    save_sync_cursors,
};

const BACKUP_AAD_V2: &[u8] = b"chatw-key-backup:argon2id-v1";
pub const BACKUP_ARGON2_MEM_KIB: u32 = 19_456;
pub const BACKUP_ARGON2_TIME_COST: u32 = 2;
pub const BACKUP_ARGON2_PARALLELISM: u32 = 1;

/// Legacy iterated SHA-256 fallback ONLY for decrypting historical backups created before Spec 058.
fn derive_legacy_sha256_backup_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut state = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(passphrase.as_bytes());
    state.copy_from_slice(&hasher.finalize());
    let rounds = iterations.clamp(10_000, 100_000);
    for r in 0..rounds {
        let mut h = Sha256::new();
        h.update(&state);
        h.update(&r.to_le_bytes());
        h.update(salt);
        state.copy_from_slice(&h.finalize());
    }
    state
}

/// Encrypt account E2EE keys for server backup using Argon2id (m=19456 KiB, t=2, p=1) + AES-256-GCM + AAD.
#[allow(deprecated)]
pub fn encrypt_keys_for_backup(
    keys: &UserKeys,
    passphrase: &str,
) -> Result<(String, String, i32), String> {
    if passphrase.chars().count() < 6 {
        return Err("The recovery password needs at least 6 characters".into());
    }
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);

    let mut derived = derive_argon2id_key(
        passphrase,
        &salt,
        BACKUP_ARGON2_MEM_KIB,
        BACKUP_ARGON2_TIME_COST,
        BACKUP_ARGON2_PARALLELISM,
    )?;
    let mut plain = export_user_keys_json(keys);
    let cipher_inst =
        Aes256Gcm::new_from_slice(&derived).map_err(|_| "cipher_init_failed".to_string());
    derived.zeroize();
    let cipher = cipher_inst?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plain.as_bytes(),
                aad: BACKUP_AAD_V2,
            },
        )
        .map_err(|_| "encrypt_failed".to_string());
    plain.zeroize();
    let cipher = cipher?;

    let mut combined = Vec::with_capacity(12 + cipher.len());
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&cipher);
    Ok((
        format!("a2v1:{}", URL_SAFE_NO_PAD.encode(combined)),
        URL_SAFE_NO_PAD.encode(salt),
        BACKUP_ARGON2_TIME_COST as i32,
    ))
}

#[allow(deprecated)]
pub fn decrypt_keys_from_backup_v2(
    user_id: &str,
    wrapped_b64: &str,
    salt_b64: &str,
    kdf_alg: Option<&str>,
    kdf_mem_kib: Option<i32>,
    kdf_time_cost: Option<i32>,
    kdf_parallelism: Option<i32>,
    iterations: i32,
    passphrase: &str,
) -> Result<UserKeys, String> {
    let salt = URL_SAFE_NO_PAD
        .decode(salt_b64)
        .map_err(|_| "invalid_salt_b64".to_string())?;

    let is_argon2 = wrapped_b64.starts_with("a2v1:") || kdf_alg == Some("argon2id-v1");
    let raw_b64 = wrapped_b64.strip_prefix("a2v1:").unwrap_or(wrapped_b64);
    let combined = URL_SAFE_NO_PAD
        .decode(raw_b64)
        .map_err(|_| "invalid_wrapped_b64".to_string())?;
    if combined.len() < 28 {
        return Err("invalid_backup_length".into());
    }
    let (nonce, cipher_bytes) = combined.split_at(12);

    if is_argon2 {
        let mem = kdf_mem_kib.unwrap_or(BACKUP_ARGON2_MEM_KIB as i32) as u32;
        let time = kdf_time_cost.unwrap_or(BACKUP_ARGON2_TIME_COST as i32) as u32;
        let par = kdf_parallelism.unwrap_or(BACKUP_ARGON2_PARALLELISM as i32) as u32;
        let mut derived = derive_argon2id_key(passphrase, &salt, mem, time, par)?;
        let cipher_inst =
            Aes256Gcm::new_from_slice(&derived).map_err(|_| "cipher_init_failed".to_string());
        derived.zeroize();
        let mut plain = cipher_inst?
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: cipher_bytes,
                    aad: BACKUP_AAD_V2,
                },
            )
            .map_err(|_| "Wrong recovery password".to_string())?;
        let json_str = std::str::from_utf8(&plain)
            .map_err(|_| "invalid_utf8".to_string())?
            .to_string();
        plain.zeroize();
        return import_user_keys_json(user_id, &json_str);
    }

    // Legacy fallback: decrypt historical SHA-256 backup seamlessly without data loss.
    let mut derived = derive_legacy_sha256_backup_key(passphrase, &salt, iterations.max(10_000) as u32);
    let cipher_inst =
        Aes256Gcm::new_from_slice(&derived).map_err(|_| "cipher_init_failed".to_string());
    derived.zeroize();
    let mut plain = cipher_inst?
        .decrypt(Nonce::from_slice(nonce), cipher_bytes)
        .map_err(|_| "Wrong recovery password".to_string())?;
    let json_str = std::str::from_utf8(&plain)
        .map_err(|_| "invalid_utf8".to_string())?
        .to_string();
    plain.zeroize();
    import_user_keys_json(user_id, &json_str)
}

pub fn decrypt_keys_from_backup(
    user_id: &str,
    wrapped_b64: &str,
    salt_b64: &str,
    iterations: i32,
    passphrase: &str,
) -> Result<UserKeys, String> {
    decrypt_keys_from_backup_v2(
        user_id,
        wrapped_b64,
        salt_b64,
        None,
        None,
        None,
        None,
        iterations,
        passphrase,
    )
}

#[derive(Clone, Debug, Default)]
pub struct DeltaSyncOutcome {
    pub total_decrypted: usize,
    pub updated_previews: HashMap<String, String>,
    pub touched_partners: HashSet<String>,
}

/// Merge fresh decrypted DM messages into an existing cached/in-memory conversation list.
/// Heals any stale `MessageStatus::Error` ("Cannot read on this device") entries in-place
/// once valid E2EE keys arrive, and refreshes metadata (pin/reactions/text) without duplicates.
pub fn merge_healed_dm_messages(
    existing: &mut Vec<DisplayMessage>,
    incoming: Vec<DisplayMessage>,
) -> bool {
    let mut index_by_id: HashMap<String, usize> = existing
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.clone(), i))
        .collect();
    let mut changed = false;

    for m in incoming {
        if let Some(&idx) = index_by_id.get(&m.id) {
            let cur = &mut existing[idx];
            if cur.status == MessageStatus::Error && m.status == MessageStatus::Sent {
                *cur = m;
                changed = true;
            } else if m.status == MessageStatus::Sent {
                let mut item_changed = false;
                if cur.is_pinned != m.is_pinned {
                    cur.is_pinned = m.is_pinned;
                    item_changed = true;
                }
                if !m.reactions.is_empty() && cur.reactions != m.reactions {
                    cur.reactions = m.reactions;
                    item_changed = true;
                }
                if !m.text.is_empty() && cur.text != m.text {
                    cur.text = m.text;
                    item_changed = true;
                }
                if item_changed {
                    changed = true;
                }
            }
        } else {
            let new_idx = existing.len();
            index_by_id.insert(m.id.clone(), new_idx);
            existing.push(m);
            changed = true;
        }
    }

    if changed {
        existing.sort_by_key(|m| m.seq);
    }
    changed
}

/// Called by an already-trusted device with valid `UserKeys` on boot/reconnect.
/// When automatic device sync is enabled (`!require_device_approval`, the default),
/// automatically wraps `my_keys` via P-256 ECDH + AES-256-GCM + ECDSA to every pending
/// new device's ephemeral transport key so newly logged-in devices decrypt all messages
/// automatically without server access to plaintext keys.
pub async fn auto_fulfill_pending_device_syncs(my_keys: &UserKeys) -> usize {
    let Ok(resp) = storage_sync::list_devices().await else {
        return 0;
    };
    if resp.pending_requests.is_empty() {
        return 0;
    }
    let my_did = DEVICE_TRUST_STATE.peek().as_ref().map(|d| d.device_id.clone());

    if resp.require_device_approval {
        if PENDING_APPROVAL_ALERT.peek().is_none() {
            if let Some(first) = resp
                .pending_requests
                .iter()
                .find(|r| my_did.as_deref() != Some(r.requesting_device_id.as_str()))
            {
                *PENDING_APPROVAL_ALERT.write() = Some(IncomingDeviceApproval {
                    request_id: first.id.clone(),
                    requesting_device_id: first.requesting_device_id.clone(),
                    device_name: first
                        .device_label
                        .clone()
                        .unwrap_or_else(|| first.platform.clone()),
                    platform: first.platform.clone(),
                    verification_code: first.verification_code.clone(),
                    ecdh_public_b64: first.ecdh_public_b64.clone(),
                });
            }
        }
        return 0;
    }

    let mut fulfilled = 0usize;
    for req in resp.pending_requests {
        if my_did.as_deref() == Some(req.requesting_device_id.as_str()) {
            continue;
        }
        let Some(pub_b64) = req.ecdh_public_b64.as_deref().filter(|s| !s.is_empty()) else {
            continue;
        };
        let Ok(wrapped) = wrap_sync_key_for_device(my_keys, pub_b64, &req.id) else {
            continue;
        };
        if storage_sync::approve_device_request(
            &req.id,
            Some(&req.verification_code),
            Some(&wrapped),
        )
        .await
        .is_ok()
        {
            fulfilled += 1;
        }
    }
    fulfilled
}

/// Initialize local OPFS/IndexedDB/CAS engine, register device identity (or ephemeral transport key if pending),
/// unwrap any approved `wrapped_sync_key`, auto-fulfill any pending sibling device sync requests,
/// upload signed prekeys, and run full decrypted incremental delta sync across all conversations.
pub async fn bootstrap_device_and_sync_for_user(
    user_id: &str,
    keys: Option<UserKeys>,
) -> (Option<UserKeys>, DeltaSyncOutcome) {
    let (cid, plat, dname) = match init_local_storage().await {
        Ok(tuple) => tuple,
        Err(_) => get_client_device_info().await,
    };

    // If this device doesn't have the account's E2EE keys yet, create an ephemeral device transport
    // ECDH public key so an existing trusted device can wrap the account's keys into `wrapped_sync_key`.
    let transport_pub = if keys.is_none() && !user_id.is_empty() {
        get_or_create_device_transport_pubkey(user_id).ok()
    } else {
        None
    };

    let (ecdh, ecdsa, fp) = match keys.as_ref() {
        Some(k) => (
            Some(k.ecdh_raw_b64.clone()),
            Some(k.ecdsa_raw_b64.clone()),
            Some(k.fingerprint_b64.clone()),
        ),
        None => (transport_pub, None, None),
    };

    let mut active_keys = keys;
    let mut recovered_now = false;
    let mut outcome = DeltaSyncOutcome::default();

    if let Ok(resp) = storage_sync::register_device(
        &cid,
        &plat,
        &dname,
        ecdh.as_deref(),
        ecdsa.as_deref(),
        fp.as_deref(),
    )
    .await
    {
        let mut info = DeviceTrustInfo {
            device_id: resp
                .get("device_id")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            client_device_id: cid,
            trust_state: resp
                .get("trust_state")
                .and_then(|s| s.as_str())
                .unwrap_or("trusted")
                .to_string(),
            verification_code: resp
                .get("verification_code")
                .and_then(|s| s.as_str())
                .map(str::to_string),
            request_id: resp
                .get("request_id")
                .and_then(|s| s.as_str())
                .map(str::to_string),
        };
        let mut is_trusted = info.trust_state == "trusted";
        *DEVICE_TRUST_STATE.write() = Some(info.clone());

        // If this device was approved while offline/reloading and now has a wrapped_sync_key waiting,
        // unwrap and import the account's E2EE keys automatically.
        if active_keys.is_none() && !user_id.is_empty() {
            if let Ok(cur) = storage_sync::current_device_status().await {
                if let Some(lr) = cur.get("latest_request") {
                    let st = lr.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let req_id = lr.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    if st == "approved"
                        && let Some(wsk) = lr.get("wrapped_sync_key").and_then(|v| v.as_str())
                        && !wsk.is_empty()
                    {
                        let expected_ecdsa = api::keys_get(user_id)
                            .await
                            .ok()
                            .map(|k| k.ecdsa_public);
                        if let Ok(recovered) = unwrap_sync_key_on_device(
                            user_id,
                            wsk,
                            req_id,
                            expected_ecdsa.as_deref(),
                        ) {
                            *E2EE_RECOVERY_REQUIRED.write() = None;
                            info.trust_state = "trusted".into();
                            is_trusted = true;
                            *DEVICE_TRUST_STATE.write() = Some(info);
                            active_keys = Some(recovered);
                            recovered_now = true;
                        }
                    }
                }
            }
        }

        if is_trusted {
            if let Some(ref k) = active_keys {
                // Publish signed prekey bundle for E2EE v2 session setup.
                let spk_marker = format!("chatw_spk_uploaded_{}", k.fingerprint_b64);
                if platform::storage_get(&spk_marker).as_deref() != Some("1") {
                    if let Ok((spk_pub, spk_sig, otks)) = build_signed_prekey_bundle(k, 16) {
                        if api::keys_upload_prekeys(1, &spk_pub, &spk_sig, &otks)
                            .await
                            .is_ok()
                        {
                            platform::storage_set(&spk_marker, "1");
                        }
                    }
                }
                // Auto-fulfill any sibling device waiting for E2EE key sync (blind relay).
                let _ = auto_fulfill_pending_device_syncs(k).await;

                // Decrypt and store all missed / historical messages into IndexedDB + previews.
                if let Ok(delta) =
                    run_incremental_delta_sync_for_user(user_id, Some(k), recovered_now).await
                {
                    outcome = delta;
                }
            }
            let _ = report_cached_copies_to_server().await;
        }
    }

    (active_keys, outcome)
}

pub async fn bootstrap_device_and_sync(keys: Option<UserKeys>) {
    let _ = bootstrap_device_and_sync_for_user("", keys).await;
}

/// Pulls paginated encrypted deltas from `/api/sync/delta`, decrypts all DM & Self-Chat envelopes
/// client-side using `UserKeys`, heals & persists every conversation in IndexedDB, and returns
/// updated sidebar previews so all conversations synchronize seamlessly on login.
pub async fn run_incremental_delta_sync_for_user(
    user_id: &str,
    keys: Option<&UserKeys>,
    full_resync: bool,
) -> Result<DeltaSyncOutcome, String> {
    let Some(my_keys) = keys else {
        // Never advance DM sync cursor before E2EE keys are unlocked on this device.
        return Ok(DeltaSyncOutcome::default());
    };
    if user_id.is_empty() {
        return Ok(DeltaSyncOutcome::default());
    }

    let (mut dm_since, mut group_since, mut channel_since) = if full_resync {
        (0, 0, 0)
    } else {
        load_sync_cursors().await
    };

    let mut peer_cache: HashMap<String, PeerKeys> = HashMap::new();
    let mut grouped_dms: HashMap<String, Vec<DisplayMessage>> = HashMap::new();
    let mut total_delta = 0usize;
    let mut total_dm_count = 0usize;
    let mut total_gp_count = 0usize;
    let mut total_ch_count = 0usize;

    for _page in 0..20 {
        let res = storage_sync::delta_sync_with_limit(dm_since, group_since, channel_since, 250)
            .await
            .map_err(|e| e.code)?;
        let next = res.get("next_cursors").cloned().unwrap_or_default();
        let dm_next = next.get("dm_seq").and_then(|n| n.as_i64()).unwrap_or(dm_since);
        let gp_next = next.get("group_seq").and_then(|n| n.as_i64()).unwrap_or(group_since);
        let ch_next = next.get("channel_seq").and_then(|n| n.as_i64()).unwrap_or(channel_since);

        let dm_arr = res.get("dm_messages").and_then(|v| v.as_array());
        let gp_arr = res.get("group_messages").and_then(|v| v.as_array());
        let ch_arr = res.get("channel_messages").and_then(|v| v.as_array());

        let dm_count = dm_arr.map(|a| a.len()).unwrap_or(0);
        let gp_count = gp_arr.map(|a| a.len()).unwrap_or(0);
        let ch_count = ch_arr.map(|a| a.len()).unwrap_or(0);
        total_dm_count += dm_count;
        total_gp_count += gp_count;
        total_ch_count += ch_count;
        total_delta += dm_count + gp_count + ch_count;

        if let Some(items) = dm_arr {
            for item in items {
                if item
                    .get("deleted_for_all")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
                {
                    continue;
                }
                let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let seq = item.get("seq").and_then(|v| v.as_i64()).unwrap_or(0);
                let Some(sender_id) = item.get("sender_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let Some(recipient_id) = item.get("recipient_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let Some(envelope) = item.get("envelope").and_then(|v| v.as_str()) else {
                    continue;
                };
                let created_at = item
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let reply_to_id = item
                    .get("reply_to_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let is_pinned = item
                    .get("is_pinned")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let is_self = sender_id == user_id;
                let partner_id = if is_self {
                    recipient_id.to_string()
                } else {
                    sender_id.to_string()
                };

                let sender_ecdsa: [u8; 65] = if is_self {
                    my_keys.ecdsa_raw
                } else {
                    if !peer_cache.contains_key(&partner_id) {
                        if let Ok(pk_resp) = api::keys_get(&partner_id).await
                            && let Ok(pk) = get_or_verify_peer_keys_scoped(
                                Some(user_id),
                                &pk_resp.user_id,
                                &pk_resp.ecdh_public,
                                &pk_resp.ecdsa_public,
                                &pk_resp.fingerprint,
                            )
                        {
                            peer_cache.insert(partner_id.clone(), pk);
                        }
                    }
                    let Some(pk) = peer_cache.get(&partner_id) else {
                        continue;
                    };
                    pk.ecdsa_raw
                };

                if let Ok(dec) = decrypt_chat_message(envelope, my_keys, &sender_ecdsa) {
                    grouped_dms.entry(partner_id).or_default().push(DisplayMessage {
                        id: id.to_string(),
                        seq,
                        sender_id: sender_id.to_string(),
                        recipient_id: recipient_id.to_string(),
                        text: dec.text,
                        created_at,
                        is_self,
                        status: MessageStatus::Sent,
                        reply_to_id,
                        reactions: Vec::new(),
                        is_pinned,
                    });
                }
            }
        }

        let progressed = dm_next > dm_since || gp_next > group_since || ch_next > channel_since;
        dm_since = dm_next;
        group_since = gp_next;
        channel_since = ch_next;

        let has_more = res
            .get("has_more")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        if !has_more || !progressed {
            break;
        }
    }

    let mut outcome = DeltaSyncOutcome::default();
    for (partner_id, mut msgs) in grouped_dms {
        msgs.sort_by_key(|m| m.seq);
        let decrypted_count = msgs.len();
        outcome.total_decrypted += decrypted_count;
        if let Some(last) = msgs.last() {
            outcome
                .updated_previews
                .insert(partner_id.clone(), last.text.clone());
        }
        outcome.touched_partners.insert(partner_id.clone());

        let idb_key = format!("dm:{user_id}:{partner_id}");
        let (mut cached, has_more) = load_cached_messages::<DisplayMessage>(&idb_key)
            .await
            .unwrap_or_else(|| (Vec::new(), decrypted_count >= 30));
        if merge_healed_dm_messages(&mut cached, msgs) {
            save_cached_messages(&idb_key, &cached, has_more).await;
        }
    }

    if total_delta > 0 {
        let summary = serde_json::json!({
            "dm_seq": dm_since,
            "group_seq": group_since,
            "channel_seq": channel_since,
            "dm_delta": total_dm_count,
            "group_delta": total_gp_count,
            "channel_delta": total_ch_count,
            "decrypted": outcome.total_decrypted,
        });
        platform::storage_set("chatw_last_delta_sync_v1", &summary.to_string());
    }

    save_sync_cursors(dm_since, group_since, channel_since).await;

    SYNC_PROGRESS.with_mut(|st| {
        st.phase_a_done = true;
        st.completed_count += total_delta;
        st.last_sync_label = format!("Delta synced (+{} messages)", outcome.total_decrypted);
    });
    Ok(outcome)
}

pub async fn run_incremental_delta_sync() -> Result<(), String> {
    let uid = api::auth_me_cached_id().unwrap_or_default();
    let keys = if uid.is_empty() {
        None
    } else {
        get_user_keys(Some(&uid))
    };
    let _ = run_incremental_delta_sync_for_user(&uid, keys.as_ref(), false).await?;
    Ok(())
}

pub async fn report_cached_copies_to_server() -> Result<usize, String> {
    let items = list_local_locations().await;
    if items.is_empty() {
        return Ok(0);
    }
    let resp = storage_sync::report_locations(items).await.map_err(|e| e.code)?;
    Ok(resp.get("reported").and_then(|n| n.as_u64()).unwrap_or(0) as usize)
}

pub fn toggle_phase_b_pause() {
    SYNC_PROGRESS.with_mut(|st| {
        st.phase_b_paused = !st.phase_b_paused;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_key_backup_argon2id_roundtrips_and_rejects_wrong_passphrase() {
        let keys = UserKeys::generate();
        let (wrapped, salt, iters) = encrypt_keys_for_backup(&keys, "correct horse battery").unwrap();
        assert!(wrapped.starts_with("a2v1:"));
        let recovered =
            decrypt_keys_from_backup("user-1", &wrapped, &salt, iters, "correct horse battery")
                .unwrap();
        assert_eq!(keys.fingerprint, recovered.fingerprint);
        assert_eq!(keys.root_key, recovered.root_key);
        assert!(decrypt_keys_from_backup("user-1", &wrapped, &salt, iters, "wrong-pass").is_err());
    }

    #[test]
    fn merge_healed_dm_messages_replaces_cannot_read_error_bubbles_with_decrypted_text() {
        let mut cached = vec![
            DisplayMessage {
                id: "m1".into(),
                seq: 1,
                sender_id: "u1".into(),
                recipient_id: "u1".into(),
                text: "Cannot read on this device".into(),
                created_at: "2026-11-23T10:00:00Z".into(),
                is_self: true,
                status: MessageStatus::Error,
                reply_to_id: None,
                reactions: vec![],
                is_pinned: false,
            },
            DisplayMessage {
                id: "m2".into(),
                seq: 2,
                sender_id: "u1".into(),
                recipient_id: "u1".into(),
                text: "Already decrypted".into(),
                created_at: "2026-11-23T10:01:00Z".into(),
                is_self: true,
                status: MessageStatus::Sent,
                reply_to_id: None,
                reactions: vec![],
                is_pinned: false,
            },
        ];
        let incoming = vec![
            DisplayMessage {
                id: "m1".into(),
                seq: 1,
                sender_id: "u1".into(),
                recipient_id: "u1".into(),
                text: "Secret synced note".into(),
                created_at: "2026-11-23T10:00:00Z".into(),
                is_self: true,
                status: MessageStatus::Sent,
                reply_to_id: None,
                reactions: vec![],
                is_pinned: true,
            },
            DisplayMessage {
                id: "m3".into(),
                seq: 3,
                sender_id: "u1".into(),
                recipient_id: "u1".into(),
                text: "Brand new message".into(),
                created_at: "2026-11-23T10:02:00Z".into(),
                is_self: true,
                status: MessageStatus::Sent,
                reply_to_id: None,
                reactions: vec![],
                is_pinned: false,
            },
        ];
        assert!(merge_healed_dm_messages(&mut cached, incoming));
        assert_eq!(cached.len(), 3);
        assert_eq!(cached[0].text, "Secret synced note");
        assert_eq!(cached[0].status, MessageStatus::Sent);
        assert!(cached[0].is_pinned);
        assert_eq!(cached[1].text, "Already decrypted");
        assert_eq!(cached[2].text, "Brand new message");
    }
}
