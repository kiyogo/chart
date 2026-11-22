use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};

use serde::{Deserialize, Serialize};

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
}

fn cas_root() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .or_else(|_| std::env::var("LOCALAPPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("share"))
                .unwrap_or_else(|_| std::env::temp_dir())
        });
    let account_part: String = crate::services::api::auth_me_cached_id()
        .map(|uid| {
            uid.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect()
        })
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "default".to_string());
    base.join("chatw").join("cas").join(account_part)
}

fn index_path() -> PathBuf {
    cas_root().join("index.json")
}

fn cursors_path() -> PathBuf {
    cas_root().join("cursors.json")
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn load_index() -> HashMap<String, NativeEntry> {
    std::fs::read_to_string(index_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_index(map: &HashMap<String, NativeEntry>) {
    let root = cas_root();
    let _ = std::fs::create_dir_all(&root);
    if let Ok(json) = serde_json::to_string(map) {
        let _ = std::fs::write(index_path(), json);
    }
}

pub async fn init_local_storage() -> Result<(String, String, String), String> {
    let _ = std::fs::create_dir_all(cas_root());
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

pub async fn put_local_object(meta: &LocalObjectMeta, bytes: &[u8]) -> Result<String, String> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let sha = sha256_hex(bytes);
    let safe_id: String = meta.ref_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    let dir = cas_root().join("blobs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(&safe_id), bytes).map_err(|e| e.to_string())?;

    let mut idx = load_index();
    idx.insert(
        meta.ref_id.clone(),
        NativeEntry {
            ref_id: meta.ref_id.clone(),
            scope_type: meta.scope_type.clone(),
            file_name: meta.file_name.clone(),
            mime_type: meta.mime_type.clone(),
            byte_size: bytes.len() as u64,
            sha256_plain: meta.sha256_plain.clone().unwrap_or(sha),
            availability_state: meta.availability_state.clone(),
            last_accessed_at: now_secs(),
        },
    );
    save_index(&idx);
    Ok("native_fs".into())
}

pub async fn get_local_object(ref_id: &str) -> Option<(Vec<u8>, String, String)> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut idx = load_index();
    let entry = idx.get_mut(ref_id)?;
    let safe_id: String = ref_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    let bytes = std::fs::read(cas_root().join("blobs").join(&safe_id)).ok()?;
    let actual_sha = sha256_hex(&bytes);
    if !entry.sha256_plain.is_empty() && actual_sha != entry.sha256_plain {
        return None;
    }
    let mime = entry.mime_type.clone();
    entry.last_accessed_at = now_secs();
    save_index(&idx);
    Some((bytes, mime, actual_sha))
}

pub async fn delete_local_object(ref_id: &str, force_unique: bool) -> Result<u64, String> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut idx = load_index();
    let Some(entry) = idx.get(ref_id).cloned() else {
        return Err("not_found".into());
    };
    if !force_unique && matches!(entry.availability_state.as_str(), "local_only" | "pending_delivery") {
        return Err("unique_local_file".into());
    }
    let safe_id: String = ref_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    let _ = std::fs::remove_file(cas_root().join("blobs").join(&safe_id));
    idx.remove(ref_id);
    save_index(&idx);
    Ok(entry.byte_size)
}

pub async fn clear_safe_cache(include_unique: bool) -> Result<(u64, usize), String> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut idx = load_index();
    let mut freed = 0u64;
    let mut protected = 0usize;
    let keys: Vec<String> = idx.keys().cloned().collect();
    for k in keys {
        if let Some(entry) = idx.get(&k) {
            let unique = matches!(entry.availability_state.as_str(), "local_only" | "pending_delivery");
            if !include_unique && unique {
                protected += 1;
                continue;
            }
            let safe_id: String = k.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
            let _ = std::fs::remove_file(cas_root().join("blobs").join(&safe_id));
            freed += entry.byte_size;
            idx.remove(&k);
        }
    }
    save_index(&idx);
    Ok((freed, protected))
}

pub async fn get_quota_estimate() -> LocalQuotaInfo {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let idx = load_index();
    let limit_mb = crate::platform::storage_get("chatw_cache_limit_mb")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1024);
    let mut total = 0u64;
    let mut disposable = 0u64;
    let mut pinned = 0u64;
    for e in idx.values() {
        total += e.byte_size;
        if matches!(e.availability_state.as_str(), "local_only" | "pending_delivery") {
            pinned += e.byte_size;
        } else {
            disposable += e.byte_size;
        }
    }
    LocalQuotaInfo {
        limit_mb,
        browser_quota_bytes: 0,
        browser_usage_bytes: total,
        persisted: true,
        opfs_supported: false,
        local_object_count: idx.len(),
        total_local_bytes: total,
        disposable_cache_bytes: disposable,
        pinned_unique_bytes: pinned,
    }
}

pub async fn list_local_locations() -> Vec<serde_json::Value> {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    load_index()
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
    let _ = std::fs::create_dir_all(cas_root());
    let _ = std::fs::write(
        cursors_path(),
        serde_json::json!({ "dm_seq": dm_seq, "group_seq": group_seq, "channel_seq": channel_seq }).to_string(),
    );
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
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let items = v.get("items")?.clone();
    let has_more = v.get("has_more").and_then(|b| b.as_bool()).unwrap_or(false);
    Some((items, has_more))
}

pub async fn save_cached_messages_json(conv_key: &str, items: &serde_json::Value, has_more: bool) {
    let path = chat_cache_path(conv_key);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = serde_json::json!({ "items": items, "has_more": has_more });
    let _ = std::fs::write(path, payload.to_string());
}

pub async fn enqueue_outbox_item(entry: &OutboxEntry) {
    let _g = INDEX_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut rows: Vec<OutboxEntry> = std::fs::read_to_string(outbox_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    rows.retain(|r| r.client_id != entry.client_id);
    rows.push(entry.clone());
    let _ = std::fs::create_dir_all(cas_root());
    if let Ok(json) = serde_json::to_string(&rows) {
        let _ = std::fs::write(outbox_path(), json);
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
    let _ = std::fs::create_dir_all(cas_root());
    if let Ok(json) = serde_json::to_string(&rows) {
        let _ = std::fs::write(outbox_path(), json);
    }
}
