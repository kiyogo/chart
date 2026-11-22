use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::KdataError;

pub const KDATA_MAGIC: &[u8; 8] = b"KIYODATA";
pub const KDATA_FOOTER_MAGIC: &[u8; 8] = b"KIYOEND1";
pub const KDATA_FORMAT_VERSION: u16 = 1;
pub const KDATA_SCHEMA_VERSION: u32 = 1;
pub const CIPHER_XCHACHA20_POLY1305_BE32: u8 = 1;

pub const DEFAULT_CHUNK_SIZE: u32 = 1024 * 1024; // 1 MiB
pub const MIN_CHUNK_SIZE: u32 = 16 * 1024;       // 16 KiB
pub const MAX_CHUNK_SIZE: u32 = 16 * 1024 * 1024;// 16 MiB

pub const FLAG_COMPRESSED_META: u8 = 0b0000_0001;
pub const FLAG_INCREMENTAL: u8 = 0b0000_0010;

/// Safe resource budgets enforced on untrusted import streams.
#[derive(Clone, Debug)]
pub struct ImportLimits {
    pub max_manifest_bytes: usize,
    pub max_index_bytes: usize,
    pub max_entries: usize,
    pub max_single_entry_bytes: u64,
    pub max_total_plain_bytes: u64,
    pub max_zip_expansion_ratio: u64,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_manifest_bytes: 32 * 1024 * 1024,
            max_index_bytes: 64 * 1024 * 1024,
            max_entries: 500_000,
            max_single_entry_bytes: 512 * 1024 * 1024,
            max_total_plain_bytes: 16 * 1024 * 1024 * 1024, // 16 GiB default safety cap (configurable)
            max_zip_expansion_ratio: 100,
        }
    }
}

/// Validates and normalizes relative entry paths inside `.kdata` or `.zip` archives.
/// Blocks path traversal (`..`), leading slashes, backslashes, Windows drive letters,
/// null bytes, and empty segments.
pub fn sanitize_relative_path(raw: &str) -> Result<String, KdataError> {
    if raw.is_empty() || raw.len() > 512 || raw.contains('\0') || raw.contains('\\') {
        return Err(KdataError::UnsafePath(raw.to_string()));
    }
    if raw.starts_with('/') || (raw.len() >= 2 && raw.as_bytes()[1] == b':') {
        return Err(KdataError::UnsafePath(raw.to_string()));
    }
    let mut parts = Vec::new();
    for seg in raw.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." || seg.chars().any(|c| c.is_control()) {
            return Err(KdataError::UnsafePath(raw.to_string()));
        }
        parts.push(seg);
    }
    Ok(parts.join("/"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportedAccountInfo {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub bio: Option<String>,
    pub status_text: Option<String>,
    #[serde(default)]
    pub preferences: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupSyncCursor {
    pub dm_seq: i64,
    pub group_seq: i64,
    pub channel_seq: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissingMediaReport {
    pub ref_id: Uuid,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub original_filename: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KdataManifest {
    pub backup_id: Uuid,
    pub snapshot_id: Uuid,
    pub format_version: u16,
    pub schema_version: u32,
    pub app_version: String,
    pub owner_user_id: Uuid,
    pub created_at: String,
    pub base_snapshot_id: Option<Uuid>,
    pub incremental: bool,
    pub checksum_algo: String,
    pub entries_count: u64,
    pub total_plain_bytes: u64,
    pub total_media_bytes: u64,
    pub sync_cursor: BackupSyncCursor,
    pub account: ExportedAccountInfo,
    #[serde(default)]
    pub missing_media: Vec<MissingMediaReport>,
    #[serde(default)]
    pub complete: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KdataEntryKind {
    Account,
    Settings,
    MessagesJson,
    ConversationMeta,
    MediaBlob,
    Avatar,
    Sticker,
    SyncMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KdataIndexEntry {
    pub entry_id: u32,
    pub kind: KdataEntryKind,
    pub logical_path: String,
    pub mime_type: String,
    pub plain_size: u64,
    pub sha256_hex: String,
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
    pub ref_id: Option<Uuid>,
    pub availability_state: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportedMessageRecord {
    pub id: Uuid,
    pub seq: i64,
    pub scope_type: String, // "dm" | "group" | "channel"
    pub conversation_id: String,
    pub sender_id: Uuid,
    pub sender_username: Option<String>,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub deleted_for_all: bool,
    pub reply_to_id: Option<Uuid>,
    #[serde(default)]
    pub reactions: Vec<serde_json::Value>,
    /// E2EE base64/text envelope or plaintext group/channel message body as stored canonically
    pub content_payload: String,
    /// Optional counterpart E2EE envelope (`recipient_envelope` when sender exported, or `sender_envelope` when recipient exported)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterpart_envelope: Option<String>,
    pub is_e2ee: bool,
    #[serde(default)]
    pub attachment_refs: Vec<Uuid>,
}
