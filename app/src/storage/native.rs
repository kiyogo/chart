use chatw_media_core::cache::{
    CacheCategory, CacheItemMeta, IntelligentCacheIndex, compute_adaptive_cache_budget,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{LocalObjectMeta, LocalQuotaInfo, OutboxEntry, sha256_hex};

static INDEX_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NativeEntry {
    ref_id: String,
    scope_type: String,
    file_name: String,
    mime_type: String,
    byte_size: u64,
    sha256_plain: String,
    availability_state: String,
    last_accessed_at: u64,
    #[serde(default)]
    source_path: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct NativeStoreState {
    entries: HashMap<String, NativeEntry>,
    #[serde(default)]
    smart_index: IntelligentCacheIndex,
}

fn current_account_part() -> String {
    crate::services::api::auth_me_cached_id()
        .map(|uid| {
            uid.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect()
        })
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

fn cas_root() -> PathBuf {
    // Platform-appropriate cache directory (keeps rebuildable cache out of iOS iCloud backups!)
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let base = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Library").join("Caches"))
        .unwrap_or_else(|_| std::env::temp_dir());
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    let base = std::env::var("XDG_CACHE_HOME")
        .or_else(|_| std::env::var("LOCALAPPDATA"))
        .or_else(|_| std::env::var("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
                .unwrap_or_else(|_| std::env::temp_dir())
        });
    base.join("chatw").join("cas").join(current_account_part())
}

pub fn native_cache_dir_for_account(account_part: &str) -> PathBuf {
    let safe = if account_part.is_empty() {
        current_account_part()
    } else {
        account_part.to_string()
    };
    cas_root()
        .parent()
        .map(|p| p.join(safe))
        .unwrap_or_else(cas_root)
}

pub fn native_messages_dir_bytes() -> u64 {
    let dir = cas_root().join("messages");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return 0;
    };
    rd.flatten()
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn index_path() -> PathBuf {
    cas_root().join("index_v2.json")
}

fn legacy_index_path() -> PathBuf {
    cas_root().join("index.json")
}

fn cursors_path() -> PathBuf {
    cas_root().join("cursors.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn atomic_write_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4().simple()));
    std::fs::write(&tmp, bytes)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

fn safe_blob_name(ref_id: &str) -> String {
    ref_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn classify_cache_category(mime: &str, availability_state: &str) -> CacheCategory {
    if matches!(availability_state, "local_only" | "pending_delivery") {
        return CacheCategory::OriginalLocal;
    }
    if mime.starts_with("video/") {
        CacheCategory::VideoSegment
    } else if mime.starts_with("audio/") {
        CacheCategory::AudioVoice
    } else if mime.starts_with("image/") {
        CacheCategory::OptimizedMedia
    } else {
        CacheCategory::FileAttachment
    }
}

fn load_state() -> NativeStoreState {
    if let Ok(raw) = std::fs::read_to_string(index_path()) {
        if let Ok(state) = serde_json::from_str::<NativeStoreState>(&raw) {
            return state;
        }
    }
    // Seamless upgrade from pre-Spec 110 `index.json`
    let legacy: HashMap<String, NativeEntry> = std::fs::read_to_string(legacy_index_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut smart = IntelligentCacheIndex::new();
    let acct = current_account_part();
    for (k, e) in &legacy {
        let pinned = matches!(e.availability_state.as_str(), "local_only" | "pending_delivery");
        smart.entries.insert(
            k.clone(),
            CacheItemMeta {
                key: k.clone(),
                account_id: acct.clone(),
                category: classify_cache_category(&e.mime_type, &e.availability_state),
                byte_size: e.byte_size,
                created_at_secs: e.last_accessed_at,
                last_accessed_secs: e.last_accessed_at,
                access_count: 1,
                download_cost_ms: 80,
                ai_recompute_cost_ms: 0,
                ttl_secs: None,
                pinned_offline: pinned,
                in_use: false,
                stale_model_version: false,
                protected_segment: false,
            },
        );
    }
    NativeStoreState {
        entries: legacy,
        smart_index: smart,
    }
}

fn save_state(state: &NativeStoreState) {
    if let Ok(json) = serde_json::to_vec(state) {
        let _ = atomic_write_file(&index_path(), &json);
    }
}

fn configured_limit_mb() -> i32 {
    crate::platform::storage_get("chatw_cache_limit_mb")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1024)
}

pub async fn init_local_storage() -> Result<(String, String, String), String> {
    let _ = std::fs::create_dir_all(cas_root().join("blobs"));
    let _ = std::fs::create_dir_all(cas_root().join("ai_outputs"));
    Ok(get_client_device_info().await)
}

pub async fn get_client_device_info() -> (String, String, String) {
    let cid = match crate::platform::storage_get("chatw_client_device_id") {
        Some(v) if v.len() >= 8 => v,
        _ => {
            let fresh = uuid::Uuid::new_v4().to_string();
            crate::platform::storage_set("chatw_client_device_id", &fresh);
            fresh
        }
    };
    let platform = if cfg!(target_os = "android") || cfg!(target_os = "ios") {
        "mobile".to_string()
    } else {
        "desktop".to_string()
    };
    let os_label = std::env::consts::OS.to_string();
    (cid, platform, format!("Desktop · {os_label}"))
}

#[cfg(unix)]
fn get_real_available_disk_space(dir: &std::path::Path) -> u64 {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    if let Ok(c_path) = CString::new(dir.as_os_str().as_bytes()) {
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                let free = (stat.f_bavail as u64) * (stat.f_frsize as u64);
                if free > 0 {
                    return free;
                }
            }
        }
    }
    16 * 1024 * 1024 * 1024
}

#[cfg(windows)]
fn get_real_available_disk_space(dir: &std::path::Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            lpDirectoryName: *const u16,
            lpFreeBytesAvailableToCaller: *mut u64,
            lpTotalNumberOfBytes: *mut u64,
            lpTotalNumberOfFreeBytes: *mut u64,
        ) -> i32;
    }
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free_avail: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free_avail, &mut total, &mut total_free) };
    if ok != 0 && free_avail > 0 {
        return free_avail;
    }
    4 * 1024 * 1024 * 1024
}

#[cfg(not(any(unix, windows)))]
fn get_real_available_disk_space(_dir: &std::path::Path) -> u64 {
    4 * 1024 * 1024 * 1024
}

pub async fn put_local_object(meta: &LocalObjectMeta, bytes: &[u8]) -> Result<String, String> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let sha = sha256_hex(bytes);
    let safe_id = safe_blob_name(&meta.ref_id);
    let blob_path = cas_root().join("blobs").join(&safe_id);
    atomic_write_file(&blob_path, bytes).map_err(|e| e.to_string())?;

    let now = now_secs();
    let mut state = load_state();
    let pinned = matches!(meta.availability_state.as_str(), "local_only" | "pending_delivery");
    let item_meta = CacheItemMeta {
        key: meta.ref_id.clone(),
        account_id: current_account_part(),
        category: classify_cache_category(&meta.mime_type, &meta.availability_state),
        byte_size: bytes.len() as u64,
        created_at_secs: now,
        last_accessed_secs: now,
        access_count: 1,
        download_cost_ms: 100,
        ai_recompute_cost_ms: 0,
        ttl_secs: None,
        pinned_offline: pinned,
        in_use: false,
        stale_model_version: false,
        protected_segment: false,
    };

    // Adaptive W-TinyLFU + Cost-Aware & Size-Aware eviction dynamically querying real free disk space!
    let lim_mb = configured_limit_mb().max(0) as u64;
    let free_disk = get_real_available_disk_space(&cas_root());
    let budget = compute_adaptive_cache_budget(free_disk, Some(lim_mb));
    let evicted_keys = state.smart_index.put_and_plan_evictions(item_meta, &budget, now);
    for ek in evicted_keys {
        if ek != meta.ref_id {
            let _ = std::fs::remove_file(cas_root().join("blobs").join(safe_blob_name(&ek)));
            state.entries.remove(&ek);
        }
    }

    let prev_source_path = state.entries.get(&meta.ref_id).and_then(|e| e.source_path.clone());
    state.entries.insert(
        meta.ref_id.clone(),
        NativeEntry {
            ref_id: meta.ref_id.clone(),
            scope_type: meta.scope_type.clone(),
            file_name: meta.file_name.clone(),
            mime_type: meta.mime_type.clone(),
            byte_size: bytes.len() as u64,
            sha256_plain: meta.sha256_plain.clone().unwrap_or(sha),
            availability_state: meta.availability_state.clone(),
            last_accessed_at: now,
            source_path: prev_source_path,
        },
    );
    save_state(&state);
    Ok("native_fs".into())
}

pub async fn get_local_object(ref_id: &str) -> Option<(Vec<u8>, String, String)> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut state = load_state();
    let entry = state.entries.get_mut(ref_id)?;
    let safe_id = safe_blob_name(ref_id);
    let blob_path = cas_root().join("blobs").join(&safe_id);
    let bytes = match std::fs::read(&blob_path) {
        Ok(b) => b,
        Err(_) => {
            // Spec 163: If the app cache blob was evicted/missing on desktop/mobile, check if sender's
            // original local device file still exists with matching SHA-256 and silently re-cache it!
            let sp = entry.source_path.as_deref()?;
            let orig = std::fs::read(sp).ok()?;
            let orig_sha = sha256_hex(&orig);
            if !entry.sha256_plain.is_empty() && orig_sha != entry.sha256_plain {
                return None;
            }
            let _ = atomic_write_file(&blob_path, &orig);
            orig
        }
    };
    let actual_sha = sha256_hex(&bytes);
    if !entry.sha256_plain.is_empty() && actual_sha != entry.sha256_plain {
        return None;
    }
    let mime = entry.mime_type.clone();
    let now = now_secs();
    entry.last_accessed_at = now;
    state.smart_index.touch(ref_id, now);
    save_state(&state);
    Some((bytes, mime, actual_sha))
}

pub async fn delete_local_object(ref_id: &str, force_unique: bool) -> Result<u64, String> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut state = load_state();
    let Some(entry) = state.entries.get(ref_id).cloned() else {
        return Err("not_found".into());
    };
    if !force_unique && matches!(entry.availability_state.as_str(), "local_only" | "pending_delivery") {
        return Err("unique_local_file".into());
    }
    let _ = std::fs::remove_file(cas_root().join("blobs").join(safe_blob_name(ref_id)));
    state.entries.remove(ref_id);
    state.smart_index.entries.remove(ref_id);
    save_state(&state);
    Ok(entry.byte_size)
}

pub async fn clear_safe_cache(include_unique: bool) -> Result<(u64, usize), String> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut state = load_state();
    let mut freed = 0u64;
    let mut protected = 0usize;
    let keys: Vec<String> = state.entries.keys().cloned().collect();
    for k in keys {
        if let Some(entry) = state.entries.get(&k) {
            let unique = matches!(entry.availability_state.as_str(), "local_only" | "pending_delivery");
            if !include_unique && unique {
                protected += 1;
                continue;
            }
            let _ = std::fs::remove_file(cas_root().join("blobs").join(safe_blob_name(&k)));
            freed += entry.byte_size;
            state.entries.remove(&k);
            state.smart_index.entries.remove(&k);
        }
    }
    save_state(&state);
    Ok((freed, protected))
}

pub async fn get_quota_estimate() -> LocalQuotaInfo {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let state = load_state();
    let limit_mb = configured_limit_mb();
    let mut total = 0u64;
    let mut disposable = 0u64;
    let mut pinned = 0u64;
    for e in state.entries.values() {
        total += e.byte_size;
        if matches!(e.availability_state.as_str(), "local_only" | "pending_delivery") {
            pinned += e.byte_size;
        } else {
            disposable += e.byte_size;
        }
    }
    let real_free = get_real_available_disk_space(&cas_root());
    LocalQuotaInfo {
        limit_mb,
        browser_quota_bytes: real_free + total,
        browser_usage_bytes: total,
        persisted: true,
        opfs_supported: false,
        local_object_count: state.entries.len(),
        total_local_bytes: total,
        disposable_cache_bytes: disposable,
        pinned_unique_bytes: pinned,
    }
}

pub async fn list_local_locations() -> Vec<serde_json::Value> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    load_state()
        .entries
        .into_values()
        .map(|e| {
            serde_json::json!({
                "ref_id": e.ref_id,
                "local_tier": "native_fs",
                "sha256_hex": e.sha256_plain,
                "has_original": true,
            })
        })
        .collect()
}

pub async fn load_sync_cursors() -> (i64, i64, i64) {
    let Ok(raw) = std::fs::read_to_string(cursors_path()) else {
        return (0, 0, 0);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return (0, 0, 0);
    };
    (
        v.get("dm_seq").and_then(|n| n.as_i64()).unwrap_or(0),
        v.get("group_seq").and_then(|n| n.as_i64()).unwrap_or(0),
        v.get("channel_seq").and_then(|n| n.as_i64()).unwrap_or(0),
    )
}

pub async fn save_sync_cursors(dm_seq: i64, group_seq: i64, channel_seq: i64) {
    let payload = serde_json::json!({ "dm_seq": dm_seq, "group_seq": group_seq, "channel_seq": channel_seq });
    let _ = atomic_write_file(&cursors_path(), payload.to_string().as_bytes());
}

fn chat_cache_path(conv_key: &str) -> PathBuf {
    let safe: String = conv_key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    cas_root().join("messages").join(format!("{safe}.json"))
}

fn outbox_path() -> PathBuf {
    cas_root().join("outbox.json")
}

pub async fn load_cached_messages_json(conv_key: &str) -> Option<(serde_json::Value, bool)> {
    let raw = std::fs::read_to_string(chat_cache_path(conv_key)).ok()?;
    let uid = crate::services::api::auth_me_cached_id().unwrap_or_default();
    let decoded = if raw.starts_with("lenc:v1:") && !uid.is_empty() {
        crate::crypto::e2ee::decrypt_account_local_data(&uid, &format!("msg_cache:{conv_key}"), &raw).ok()?
    } else {
        raw
    };
    let v: serde_json::Value = serde_json::from_str(&decoded).ok()?;
    let items = v.get("items")?.clone();
    let has_more = v.get("has_more").and_then(|b| b.as_bool()).unwrap_or(false);
    Some((items, has_more))
}

pub async fn save_cached_messages_json(conv_key: &str, items: &serde_json::Value, has_more: bool) {
    let path = chat_cache_path(conv_key);
    let payload = serde_json::json!({ "items": items, "has_more": has_more }).to_string();
    let uid = crate::services::api::auth_me_cached_id().unwrap_or_default();
    let to_write = if !uid.is_empty() {
        crate::crypto::e2ee::encrypt_account_local_data(&uid, &format!("msg_cache:{conv_key}"), &payload)
            .unwrap_or(payload)
    } else {
        payload
    };
    let _ = atomic_write_file(&path, to_write.as_bytes());
}

pub fn save_draft_native(conv_key: &str, text: &str) {
    let safe = safe_blob_name(conv_key);
    let path = cas_root().join("drafts").join(format!("{safe}.txt"));
    if text.is_empty() {
        let _ = std::fs::remove_file(path);
    } else {
        let uid = crate::services::api::auth_me_cached_id().unwrap_or_default();
        let data = if !uid.is_empty() {
            crate::crypto::e2ee::encrypt_account_local_data(&uid, &format!("draft:{conv_key}"), text)
                .unwrap_or_else(|_| text.to_string())
        } else {
            text.to_string()
        };
        let _ = atomic_write_file(&path, data.as_bytes());
    }
}

pub fn load_draft_native(conv_key: &str) -> String {
    let safe = safe_blob_name(conv_key);
    let path = cas_root().join("drafts").join(format!("{safe}.txt"));
    let Ok(raw) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let uid = crate::services::api::auth_me_cached_id().unwrap_or_default();
    if raw.starts_with("lenc:v1:") && !uid.is_empty() {
        crate::crypto::e2ee::decrypt_account_local_data(&uid, &format!("draft:{conv_key}"), &raw)
            .unwrap_or_default()
    } else {
        raw
    }
}

pub fn save_dialogs_native(uid: &str, payload: &serde_json::Value) {
    let path = cas_root().join("dialogs.enc");
    let raw = payload.to_string();
    let data = crate::crypto::e2ee::encrypt_account_local_data(uid, "dialogs_cache", &raw)
        .unwrap_or(raw);
    let _ = atomic_write_file(&path, data.as_bytes());
}

pub fn load_dialogs_native(uid: &str) -> Option<serde_json::Value> {
    let path = cas_root().join("dialogs.enc");
    let raw = std::fs::read_to_string(path).ok()?;
    let dec = if raw.starts_with("lenc:v1:") {
        crate::crypto::e2ee::decrypt_account_local_data(uid, "dialogs_cache", &raw).ok()?
    } else {
        raw
    };
    serde_json::from_str(&dec).ok()
}

pub fn wipe_session_native_cache() {
    let root = cas_root();
    let _ = std::fs::remove_dir_all(root.join("messages"));
    let _ = std::fs::remove_dir_all(root.join("drafts"));
    let _ = std::fs::remove_file(root.join("dialogs.enc"));
    let _ = std::fs::remove_file(root.join("cursors.json"));
    let _ = std::fs::remove_file(root.join("outbox.json"));
    let _ = std::fs::remove_file(root.join("search_index.cwse1"));
}

pub async fn enqueue_outbox_item(entry: &OutboxEntry) {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut rows: Vec<OutboxEntry> = std::fs::read_to_string(outbox_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    rows.retain(|r| r.client_id != entry.client_id);
    rows.push(entry.clone());
    if let Ok(json) = serde_json::to_vec(&rows) {
        let _ = atomic_write_file(&outbox_path(), &json);
    }
}

pub async fn list_outbox_items() -> Vec<OutboxEntry> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    std::fs::read_to_string(outbox_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub async fn remove_outbox_item(client_id: &str) {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut rows: Vec<OutboxEntry> = std::fs::read_to_string(outbox_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    rows.retain(|r| r.client_id != client_id);
    if let Ok(json) = serde_json::to_vec(&rows) {
        let _ = atomic_write_file(&outbox_path(), &json);
    }
}
