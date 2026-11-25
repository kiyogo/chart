//! Core domain models, metadata, capability profiles, and telemetry snapshots for Spec 164.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Current binary serialization format version for `CWMC1` message chunks.
pub const CHUNK_FORMAT_VERSION: u16 = 1;
/// Mandatory Zstandard compression level (never silently downgraded).
pub const MANDATORY_ZSTD_LEVEL: i32 = 22;
/// Minimum message age (7 days in seconds) before becoming eligible for compression.
pub const MIN_COMPRESS_AGE_SECS: i64 = 7 * 86_400;
/// Maximum wait after crossing 7-day eligibility for low-volume conversations (48 hours).
pub const LOW_VOLUME_MAX_WAIT_SECS: i64 = 48 * 3_600;
/// Encryption scheme version (`CWMC1_AEAD` = AES-256-GCM + HKDF-SHA-256 + bucket padding).
pub const ENCRYPTION_VERSION_V1: u16 = 1;
/// Hard safety ceiling on decompressed chunk payload size (32 MiB) against compression bombs.
pub const MAX_DECOMPRESSED_CHUNK_BYTES: usize = 32 * 1024 * 1024;
/// Hard safety ceiling on message count per single chunk against pathological payloads.
pub const MAX_MESSAGES_PER_CHUNK: usize = 50_000;
/// Hard safety ceiling on compression expansion ratio against Zstd bombs.
pub const MAX_EXPANSION_RATIO: u64 = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsgStoreError {
    InvalidFormat(String),
    UnsupportedVersion(u16),
    CompressionFailed(String),
    DecompressionFailed(String),
    DecompressionBomb { limit: usize, actual: usize },
    ResourceConstrained(String),
    CryptoError(String),
    IntegrityMismatch { expected: String, actual: String },
    DictionaryMissing(u32),
    ScopeViolation(String),
    NoSpaceSavings { original: usize, packed: usize },
    Cancelled,
}

impl fmt::Display for MsgStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(s) => write!(f, "invalid format: {s}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            Self::CompressionFailed(s) => write!(f, "compression failed: {s}"),
            Self::DecompressionFailed(s) => write!(f, "decompression failed: {s}"),
            Self::DecompressionBomb { limit, actual } => {
                write!(f, "decompression bomb blocked: actual {actual} > limit {limit}")
            }
            Self::ResourceConstrained(s) => write!(f, "deferred due to device constraints: {s}"),
            Self::CryptoError(s) => write!(f, "crypto error: {s}"),
            Self::IntegrityMismatch { expected, actual } => {
                write!(f, "integrity mismatch: expected {expected}, got {actual}")
            }
            Self::DictionaryMissing(id) => write!(f, "missing zstd dictionary id={id}"),
            Self::ScopeViolation(s) => write!(f, "scope isolation violation: {s}"),
            Self::NoSpaceSavings { original, packed } => {
                write!(f, "skipping compression: packed {packed}B >= original {original}B")
            }
            Self::Cancelled => write!(f, "operation cancelled"),
        }
    }
}

impl std::error::Error for MsgStoreError {}

pub type MsgStoreResult<T> = Result<T, MsgStoreError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Dm,
    Group,
    Channel,
}

impl ScopeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dm => "dm",
            Self::Group => "group",
            Self::Channel => "channel",
        }
    }

    pub fn from_str_checked(s: &str) -> Option<Self> {
        match s {
            "dm" => Some(Self::Dm),
            "group" => Some(Self::Group),
            "channel" => Some(Self::Channel),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Dm => 1,
            Self::Group => 2,
            Self::Channel => 3,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Dm),
            2 => Some(Self::Group),
            3 => Some(Self::Channel),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageTier {
    /// Messages < 7 days old and active RAM/L1 hot data.
    Hot,
    /// Compressed encrypted chunks kept locally (L2) or fast server CAS.
    Warm,
    /// Infrequently accessed old chunks archived to S3/MinIO cold storage.
    Cold,
}

impl StorageTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    /// Default: dynamically adapts to device free space and usage patterns.
    Auto,
    /// Smaller cache, reduced prefetching, less local history retention.
    SpaceSaver,
    /// Larger cache within safe bounds, aggressive prefetching, larger search index.
    HighPerformance,
    /// User-specified limit in MiB, strictly clamped to real safe device ceiling.
    Custom,
}

impl StorageMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::SpaceSaver => "space_saver",
            Self::HighPerformance => "high_performance",
            Self::Custom => "custom",
        }
    }

    pub fn parse_mode(s: &str) -> Self {
        match s {
            "space_saver" => Self::SpaceSaver,
            "high_performance" => Self::HighPerformance,
            "custom" => Self::Custom,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text = 0,
    Image = 1,
    Video = 2,
    AudioVoice = 3,
    File = 4,
    Sticker = 5,
    Gif = 6,
    System = 7,
    CallLog = 8,
    Link = 9,
}

impl MessageType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Image,
            2 => Self::Video,
            3 => Self::AudioVoice,
            4 => Self::File,
            5 => Self::Sticker,
            6 => Self::Gif,
            7 => Self::System,
            8 => Self::CallLog,
            9 => Self::Link,
            _ => Self::Text,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRef {
    pub ref_id: String,
    pub mime_type: String,
    pub file_name: String,
    pub byte_size: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub duration_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_b64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce_b64: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionEntry {
    pub emoji: String,
    pub user_id: String,
    pub created_at_ms: i64,
}

/// Lossless canonical representation of any ChatW message across DM, Group, and Channel scopes.
/// Preserves 100% precision for IDs, sequences, millisecond timestamps, sender, reply, flags,
/// reactions, attachment references, and arbitrary metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactMessage {
    pub id: String,
    pub seq: i64,
    pub sender_id: String,
    /// Exact UTC timestamp in milliseconds since Unix epoch (zero precision loss).
    pub created_at_ms: i64,
    pub msg_type: MessageType,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    #[serde(default)]
    pub is_edited: bool,
    #[serde(default)]
    pub deleted_for_all: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_bookmarked: bool,
    #[serde(default)]
    pub status_code: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<ReactionEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentRef>,
    /// Optional lossless JSON/structured extra metadata (forwarded info, thread_id, bot_id, custom fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_meta: Option<String>,
}

/// Full metadata describing an immutable Message Chunk (Section 4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_id: String,
    pub account_id: String,
    pub scope_type: ScopeKind,
    pub scope_id: String,
    pub chunk_version: u32,
    pub format_version: u16,
    /// History visibility epoch (e.g. group membership/key rotation epoch) preventing cross-epoch mixing.
    pub access_epoch: u32,
    pub compression_algo: String,
    pub compression_level: i32,
    /// `0` = no dictionary; `1` = built-in public multilingual chat dict v1; `>= 1000` = private E2EE dict.
    pub dictionary_id: u32,
    pub original_size: u64,
    pub serialized_size: u64,
    pub compressed_size: u64,
    pub padded_size: u64,
    pub encrypted_size: u64,
    pub message_count: u32,
    pub seq_start: i64,
    pub seq_end: i64,
    pub first_created_at_ms: i64,
    pub last_created_at_ms: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub encryption_version: u16,
    pub key_version: u32,
    pub nonce_b64: String,
    /// BLAKE3 hash of serialized plaintext bytes before compression (integrity verification).
    pub plaintext_blake3_hex: String,
    /// BLAKE3 hash of final sealed ciphertext blob.
    pub ciphertext_blake3_hex: String,
    pub storage_tier: StorageTier,
    pub storage_location: String,
    pub sync_state: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOp {
    Edit,
    Delete,
    Recall,
    Pin,
    Unpin,
    ReactionAdd,
    ReactionRemove,
    Bookmark,
    Unbookmark,
}

/// Append-only mutation / tombstone overlay applied over immutable compressed chunks (Section 22).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkMutation {
    pub mutation_seq: i64,
    pub scope_type: ScopeKind,
    pub scope_id: String,
    pub message_id: String,
    pub chunk_id: Option<String>,
    pub op: MutationOp,
    pub actor_id: String,
    pub new_text: Option<String>,
    pub emoji: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformKind {
    Web,
    Android,
    Ios,
    Windows,
    Linux,
    MacOs,
    Server,
}

/// Device hardware & OS capability profile driving chunk sizing, Zstd-22 window limits, and budgets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCapabilityProfile {
    pub platform: PlatformKind,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub cpu_cores: u32,
    pub total_disk_bytes: u64,
    pub free_disk_bytes: u64,
    pub os_quota_bytes: Option<u64>,
    pub is_ssd: bool,
    pub is_mobile: bool,
}

impl Default for DeviceCapabilityProfile {
    fn default() -> Self {
        Self {
            platform: PlatformKind::Linux,
            total_ram_mb: 8192,
            available_ram_mb: 4096,
            cpu_cores: 4,
            total_disk_bytes: 256 * 1024 * 1024 * 1024,
            free_disk_bytes: 32 * 1024 * 1024 * 1024,
            os_quota_bytes: None,
            is_ssd: true,
            is_mobile: false,
        }
    }
}

/// Live runtime pressure signals used by the ResourceScheduler (Section 19).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceSignals {
    pub active_voice_or_video_call: bool,
    pub user_actively_scrolling_or_typing: bool,
    pub app_in_foreground: bool,
    pub battery_level_pct: Option<u8>,
    pub on_battery_saver: bool,
    pub thermal_throttled: bool,
    pub on_metered_or_cellular: bool,
    pub data_saver_enabled: bool,
    pub is_offline: bool,
    pub estimated_bandwidth_kbps: u32,
    pub ram_pressure_pct: u8,
}

/// Privacy-safe technical metrics snapshot (Section 32 — ZERO message text, queries, or keys).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub compression_ratio: f64,
    pub compression_queue_size: u64,
    pub compression_success_count: u64,
    pub compression_failure_count: u64,
    pub decompression_success_count: u64,
    pub decompression_failure_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub storage_usage_bytes: u64,
    pub storage_budget_bytes: u64,
    pub total_saved_bytes: u64,
    pub indexed_message_count: u64,
    pub pending_index_count: u64,
    pub last_search_latency_ms: f64,
    pub last_sync_latency_ms: f64,
    pub chunk_count: u64,
    pub gc_reclaimed_bytes: u64,
    pub cpu_budget_ms_per_tick: u32,
    pub ram_budget_bytes: u64,
    pub network_budget_kbps: u32,
}

impl TelemetrySnapshot {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    pub fn compression_failure_rate(&self) -> f64 {
        let total = self.compression_success_count + self.compression_failure_count;
        if total == 0 {
            0.0
        } else {
            self.compression_failure_count as f64 / total as f64
        }
    }

    pub fn decompression_failure_rate(&self) -> f64 {
        let total = self.decompression_success_count + self.decompression_failure_count;
        if total == 0 {
            0.0
        } else {
            self.decompression_failure_count as f64 / total as f64
        }
    }
}
