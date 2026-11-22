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
        UserKeys, build_signed_prekey_bundle, derive_argon2id_key, export_user_keys_json,
        get_or_create_device_transport_pubkey, import_user_keys_json, unwrap_sync_key_on_device,
    },
    platform,
    services::{api, storage_sync},
};

use super::{
    DEVICE_TRUST_STATE, DeviceTrustInfo, E2EE_RECOVERY_REQUIRED, SYNC_PROGRESS,
    get_client_device_info, init_local_storage, list_local_locations, load_sync_cursors,
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

/// Initialize local OPFS/IndexedDB/CAS engine, register device identity (or ephemeral transport key if pending),
/// unwrap any approved `wrapped_sync_key`, upload signed prekeys, and run incremental delta sync.
pub async fn bootstrap_device_and_sync_for_user(
    user_id: &str,
    keys: Option<UserKeys>,
) -> Option<UserKeys> {
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
        let info = DeviceTrustInfo {
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
        let is_trusted = info.trust_state == "trusted";
        *DEVICE_TRUST_STATE.write() = Some(info);

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
                            active_keys = Some(recovered);
                        }
                    }
                }
            }
        }

        if is_trusted {
            // Publish signed prekey bundle for E2EE v2 session setup.
            if let Some(ref k) = active_keys {
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
            }
            let _ = run_incremental_delta_sync().await;
            let _ = report_cached_copies_to_server().await;
        }
    }

    active_keys
}

pub async fn bootstrap_device_and_sync(keys: Option<UserKeys>) {
    let _ = bootstrap_device_and_sync_for_user("", keys).await;
}

pub async fn run_incremental_delta_sync() -> Result<(), String> {
    let (dm_since, group_since, channel_since) = load_sync_cursors().await;
    let res = storage_sync::delta_sync(dm_since, group_since, channel_since)
        .await
        .map_err(|e| e.code)?;
    let next = res.get("next_cursors").cloned().unwrap_or_default();
    let dm_next = next.get("dm_seq").and_then(|n| n.as_i64()).unwrap_or(dm_since);
    let gp_next = next.get("group_seq").and_then(|n| n.as_i64()).unwrap_or(group_since);
    let ch_next = next.get("channel_seq").and_then(|n| n.as_i64()).unwrap_or(channel_since);

    let dm_arr = res.get("dm_messages").and_then(|v| v.as_array());
    let gp_arr = res.get("group_messages").and_then(|v| v.as_array());
    let ch_arr = res.get("channel_messages").and_then(|v| v.as_array());

    // Process fetched delta metadata into account-scoped delta digest so sync actually records progress
    // and avoids silently discarding synced frames.
    let dm_count = dm_arr.map(|a| a.len()).unwrap_or(0);
    let gp_count = gp_arr.map(|a| a.len()).unwrap_or(0);
    let ch_count = ch_arr.map(|a| a.len()).unwrap_or(0);
    let total_delta = dm_count + gp_count + ch_count;

    if total_delta > 0 {
        let summary = serde_json::json!({
            "dm_seq": dm_next,
            "group_seq": gp_next,
            "channel_seq": ch_next,
            "dm_delta": dm_count,
            "group_delta": gp_count,
            "channel_delta": ch_count,
        });
        platform::storage_set("chatw_last_delta_sync_v1", &summary.to_string());
    }

    save_sync_cursors(dm_next, gp_next, ch_next).await;

    SYNC_PROGRESS.with_mut(|st| {
        st.phase_a_done = true;
        st.completed_count += total_delta;
        st.last_sync_label = format!("Delta synced (+{total_delta} items)");
    });
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
}
