#[cfg(target_arch = "wasm32")]
pub mod browser;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod media_opt;
#[cfg(target_arch = "wasm32")]
pub mod realesrgan_gpu;
pub mod recovery;
pub mod sync_manager;
pub mod wasm_bridge;

use dioxus::prelude::GlobalSignal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Global live device trust state for UI banners and approval prompts.
pub static DEVICE_TRUST_STATE: GlobalSignal<Option<DeviceTrustInfo>> = GlobalSignal::new(|| None);
/// Pending incoming new-device approval challenge on an active trusted session.
pub static PENDING_APPROVAL_ALERT: GlobalSignal<Option<IncomingDeviceApproval>> = GlobalSignal::new(|| None);
/// Live two-phase sync progress summary.
pub static SYNC_PROGRESS: GlobalSignal<SyncProgressState> = GlobalSignal::new(SyncProgressState::default);
/// Non-None when the current account already has server E2EE keys (`me.has_keys == true`)
/// and this device needs passphrase unlock or trusted-device key sync instead of overwriting keys.
pub static E2EE_RECOVERY_REQUIRED: GlobalSignal<Option<String>> = GlobalSignal::new(|| None);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceTrustInfo {
    pub device_id: String,
    pub client_device_id: String,
    pub trust_state: String,
    pub verification_code: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IncomingDeviceApproval {
    pub request_id: String,
    pub requesting_device_id: String,
    pub device_name: String,
    pub platform: String,
    pub verification_code: String,
    #[serde(default)]
    pub ecdh_public_b64: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SyncProgressState {
    pub phase_a_done: bool,
    pub phase_b_running: bool,
    pub phase_b_paused: bool,
    pub queued_count: usize,
    pub completed_count: usize,
    pub last_sync_label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalQuotaInfo {
    pub limit_mb: i32,
    pub browser_quota_bytes: u64,
    pub browser_usage_bytes: u64,
    pub persisted: bool,
    pub opfs_supported: bool,
    pub local_object_count: usize,
    pub total_local_bytes: u64,
    pub disposable_cache_bytes: u64,
    pub pinned_unique_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalObjectMeta {
    pub ref_id: String,
    pub scope_type: String,
    pub file_name: String,
    pub mime_type: String,
    pub sha256_plain: Option<String>,
    pub availability_state: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub client_id: String,
    pub kind: String, // "dm" | "group" | "channel"
    pub target_id: String,
    #[serde(default)]
    pub sub_id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub reply_to_id: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub status: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Extract attachment/reference UUID from `/api/attachments/{uuid}` URL.
pub fn extract_ref_id(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    path.strip_prefix("/api/attachments/")
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.contains('/'))
        .map(str::to_string)
}

#[cfg(target_arch = "wasm32")]
pub use browser::{
    clear_safe_cache, delete_local_object, enqueue_outbox_item, get_client_device_info,
    get_local_object, get_quota_estimate, init_local_storage, list_local_locations,
    list_outbox_items, load_cached_messages_json, load_sync_cursors, put_local_object,
    remove_outbox_item, save_cached_messages_json, save_sync_cursors,
};

#[cfg(not(target_arch = "wasm32"))]
pub use native::{
    clear_safe_cache, delete_local_object, enqueue_outbox_item, get_client_device_info,
    get_local_object, get_quota_estimate, init_local_storage, list_local_locations,
    list_outbox_items, load_cached_messages_json, load_sync_cursors, put_local_object,
    remove_outbox_item, save_cached_messages_json, save_sync_cursors,
};

pub async fn load_cached_messages<T: serde::de::DeserializeOwned>(
    conv_key: &str,
) -> Option<(Vec<T>, bool)> {
    let (items_val, has_more) = load_cached_messages_json(conv_key).await?;
    let parsed: Vec<T> = serde_json::from_value(items_val).ok()?;
    Some((parsed, has_more))
}

pub async fn save_cached_messages<T: serde::Serialize>(
    conv_key: &str,
    items: &[T],
    has_more: bool,
) {
    if let Ok(val) = serde_json::to_value(items) {
        save_cached_messages_json(conv_key, &val, has_more).await;
    }
}

pub fn format_bytes_human(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", (bytes as f64) / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", (bytes as f64) / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", (bytes as f64) / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn availability_badge_label(state: &str) -> &'static str {
    match state {
        "local_only" => "Only available on this device",
        "pending_delivery" => "Wait for the device to receive",
        "available_on_peer" => "Available on enemy devices",
        "replicated_multi_device" => "Synchronized many devices",
        "backed_up_encrypted" => "Backed up encrypted",
        "server_durable" => "Persistently saved server",
        "restored_from_peer" => "Recovered from friends",
        "recovery_pending" => "Restoring",
        "queued_offline_peer" => "Wait for the other device to come online",
        "denied_by_peer" => "The opponent turns off recovery",
        "no_remaining_copies" => "There are no copies left",
        "deleted_for_everyone" => "Deleted for everyone",
        _ => "Status unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attachment_ref_ids_cleanly() {
        assert_eq!(extract_ref_id("/api/attachments/550e8400-e29b-41d4-a716-446655440000?variant=thumb").as_deref(), Some("550e8400-e29b-41d4-a716-446655440000"));
        assert_eq!(extract_ref_id("/uploads/media/abc.png"), None);
    }

    #[test]
    fn formats_bytes_and_badges_human_readably() {
        assert_eq!(format_bytes_human(512), "512 B");
        assert_eq!(format_bytes_human(2 * 1024 * 1024), "2.0 MB");
        assert_eq!(availability_badge_label("local_only"), "Only available on this device");
        assert_eq!(availability_badge_label("restored_from_peer"), "Recovered from friends");
    }
}
