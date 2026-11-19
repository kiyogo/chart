use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
#[allow(unused_imports)]
use dioxus::prelude::{ReadableExt, WritableExt};
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::{
    crypto::e2ee::{UserKeys, export_user_keys_json, import_user_keys_json},
    services::storage_sync,
};

use super::{
    DEVICE_TRUST_STATE, DeviceTrustInfo, SYNC_PROGRESS, get_client_device_info,
    init_local_storage, list_local_locations, load_sync_cursors, save_sync_cursors,
};

/// Derive 256-bit key cleanly across WASM & Native without extra PBKDF crates.
fn derive_backup_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
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

#[allow(deprecated)]
pub fn encrypt_keys_for_backup(keys: &UserKeys, passphrase: &str) -> Result<(String, String, i32), String> {
    if passphrase.chars().count() < 6 {
        return Err("Mật khẩu khôi phục cần ít nhất 6 ký tự".into());
    }
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let iterations = 50_000i32;
    let derived = derive_backup_key(passphrase, &salt, iterations as u32);
    let plain = export_user_keys_json(keys);
    let cipher = Aes256Gcm::new_from_slice(&derived)
        .map_err(|_| "cipher_init_failed")?
        .encrypt(Nonce::from_slice(&nonce), plain.as_bytes())
        .map_err(|_| "encrypt_failed")?;
    let mut combined = Vec::with_capacity(12 + cipher.len());
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&cipher);
    Ok((URL_SAFE_NO_PAD.encode(combined), URL_SAFE_NO_PAD.encode(salt), iterations))
}

#[allow(deprecated)]
pub fn decrypt_keys_from_backup(
    user_id: &str,
    wrapped_b64: &str,
    salt_b64: &str,
    iterations: i32,
    passphrase: &str,
) -> Result<UserKeys, String> {
    let combined = URL_SAFE_NO_PAD.decode(wrapped_b64).map_err(|_| "invalid_wrapped_b64")?;
    let salt = URL_SAFE_NO_PAD.decode(salt_b64).map_err(|_| "invalid_salt_b64")?;
    if combined.len() < 16 {
        return Err("invalid_backup_length".into());
    }
    let (nonce, cipher) = combined.split_at(12);
    let derived = derive_backup_key(passphrase, &salt, iterations.max(10_000) as u32);
    let plain = Aes256Gcm::new_from_slice(&derived)
        .map_err(|_| "cipher_init_failed")?
        .decrypt(Nonce::from_slice(nonce), cipher)
        .map_err(|_| "Sai mật khẩu khôi phục".to_string())?;
    let json_str = std::str::from_utf8(&plain).map_err(|_| "invalid_utf8")?;
    import_user_keys_json(user_id, json_str)
}

/// Initialize local OPFS/IndexedDB/CAS engine, register device identity with server,
/// and kick off Phase A incremental cursor sync + local copy reporting.
pub async fn bootstrap_device_and_sync(keys: Option<UserKeys>) {
    let (cid, plat, dname) = match init_local_storage().await {
        Ok(tuple) => tuple,
        Err(_) => get_client_device_info().await,
    };

    let (ecdh, ecdsa, fp) = match keys.as_ref() {
        Some(k) => (Some(k.ecdh_raw_b64.as_str()), Some(k.ecdsa_raw_b64.as_str()), Some(k.fingerprint_b64.as_str())),
        None => (None, None, None),
    };

    if let Ok(resp) = storage_sync::register_device(&cid, &plat, &dname, ecdh, ecdsa, fp).await {
        let info = DeviceTrustInfo {
            device_id: resp.get("device_id").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
            client_device_id: cid,
            trust_state: resp.get("trust_state").and_then(|s| s.as_str()).unwrap_or("trusted").to_string(),
            verification_code: resp.get("verification_code").and_then(|s| s.as_str()).map(str::to_string),
            request_id: resp.get("request_id").and_then(|s| s.as_str()).map(str::to_string),
        };
        let is_trusted = info.trust_state == "trusted";
        *DEVICE_TRUST_STATE.write() = Some(info);
        if is_trusted {
            let _ = run_incremental_delta_sync().await;
            let _ = report_cached_copies_to_server().await;
        }
    }
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
    save_sync_cursors(dm_next, gp_next, ch_next).await;

    let total_delta = res.get("dm_messages").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
        + res.get("group_messages").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
        + res.get("channel_messages").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    SYNC_PROGRESS.with_mut(|st| {
        st.phase_a_done = true;
        st.completed_count += total_delta;
        st.last_sync_label = format!("Đã đồng bộ delta (+{total_delta} mục)");
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
    fn encrypted_key_backup_roundtrips_and_rejects_wrong_passphrase() {
        let keys = UserKeys::generate();
        let (wrapped, salt, iters) = encrypt_keys_for_backup(&keys, "correct horse battery").unwrap();
        let recovered = decrypt_keys_from_backup("user-1", &wrapped, &salt, iters, "correct horse battery").unwrap();
        assert_eq!(keys.fingerprint, recovered.fingerprint);
        assert!(decrypt_keys_from_backup("user-1", &wrapped, &salt, iters, "wrong-pass").is_err());
    }
}
