use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Write},
    path::PathBuf,
};

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::Utc;
use kiyo_kdata::{
    BackupSyncCursor, DEFAULT_CHUNK_SIZE, ExportedAccountInfo, ExportedMessageRecord, ImportLimits,
    KDATA_FORMAT_VERSION, KDATA_MAGIC, KDATA_SCHEMA_VERSION, KdataEntryKind, KdataIndexEntry,
    KdataManifest, KdataStreamWriter, KdfParams, MissingMediaReport, ZipEncryptionMode,
    ZipEntryInput, read_zip_archive, sanitize_relative_path, stream_read_kdata, write_zip_archive,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::permissions as perm_flags,
    servers::permissions as server_perms,
    storage::{StorageAdapter, StorageRegistry, local_cas::LocalFileStorageAdapter, safe_join_relative, sha256_bytes},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExportScopeOptions {
    #[serde(default = "yes")]
    pub include_account: bool,
    #[serde(default = "yes")]
    pub include_messages: bool,
    #[serde(default = "yes")]
    pub include_media: bool,
    #[serde(default = "yes")]
    pub include_settings: bool,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

impl Default for ExportScopeOptions {
    fn default() -> Self {
        Self {
            include_account: true,
            include_messages: true,
            include_media: true,
            include_settings: true,
            conversation_id: None,
        }
    }
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize)]
pub struct ExportPreviewResponse {
    pub account_username: Option<String>,
    pub dm_messages: i64,
    pub group_messages: i64,
    pub channel_messages: i64,
    pub durable_media_count: i64,
    pub durable_media_bytes: i64,
    pub missing_or_client_only_count: i64,
    pub categories: Vec<String>,
    pub honest_notice: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InspectArchiveResponse {
    pub detected_format: String,
    pub backup_id: Option<Uuid>,
    pub created_at: Option<String>,
    pub owner_user_id: Option<Uuid>,
    pub owner_mismatch: bool,
    pub total_entries: usize,
    pub total_plain_bytes: u64,
    pub total_messages: usize,
    pub new_messages: usize,
    pub duplicate_messages: usize,
    pub total_media: usize,
    pub new_media: usize,
    pub duplicate_media: usize,
    pub missing_media_in_backup: Vec<MissingMediaReport>,
    pub integrity_verified: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestoreOptions {
    #[serde(default = "default_mode")]
    pub mode: String, // full | messages | images | videos | audio | files | avatar | settings | single_conversation | missing_only
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

fn default_mode() -> String {
    "full".into()
}

#[derive(Clone, Debug, Serialize)]
pub struct RestoreResultResponse {
    pub backup_id: Option<Uuid>,
    pub mode: String,
    pub restored_messages: usize,
    pub skipped_duplicate_messages: usize,
    pub skipped_unauthorized_messages: usize,
    pub restored_media: usize,
    pub skipped_duplicate_media: usize,
    pub restored_settings: bool,
    pub restored_avatar: bool,
    pub integrity_verified: bool,
    pub warnings: Vec<String>,
}

pub async fn build_export_preview(pool: &PgPool, user_id: Uuid) -> AppResult<ExportPreviewResponse> {
    let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .flatten();

    let dm_messages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE (sender_id = $1 OR receiver_id = $1) AND deleted_for_all = FALSE",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let group_messages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM group_messages gm
         WHERE EXISTS (SELECT 1 FROM group_members mem WHERE mem.group_id = gm.group_id AND mem.user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let mut channel_messages: i64 = 0;
    let channel_rows = sqlx::query(
        "SELECT c.id, c.server_id FROM channels c
         JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut allowed_channels = Vec::new();
    for r in channel_rows {
        let cid: Uuid = r.try_get("id")?;
        let sid: Uuid = r.try_get("server_id")?;
        if server_perms::require_server_permission(pool, sid, user_id, perm_flags::VIEW_CHANNELS)
            .await
            .is_ok()
        {
            allowed_channels.push(cid);
        }
    }
    if !allowed_channels.is_empty() {
        channel_messages = sqlx::query_scalar(
            "SELECT COUNT(*) FROM channel_messages WHERE channel_id = ANY($1)",
        )
        .bind(&allowed_channels)
        .fetch_one(pool)
        .await?;
    }

    let media_rows = sqlx::query(
        "SELECT mr.id, mr.scope_type, mr.scope_id, mr.owner_id, mr.byte_size, mr.object_id, mr.availability_state
         FROM media_references mr
         WHERE mr.deleted_for_all = FALSE
           AND (mr.owner_id = $1
                OR (mr.scope_type = 'dm' AND mr.scope_id = $1)
                OR (mr.scope_type = 'group' AND mr.scope_id IN (SELECT group_id FROM group_members WHERE user_id = $1))
                OR (mr.scope_type = 'channel' AND mr.scope_id = ANY($2)))",
    )
    .bind(user_id)
    .bind(&allowed_channels)
    .fetch_all(pool)
    .await?;

    let mut durable_media_count = 0i64;
    let mut durable_media_bytes = 0i64;
    let mut missing_or_client_only_count = 0i64;
    for mr in media_rows {
        let obj_id: Option<Uuid> = mr.try_get("object_id")?;
        let sz: i64 = mr.try_get("byte_size")?;
        if obj_id.is_some() {
            durable_media_count += 1;
            durable_media_bytes += sz.max(0);
        } else {
            missing_or_client_only_count += 1;
        }
    }

    let categories = vec![
        "Account profile & personal settings (no passwords or tokens)".to_string(),
        format!("Private chat messages ({dm_messages})"),
        format!("Group messages ({group_messages})"),
        format!("Messages the server is allowed to view ({channel_messages})"),
        format!("Media files on the server ({durable_media_count} files)"),
    ];

    let honest_notice = if missing_or_client_only_count > 0 {
        format!(
            "There are {missing_or_client_only_count} private chat files that are only saved locally on the device or on the partner's side and not on the server; Missing files will be faithfully noted in the manifest."
        )
    } else {
        "All data with valid access rights on the server is ready to be included in the backup.".to_string()
    };

    Ok(ExportPreviewResponse {
        account_username: username,
        dm_messages,
        group_messages,
        channel_messages,
        durable_media_count,
        durable_media_bytes,
        missing_or_client_only_count,
        categories,
        honest_notice,
    })
}

pub enum MediaSourceStream {
    Memory(Vec<u8>),
    File(PathBuf),
}

pub struct PreparedEntry {
    pub index: KdataIndexEntry,
    pub source: MediaSourceStream,
}

pub struct PreparedBackupDataset {
    pub manifest: KdataManifest,
    pub entries: Vec<PreparedEntry>,
}

pub(crate) async fn prepare_backup_dataset(
    pool: &PgPool,
    storage: &StorageRegistry,
    user_id: Uuid,
    opts: &ExportScopeOptions,
) -> AppResult<PreparedBackupDataset> {
    let backup_id = Uuid::new_v4();
    let snapshot_id = Uuid::new_v4();

    // 1. Safe account profile (NEVER secrets, tokens, TOTP, password_phc, or private keys)
    let urow = sqlx::query(
        "SELECT username, display_name, avatar_url, custom_banner_url, bio, status_text,
                profile_color, profile_accent, avatar_decoration, profile_effect
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let user_settings_json: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT data FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let storage_policy_row = sqlx::query(
        "SELECT cache_limit_mb, auto_download_mode, dm_relay_ttl_hours, dm_encrypted_backup, require_device_approval, prefer_original_upload
         FROM device_storage_policies WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let mut prefs_obj = user_settings_json.unwrap_or_else(|| serde_json::json!({}));
    if let Some(sp) = storage_policy_row {
        if let Some(map) = prefs_obj.as_object_mut() {
            map.insert(
                "storage_policy".into(),
                serde_json::json!({
                    "cache_limit_mb": sp.try_get::<i32, _>("cache_limit_mb").unwrap_or(1024),
                    "auto_download_mode": sp.try_get::<String, _>("auto_download_mode").unwrap_or_else(|_| "wifi".into()),
                    "dm_relay_ttl_hours": sp.try_get::<i32, _>("dm_relay_ttl_hours").unwrap_or(168),
                    "dm_encrypted_backup": sp.try_get::<bool, _>("dm_encrypted_backup").unwrap_or(false),
                    "require_device_approval": sp.try_get::<bool, _>("require_device_approval").unwrap_or(false),
                    "prefer_original_upload": sp.try_get::<bool, _>("prefer_original_upload").unwrap_or(false),
                }),
            );
        }
    }

    let account = ExportedAccountInfo {
        user_id,
        username: urow.try_get("username")?,
        display_name: urow.try_get("display_name")?,
        avatar_url: urow.try_get("avatar_url")?,
        banner_url: urow.try_get("custom_banner_url")?,
        bio: urow.try_get("bio")?,
        status_text: urow.try_get("status_text")?,
        preferences: prefs_obj.clone(),
    };

    let mut prepared: Vec<PreparedEntry> = Vec::new();
    let mut missing_media: Vec<MissingMediaReport> = Vec::new();
    let mut next_id = 0u32;
    let mut total_plain = 0u64;
    let mut total_media = 0u64;

    let mut push_mem = |kind: KdataEntryKind,
                        path: String,
                        mime: &str,
                        bytes: Vec<u8>,
                        scope_type: Option<String>,
                        scope_id: Option<Uuid>,
                        ref_id: Option<Uuid>,
                        avail: Option<String>| {
        let len = bytes.len() as u64;
        let sha = hex::encode(sha256_bytes(&bytes));
        total_plain += len;
        let entry_id = next_id;
        next_id += 1;
        prepared.push(PreparedEntry {
            index: KdataIndexEntry {
                entry_id,
                kind,
                logical_path: path,
                mime_type: mime.to_string(),
                plain_size: len,
                sha256_hex: sha,
                scope_type,
                scope_id,
                ref_id,
                availability_state: avail,
            },
            source: MediaSourceStream::Memory(bytes),
        });
    };

    if opts.include_account {
        let acc_bytes = serde_json::to_vec_pretty(&account).map_err(|_| AppError::Internal)?;
        push_mem(
            KdataEntryKind::Account,
            "account/profile.json".into(),
            "application/json",
            acc_bytes,
            None,
            None,
            None,
            None,
        );
    }

    if opts.include_settings {
        let set_bytes = serde_json::to_vec_pretty(&prefs_obj).map_err(|_| AppError::Internal)?;
        push_mem(
            KdataEntryKind::Settings,
            "account/settings.json".into(),
            "application/json",
            set_bytes,
            None,
            None,
            None,
            None,
        );
    }

    // RBAC-verified channels
    let channel_rows = sqlx::query(
        "SELECT c.id, c.server_id FROM channels c
         JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut allowed_channels = Vec::new();
    for r in channel_rows {
        let cid: Uuid = r.try_get("id")?;
        let sid: Uuid = r.try_get("server_id")?;
        if server_perms::require_server_permission(pool, sid, user_id, perm_flags::VIEW_CHANNELS)
            .await
            .is_ok()
        {
            allowed_channels.push(cid);
        }
    }

    let mut dm_max_seq = 0i64;
    let mut group_max_seq = 0i64;
    let mut channel_max_seq = 0i64;

    if opts.include_messages {
        let mut msg_records: Vec<ExportedMessageRecord> = Vec::new();

        // DM messages (excluding permanently deleted messages)
        let dm_rows = sqlx::query(
            "SELECT m.id, m.seq, m.sender_id, m.receiver_id, m.recipient_envelope, m.sender_envelope,
                    m.reply_to_id, m.is_edited, m.deleted_for_all, m.created_at, u.username AS sender_username
             FROM messages m
             JOIN users u ON u.id = m.sender_id
             WHERE (m.sender_id = $1 OR m.receiver_id = $1) AND m.deleted_for_all = FALSE
             ORDER BY m.seq ASC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        for r in dm_rows {
            let seq: i64 = r.try_get("seq")?;
            dm_max_seq = dm_max_seq.max(seq);
            let sender_id: Uuid = r.try_get("sender_id")?;
            let receiver_id: Uuid = r.try_get("receiver_id")?;
            let partner_id = if sender_id == user_id { receiver_id } else { sender_id };
            let conv = partner_id.to_string();
            if let Some(ref filter) = opts.conversation_id {
                if filter != &conv && filter != &format!("dm:{conv}") {
                    continue;
                }
            }
            let sender_env_bytes: Vec<u8> = r.try_get("sender_envelope")?;
            let recipient_env_bytes: Vec<u8> = r.try_get("recipient_envelope")?;
            let (my_env, other_env) = if sender_id == user_id {
                (sender_env_bytes, recipient_env_bytes)
            } else {
                (recipient_env_bytes, sender_env_bytes)
            };
            let created_at: chrono::DateTime<Utc> = r.try_get("created_at")?;
            let is_edited: bool = r.try_get("is_edited")?;
            msg_records.push(ExportedMessageRecord {
                id: r.try_get("id")?,
                seq,
                scope_type: "dm".into(),
                conversation_id: conv,
                sender_id,
                sender_username: r.try_get("sender_username")?,
                created_at: created_at.to_rfc3339(),
                edited_at: if is_edited { Some(created_at.to_rfc3339()) } else { None },
                deleted_for_all: false,
                reply_to_id: r.try_get("reply_to_id")?,
                reactions: Vec::new(),
                content_payload: B64.encode(my_env),
                counterpart_envelope: Some(B64.encode(other_env)),
                is_e2ee: true,
                attachment_refs: Vec::new(),
            });
        }

        // Group messages (excluding removed/non-member groups)
        let grp_rows = sqlx::query(
            "SELECT gm.id, gm.seq, gm.group_id, gm.sender_id, gm.content, gm.reply_to_id, gm.is_edited, gm.created_at, u.username AS sender_username
             FROM group_messages gm
             JOIN users u ON u.id = gm.sender_id
             WHERE EXISTS (SELECT 1 FROM group_members mem WHERE mem.group_id = gm.group_id AND mem.user_id = $1)
             ORDER BY gm.seq ASC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        for r in grp_rows {
            let seq: i64 = r.try_get("seq")?;
            group_max_seq = group_max_seq.max(seq);
            let gid: Uuid = r.try_get("group_id")?;
            let conv = gid.to_string();
            if let Some(ref filter) = opts.conversation_id {
                if filter != &conv && filter != &format!("group:{conv}") {
                    continue;
                }
            }
            let created_at: chrono::DateTime<Utc> = r.try_get("created_at")?;
            let is_edited: bool = r.try_get("is_edited")?;
            msg_records.push(ExportedMessageRecord {
                id: r.try_get("id")?,
                seq,
                scope_type: "group".into(),
                conversation_id: conv,
                sender_id: r.try_get("sender_id")?,
                sender_username: r.try_get("sender_username")?,
                created_at: created_at.to_rfc3339(),
                edited_at: if is_edited { Some(created_at.to_rfc3339()) } else { None },
                deleted_for_all: false,
                reply_to_id: r.try_get("reply_to_id")?,
                reactions: Vec::new(),
                content_payload: r.try_get("content")?,
                counterpart_envelope: None,
                is_e2ee: false,
                attachment_refs: Vec::new(),
            });
        }

        // Channel messages (strictly filtered by RBAC VIEW_CHANNELS)
        if !allowed_channels.is_empty() {
            let ch_rows = sqlx::query(
                "SELECT cm.id, cm.seq, cm.channel_id, cm.sender_id, cm.content, cm.reply_to_id, cm.is_edited, cm.created_at, u.username AS sender_username
                 FROM channel_messages cm
                 JOIN users u ON u.id = cm.sender_id
                 WHERE cm.channel_id = ANY($1)
                 ORDER BY cm.seq ASC",
            )
            .bind(&allowed_channels)
            .fetch_all(pool)
            .await?;

            for r in ch_rows {
                let seq: i64 = r.try_get("seq")?;
                channel_max_seq = channel_max_seq.max(seq);
                let cid: Uuid = r.try_get("channel_id")?;
                let conv = cid.to_string();
                if let Some(ref filter) = opts.conversation_id {
                    if filter != &conv && filter != &format!("channel:{conv}") {
                        continue;
                    }
                }
                let created_at: chrono::DateTime<Utc> = r.try_get("created_at")?;
                let is_edited: bool = r.try_get("is_edited")?;
                msg_records.push(ExportedMessageRecord {
                    id: r.try_get("id")?,
                    seq,
                    scope_type: "channel".into(),
                    conversation_id: conv,
                    sender_id: r.try_get("sender_id")?,
                    sender_username: r.try_get("sender_username")?,
                    created_at: created_at.to_rfc3339(),
                    edited_at: if is_edited { Some(created_at.to_rfc3339()) } else { None },
                    deleted_for_all: false,
                    reply_to_id: r.try_get("reply_to_id")?,
                    reactions: Vec::new(),
                    content_payload: r.try_get("content")?,
                    counterpart_envelope: None,
                    is_e2ee: false,
                    attachment_refs: Vec::new(),
                });
            }
        }

        let msgs_bytes = serde_json::to_vec(&msg_records).map_err(|_| AppError::Internal)?;
        push_mem(
            KdataEntryKind::MessagesJson,
            "messages/history.json".into(),
            "application/json",
            msgs_bytes,
            None,
            None,
            None,
            None,
        );
    }

    if opts.include_media {
        let media_rows = sqlx::query(
            "SELECT mr.id, mr.scope_type, mr.scope_id, mr.original_filename, mr.mime_type,
                    mr.byte_size, mr.sha256_plain, mr.availability_state,
                    mo.storage_backend, mo.storage_key, mo.byte_size AS obj_bytes, mo.sha256 AS obj_sha
             FROM media_references mr
             LEFT JOIN media_objects mo ON mo.id = mr.object_id
             WHERE mr.deleted_for_all = FALSE
               AND (mr.owner_id = $1
                    OR (mr.scope_type = 'dm' AND mr.scope_id = $1)
                    OR (mr.scope_type = 'group' AND mr.scope_id IN (SELECT group_id FROM group_members WHERE user_id = $1))
                    OR (mr.scope_type = 'channel' AND mr.scope_id = ANY($2)))
             ORDER BY mr.created_at ASC",
        )
        .bind(user_id)
        .bind(&allowed_channels)
        .fetch_all(pool)
        .await?;

        let mut seen_names = HashSet::new();
        for mr in media_rows {
            let ref_id: Uuid = mr.try_get("id")?;
            let scope_type: String = mr.try_get("scope_type")?;
            let scope_id: Option<Uuid> = mr.try_get("scope_id")?;
            if let Some(ref filter) = opts.conversation_id {
                let s_str = scope_id.map(|u| u.to_string()).unwrap_or_default();
                if filter != &s_str && filter != &format!("{scope_type}:{s_str}") {
                    continue;
                }
            }
            let raw_name: String = mr.try_get("original_filename")?;
            let mime: String = mr.try_get("mime_type")?;
            let avail: String = mr.try_get("availability_state")?;
            let backend_opt: Option<String> = mr.try_get("storage_backend")?;
            let key_opt: Option<String> = mr.try_get("storage_key")?;

            let safe_base: String = raw_name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
                .take(80)
                .collect();
            let base = if safe_base.is_empty() { "file.bin".to_string() } else { safe_base };
            let mut logical = format!("media/{scope_type}/{}_{}", &ref_id.simple().to_string()[..8], base);
            if !seen_names.insert(logical.clone()) {
                logical = format!("media/{scope_type}/{}_{}", ref_id.simple(), base);
            }

            let Some((backend, skey)) = backend_opt.zip(key_opt) else {
                missing_media.push(MissingMediaReport {
                    ref_id,
                    scope_type,
                    scope_id,
                    original_filename: raw_name,
                    reason: format!("The file has no copy on the server ({avail}); Need to sync directly from the device holding the file"),
                });
                continue;
            };

            match resolve_media_source(storage, &backend, &skey).await {
                Ok((source, byte_len, sha_hex)) => {
                    let entry_id = next_id;
                    next_id += 1;
                    total_plain += byte_len;
                    total_media += byte_len;
                    let kind = if scope_type == "avatar" {
                        KdataEntryKind::Avatar
                    } else if scope_type == "sticker" {
                        KdataEntryKind::Sticker
                    } else {
                        KdataEntryKind::MediaBlob
                    };
                    prepared.push(PreparedEntry {
                        index: KdataIndexEntry {
                            entry_id,
                            kind,
                            logical_path: logical,
                            mime_type: mime,
                            plain_size: byte_len,
                            sha256_hex: sha_hex,
                            scope_type: Some(scope_type),
                            scope_id,
                            ref_id: Some(ref_id),
                            availability_state: Some(avail),
                        },
                        source,
                    });
                }
                Err(_) => {
                    missing_media.push(MissingMediaReport {
                        ref_id,
                        scope_type,
                        scope_id,
                        original_filename: raw_name,
                        reason: "The file on the storage backend does not exist or has expired in transit".into(),
                    });
                }
            }
        }
    }

    let complete = missing_media.is_empty();
    let mut notes = vec![
        "Official format .kdata (Argon2id + XChaCha20-Poly1305 STREAM BE32 + SHA-256)".to_string(),
    ];
    if !complete {
        notes.push(format!(
            "There are {} media files that do not have a copy available on the server at the time of export.",
            missing_media.len()
        ));
    }

    let manifest = KdataManifest {
        backup_id,
        snapshot_id,
        format_version: KDATA_FORMAT_VERSION,
        schema_version: KDATA_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        owner_user_id: user_id,
        created_at: Utc::now().to_rfc3339(),
        base_snapshot_id: None,
        incremental: false,
        checksum_algo: "sha256".into(),
        entries_count: prepared.len() as u64,
        total_plain_bytes: total_plain,
        total_media_bytes: total_media,
        sync_cursor: BackupSyncCursor {
            dm_seq: dm_max_seq,
            group_seq: group_max_seq,
            channel_seq: channel_max_seq,
        },
        account,
        missing_media,
        complete,
        notes,
    };

    Ok(PreparedBackupDataset {
        manifest,
        entries: prepared,
    })
}

async fn resolve_media_source(
    storage: &StorageRegistry,
    backend: &str,
    key: &str,
) -> Result<(MediaSourceStream, u64, String), crate::storage::StorageError> {
    let local_root: Option<&std::path::Path> = match backend {
        "local_cas" => Some(storage.local_cas.root()),
        "temp_relay" => Some(storage.temp_relay.root()),
        "encrypted_backup" => Some(storage.encrypted_backup.root()),
        _ => None,
    };

    if let Some(root) = local_root {
        let path = safe_join_relative(root, key)?;
        let (len, sha) = crate::storage::sha256_file(&path).await?;
        return Ok((MediaSourceStream::File(path), len, hex::encode(sha)));
    }

    let bytes = match backend {
        "legacy" => storage.existing.get_object(key).await?,
        "s3_ready" => storage.s3.get_object(key).await?,
        _ => return Err(crate::storage::StorageError::NotFound(key.into())),
    };
    let len = bytes.len() as u64;
    let sha = hex::encode(sha256_bytes(&bytes));
    Ok((MediaSourceStream::Memory(bytes), len, sha))
}

/// Streams a complete encrypted `.kdata` archive directly into `sink` without holding all media in RAM.
pub fn write_prepared_dataset_to_kdata<W: Write>(
    sink: W,
    dataset: PreparedBackupDataset,
    encryption_password: &str,
    job_progress: Option<(&PgPool, Uuid)>,
) -> Result<(W, KdataManifest), crate::error::AppError> {
    let index: Vec<KdataIndexEntry> = dataset.entries.iter().map(|e| e.index.clone()).collect();
    let manifest = dataset.manifest;
    let mut writer = KdataStreamWriter::begin(
        sink,
        encryption_password,
        &manifest,
        &index,
        DEFAULT_CHUNK_SIZE,
        KdfParams::default(),
        Utc::now().timestamp(),
    )
    .map_err(|_| AppError::BadRequest("invalid_encryption_password"))?;

    let _ = job_progress;
    for item in dataset.entries {
        match item.source {
            MediaSourceStream::Memory(bytes) => {
                writer
                    .write_entry_bytes(item.index.entry_id, &bytes)
                    .map_err(|_| AppError::Internal)?;
            }
            MediaSourceStream::File(path) => {
                let mut f = std::fs::File::open(&path).map_err(|_| AppError::Internal)?;
                writer
                    .write_entry_from_reader(item.index.entry_id, item.index.plain_size, &mut f)
                    .map_err(|_| AppError::Internal)?;
            }
        }
    }

    let finished = writer.finish().map_err(|_| AppError::Internal)?;
    Ok((finished, manifest))
}

/// Builds an authorized ZIP archive (either AES-256 or locked plaintext ZIP).
pub async fn build_zip_from_dataset(
    dataset: PreparedBackupDataset,
    aes_password: Option<&str>,
) -> AppResult<(Vec<u8>, KdataManifest)> {
    let manifest = dataset.manifest;
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|_| AppError::Internal)?;
    let pass_owned = aes_password.map(str::to_string);

    let zip_bytes = tokio::task::spawn_blocking(move || -> AppResult<Vec<u8>> {
        let mut owned_blobs: Vec<(String, Vec<u8>)> = Vec::with_capacity(dataset.entries.len() + 1);
        owned_blobs.push(("manifest.json".to_string(), manifest_bytes));
        for item in dataset.entries {
            let bytes = match item.source {
                MediaSourceStream::Memory(b) => b,
                MediaSourceStream::File(p) => std::fs::read(&p).map_err(|_| AppError::Internal)?,
            };
            owned_blobs.push((item.index.logical_path, bytes));
        }
        let inputs: Vec<ZipEntryInput<'_>> = owned_blobs
            .iter()
            .map(|(p, b)| ZipEntryInput {
                path: p.as_str(),
                bytes: b.as_slice(),
            })
            .collect();
        let mode = match pass_owned.as_deref() {
            Some(pw) if !pw.is_empty() => ZipEncryptionMode::Aes256(pw),
            Some(_) => return Err(AppError::BadRequest("zip_password_required")),
            None => ZipEncryptionMode::UnencryptedAuthorized,
        };
        let cursor = write_zip_archive(Cursor::new(Vec::new()), &inputs, mode)
            .map_err(|_| AppError::Internal)?;
        Ok(cursor.into_inner())
    })
    .await
    .map_err(|_| AppError::Internal)??;

    Ok((zip_bytes, manifest))
}

struct DecodedBackupBundle {
    format: String,
    manifest: Option<KdataManifest>,
    messages: Vec<ExportedMessageRecord>,
    media_files: Vec<(KdataIndexEntry, Vec<u8>)>,
    account: Option<ExportedAccountInfo>,
    settings: Option<serde_json::Value>,
    warnings: Vec<String>,
}

fn decode_archive_bundle(bytes: &[u8], password: Option<&str>) -> Result<DecodedBackupBundle, AppError> {
    let limits = ImportLimits::default();
    if bytes.len() >= 8 && &bytes[0..8] == KDATA_MAGIC {
        let pw = password.filter(|s| !s.is_empty()).ok_or(AppError::BadRequest("decryption_password_required"))?;
        let mut entry_buffers: HashMap<u32, Vec<u8>> = HashMap::new();
        let opened = stream_read_kdata(&mut Cursor::new(bytes), pw, &limits, |entry, chunk, _| {
            entry_buffers.entry(entry.entry_id).or_default().extend_from_slice(chunk);
            Ok(())
        })
        .map_err(|e| match e {
            kiyo_kdata::KdataError::WrongPasswordOrTampered => AppError::Unauthorized,
            _ => AppError::BadRequest("invalid_or_corrupted_kdata"),
        })?;

        return assemble_decoded_bundle("kdata".into(), Some(opened.manifest), &opened.index, entry_buffers);
    }

    if bytes.len() >= 4 && (&bytes[0..4] == b"PK\x03\x04" || &bytes[0..4] == b"PK\x05\x06") {
        let extracted = read_zip_archive(Cursor::new(bytes), bytes.len() as u64, password, &limits)
            .map_err(|e| match e {
                kiyo_kdata::KdataError::WrongPasswordOrTampered => AppError::Unauthorized,
                kiyo_kdata::KdataError::ResourceLimitExceeded(_) => AppError::BadRequest("zip_bomb_rejected"),
                kiyo_kdata::KdataError::UnsafePath(_) => AppError::BadRequest("unsafe_path_rejected"),
                _ => AppError::BadRequest("invalid_zip_archive"),
            })?;

        let mut manifest: Option<KdataManifest> = None;
        let mut index = Vec::new();
        let mut entry_buffers = HashMap::new();
        for (i, item) in extracted.into_iter().enumerate() {
            if item.path == "manifest.json" {
                manifest = serde_json::from_slice::<KdataManifest>(&item.bytes).ok();
                continue;
            }
            let kind = if item.path == "account/profile.json" {
                KdataEntryKind::Account
            } else if item.path == "account/settings.json" {
                KdataEntryKind::Settings
            } else if item.path.starts_with("messages/") {
                KdataEntryKind::MessagesJson
            } else if item.path.starts_with("media/avatar") {
                KdataEntryKind::Avatar
            } else if item.path.starts_with("media/sticker") {
                KdataEntryKind::Sticker
            } else {
                KdataEntryKind::MediaBlob
            };
            let eid = i as u32;
            index.push(KdataIndexEntry {
                entry_id: eid,
                kind,
                logical_path: item.path,
                mime_type: "application/octet-stream".into(),
                plain_size: item.bytes.len() as u64,
                sha256_hex: item.sha256_hex,
                scope_type: None,
                scope_id: None,
                ref_id: None,
                availability_state: None,
            });
            entry_buffers.insert(eid, item.bytes);
        }
        return assemble_decoded_bundle("zip".into(), manifest, &index, entry_buffers);
    }

    Err(AppError::BadRequest("unsupported_backup_magic_bytes"))
}

fn assemble_decoded_bundle(
    format: String,
    manifest: Option<KdataManifest>,
    index: &[KdataIndexEntry],
    mut buffers: HashMap<u32, Vec<u8>>,
) -> Result<DecodedBackupBundle, AppError> {
    let mut messages = Vec::new();
    let mut media_files = Vec::new();
    let mut account = manifest.as_ref().map(|m| m.account.clone());
    let mut settings = None;
    let mut warnings = Vec::new();

    for entry in index {
        sanitize_relative_path(&entry.logical_path).map_err(|_| AppError::BadRequest("unsafe_path_rejected"))?;
        let data = buffers.remove(&entry.entry_id).unwrap_or_default();
        match entry.kind {
            KdataEntryKind::Account => {
                if let Ok(acc) = serde_json::from_slice::<ExportedAccountInfo>(&data) {
                    account = Some(acc);
                }
            }
            KdataEntryKind::Settings => {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&data) {
                    settings = Some(val);
                }
            }
            KdataEntryKind::MessagesJson => {
                if let Ok(mut list) = serde_json::from_slice::<Vec<ExportedMessageRecord>>(&data) {
                    messages.append(&mut list);
                } else {
                    warnings.push(format!("Unable to read message file {}", entry.logical_path));
                }
            }
            KdataEntryKind::MediaBlob | KdataEntryKind::Avatar | KdataEntryKind::Sticker => {
                media_files.push((entry.clone(), data));
            }
            KdataEntryKind::ConversationMeta | KdataEntryKind::SyncMetadata => {}
        }
    }

    Ok(DecodedBackupBundle {
        format,
        manifest,
        messages,
        media_files,
        account,
        settings,
        warnings,
    })
}

pub async fn inspect_backup_archive(
    pool: &PgPool,
    user_id: Uuid,
    archive_bytes: &[u8],
    password: Option<&str>,
) -> AppResult<InspectArchiveResponse> {
    let bytes_owned = archive_bytes.to_vec();
    let pw_owned = password.map(str::to_string);
    let decoded = tokio::task::spawn_blocking(move || decode_archive_bundle(&bytes_owned, pw_owned.as_deref()))
        .await
        .map_err(|_| AppError::Internal)??;

    let msg_ids: Vec<Uuid> = decoded.messages.iter().map(|m| m.id).collect();
    let existing_dm: HashSet<Uuid> = if msg_ids.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM messages WHERE id = ANY($1)")
            .bind(&msg_ids)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect()
    };
    let existing_grp: HashSet<Uuid> = if msg_ids.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM group_messages WHERE id = ANY($1)")
            .bind(&msg_ids)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect()
    };
    let existing_ch: HashSet<Uuid> = if msg_ids.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM channel_messages WHERE id = ANY($1)")
            .bind(&msg_ids)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect()
    };

    let mut dup_msgs = 0usize;
    for m in &decoded.messages {
        if existing_dm.contains(&m.id) || existing_grp.contains(&m.id) || existing_ch.contains(&m.id) {
            dup_msgs += 1;
        }
    }
    let new_msgs = decoded.messages.len().saturating_sub(dup_msgs);

    let mut dup_media = 0usize;
    for (entry, _) in &decoded.media_files {
        if let Ok(sha_bytes) = hex::decode(&entry.sha256_hex) {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM media_references WHERE owner_id = $1 AND sha256_plain = $2 AND object_id IS NOT NULL)",
            )
            .bind(user_id)
            .bind(&sha_bytes)
            .fetch_one(pool)
            .await?;
            if exists {
                dup_media += 1;
            }
        }
    }
    let new_media = decoded.media_files.len().saturating_sub(dup_media);

    let owner_mismatch = decoded
        .manifest
        .as_ref()
        .is_some_and(|m| m.owner_user_id != user_id);
    let mut warnings = decoded.warnings.clone();
    if owner_mismatch {
        warnings.push("The backup belongs to another account: the system will block unauthorized import to the current account.".into());
    }

    let total_plain_bytes: u64 = decoded
        .manifest
        .as_ref()
        .map(|m| m.total_plain_bytes)
        .unwrap_or_else(|| decoded.media_files.iter().map(|(e, _)| e.plain_size).sum());

    Ok(InspectArchiveResponse {
        detected_format: decoded.format,
        backup_id: decoded.manifest.as_ref().map(|m| m.backup_id),
        created_at: decoded.manifest.as_ref().map(|m| m.created_at.clone()),
        owner_user_id: decoded.manifest.as_ref().map(|m| m.owner_user_id),
        owner_mismatch,
        total_entries: decoded.messages.len() + decoded.media_files.len(),
        total_plain_bytes,
        total_messages: decoded.messages.len(),
        new_messages: new_msgs,
        duplicate_messages: dup_msgs,
        total_media: decoded.media_files.len(),
        new_media,
        duplicate_media: dup_media,
        missing_media_in_backup: decoded
            .manifest
            .as_ref()
            .map(|m| m.missing_media.clone())
            .unwrap_or_default(),
        integrity_verified: true,
        warnings,
    })
}

pub async fn restore_backup_archive(
    pool: &PgPool,
    storage: &StorageRegistry,
    user_id: Uuid,
    archive_bytes: &[u8],
    opts: &RestoreOptions,
) -> AppResult<RestoreResultResponse> {
    let bytes_owned = archive_bytes.to_vec();
    let pw_owned = opts.password.clone();
    let decoded = tokio::task::spawn_blocking(move || decode_archive_bundle(&bytes_owned, pw_owned.as_deref()))
        .await
        .map_err(|_| AppError::Internal)??;

    // Cross-account anti-impersonation check: never import another user's private backup into this account!
    if let Some(ref mf) = decoded.manifest {
        if mf.owner_user_id != user_id {
            return Err(AppError::Forbidden("backup_belongs_to_another_account"));
        }
    }

    let mode = opts.mode.as_str();
    let mut restored_messages = 0usize;
    let mut skipped_duplicate_messages = 0usize;
    let mut skipped_unauthorized_messages = 0usize;
    let mut restored_media = 0usize;
    let mut skipped_duplicate_media = 0usize;
    let mut restored_settings = false;
    let mut restored_avatar = false;
    let mut warnings = decoded.warnings;

    // 1. Settings & account profile (safe non-privileged fields only; never admin flags or tokens)
    if matches!(mode, "full" | "settings" | "avatar") {
        if matches!(mode, "full" | "avatar") {
            if let Some(ref acc) = decoded.account {
                if let Some(ref dn) = acc.display_name {
                    let _ = sqlx::query(
                        "UPDATE users SET display_name = COALESCE(display_name, $1), bio = COALESCE(bio, $2) WHERE id = $3",
                    )
                    .bind(dn)
                    .bind(&acc.bio)
                    .bind(user_id)
                    .execute(pool)
                    .await;
                }
                if let Some(ref av) = acc.avatar_url {
                    if av.starts_with("/uploads/avatars/") {
                        let _ = sqlx::query("UPDATE users SET avatar_url = COALESCE(avatar_url, $1) WHERE id = $2")
                            .bind(av)
                            .bind(user_id)
                            .execute(pool)
                            .await;
                        restored_avatar = true;
                    }
                }
            }
        }
        if matches!(mode, "full" | "settings") {
            if let Some(ref st) = decoded.settings {
                let mut clean_st = st.clone();
                if let Some(obj) = clean_st.as_object_mut() {
                    obj.remove("storage_policy");
                }
                let _ = sqlx::query(
                    "INSERT INTO user_settings (user_id, data, updated_at)
                     VALUES ($1, $2, now())
                     ON CONFLICT (user_id) DO NOTHING",
                )
                .bind(user_id)
                .bind(&clean_st)
                .execute(pool)
                .await;
                restored_settings = true;
            }
        }
    }

    // 2. Restore messages safely (dedup by stable ID, never forge sender_id or deleted records)
    if matches!(mode, "full" | "messages" | "single_conversation" | "missing_only") {
        for m in &decoded.messages {
            if m.deleted_for_all {
                skipped_unauthorized_messages += 1;
                continue;
            }
            if let Some(ref conv_filter) = opts.conversation_id {
                if conv_filter != &m.conversation_id && conv_filter != &format!("{}:{}", m.scope_type, m.conversation_id) {
                    continue;
                }
            }

            match m.scope_type.as_str() {
                "dm" => {
                    let Ok(partner_id) = Uuid::parse_str(&m.conversation_id) else {
                        skipped_unauthorized_messages += 1;
                        continue;
                    };
                    // Strict anti-forgery check: caller can only restore DMs involving themselves,
                    // and cannot fabricate new messages claiming to be authored by someone else unless
                    // that exact message ID already exists or the user is restoring their own self-chat/sent E2EE record.
                    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM messages WHERE id = $1)")
                        .bind(m.id)
                        .fetch_one(pool)
                        .await?;
                    if exists {
                        skipped_duplicate_messages += 1;
                        continue;
                    }
                    let Ok(primary_env_bytes) = B64.decode(&m.content_payload) else {
                        skipped_unauthorized_messages += 1;
                        continue;
                    };
                    let counterpart_env_bytes = m
                        .counterpart_envelope
                        .as_deref()
                        .and_then(|b| B64.decode(b).ok());

                    // Determine sender/receiver and respective envelopes, verifying ECDSA signature to prevent forgery.
                    let (msg_sender_id, msg_receiver_id, recipient_env_raw, sender_env_raw) =
                        if m.sender_id == user_id {
                            let s_env = primary_env_bytes;
                            let r_env = counterpart_env_bytes.unwrap_or_else(|| s_env.clone());
                            (user_id, partner_id, r_env, s_env)
                        } else if m.sender_id == partner_id {
                            let r_env = primary_env_bytes;
                            let s_env = counterpart_env_bytes.unwrap_or_else(|| r_env.clone());
                            (partner_id, user_id, r_env, s_env)
                        } else {
                            skipped_unauthorized_messages += 1;
                            continue;
                        };

                    let Ok(parsed_primary) = crate::crypto::Envelope::parse(if m.sender_id == user_id {
                        &sender_env_raw
                    } else {
                        &recipient_env_raw
                    }) else {
                        skipped_unauthorized_messages += 1;
                        continue;
                    };
                    if parsed_primary.message_id != m.id || parsed_primary.sender_id != msg_sender_id {
                        skipped_unauthorized_messages += 1;
                        continue;
                    }
                    // For incoming messages authored by another user, strictly verify ECDSA signature against sender's key (current or historical).
                    if msg_sender_id != user_id {
                        let mut sig_ok = false;
                        if let Ok(Some(ecdsa_pub)) = sqlx::query_scalar::<_, Vec<u8>>(
                            "SELECT ecdsa_public FROM device_keys WHERE user_id = $1",
                        )
                        .bind(msg_sender_id)
                        .fetch_optional(pool)
                        .await
                        {
                            if parsed_primary.verify_signature(&ecdsa_pub).is_ok() {
                                sig_ok = true;
                            }
                        }
                        if !sig_ok {
                            if let Ok(hist_keys) = sqlx::query_scalar::<_, Vec<u8>>(
                                "SELECT ecdsa_public FROM identity_key_history WHERE user_id = $1",
                            )
                            .bind(msg_sender_id)
                            .fetch_all(pool)
                            .await
                            {
                                sig_ok = hist_keys
                                    .iter()
                                    .any(|k| parsed_primary.verify_signature(k).is_ok());
                            }
                        }
                        if !sig_ok {
                            skipped_unauthorized_messages += 1;
                            continue;
                        }
                    }

                    let created_at = chrono::DateTime::parse_from_rfc3339(&m.created_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let res = sqlx::query(
                        "INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope, reply_to_id, is_edited, created_at)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                         ON CONFLICT (id) DO NOTHING",
                    )
                    .bind(m.id)
                    .bind(msg_sender_id)
                    .bind(msg_receiver_id)
                    .bind(&recipient_env_raw)
                    .bind(&sender_env_raw)
                    .bind(m.reply_to_id)
                    .bind(m.edited_at.is_some())
                    .bind(created_at)
                    .execute(pool)
                    .await;
                    match res {
                        Ok(done) if done.rows_affected() > 0 => restored_messages += 1,
                        Ok(_) => skipped_duplicate_messages += 1,
                        Err(_) => skipped_unauthorized_messages += 1,
                    }
                }
                "group" => {
                    let Ok(gid) = Uuid::parse_str(&m.conversation_id) else {
                        skipped_unauthorized_messages += 1;
                        continue;
                    };
                    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM group_messages WHERE id = $1)")
                        .bind(m.id)
                        .fetch_one(pool)
                        .await?;
                    if exists {
                        skipped_duplicate_messages += 1;
                        continue;
                    }
                    let is_member: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
                    )
                    .bind(gid)
                    .bind(user_id)
                    .fetch_one(pool)
                    .await?;
                    if !is_member || m.sender_id != user_id {
                        skipped_unauthorized_messages += 1;
                        continue;
                    }
                    let created_at = chrono::DateTime::parse_from_rfc3339(&m.created_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let res = sqlx::query(
                        "INSERT INTO group_messages (id, group_id, sender_id, content, reply_to_id, is_edited, created_at)
                         VALUES ($1, $2, $3, $4, $5, $6, $7)
                         ON CONFLICT (id) DO NOTHING",
                    )
                    .bind(m.id)
                    .bind(gid)
                    .bind(user_id)
                    .bind(&m.content_payload)
                    .bind(m.reply_to_id)
                    .bind(m.edited_at.is_some())
                    .bind(created_at)
                    .execute(pool)
                    .await;
                    match res {
                        Ok(done) if done.rows_affected() > 0 => restored_messages += 1,
                        Ok(_) => skipped_duplicate_messages += 1,
                        Err(_) => skipped_unauthorized_messages += 1,
                    }
                }
                "channel" => {
                    let Ok(cid) = Uuid::parse_str(&m.conversation_id) else {
                        skipped_unauthorized_messages += 1;
                        continue;
                    };
                    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM channel_messages WHERE id = $1)")
                        .bind(m.id)
                        .fetch_one(pool)
                        .await?;
                    if exists {
                        skipped_duplicate_messages += 1;
                        continue;
                    }
                    if m.sender_id != user_id
                        || crate::storage::permissions::authorize_channel_access(
                            pool,
                            user_id,
                            cid,
                            perm_flags::SEND_MESSAGES,
                        )
                        .await
                        .is_err()
                    {
                        skipped_unauthorized_messages += 1;
                        continue;
                    }
                    let created_at = chrono::DateTime::parse_from_rfc3339(&m.created_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let res = sqlx::query(
                        "INSERT INTO channel_messages (id, channel_id, sender_id, content, reply_to_id, is_edited, created_at)
                         VALUES ($1, $2, $3, $4, $5, $6, $7)
                         ON CONFLICT (id) DO NOTHING",
                    )
                    .bind(m.id)
                    .bind(cid)
                    .bind(user_id)
                    .bind(&m.content_payload)
                    .bind(m.reply_to_id)
                    .bind(m.edited_at.is_some())
                    .bind(created_at)
                    .execute(pool)
                    .await;
                    match res {
                        Ok(done) if done.rows_affected() > 0 => restored_messages += 1,
                        Ok(_) => skipped_duplicate_messages += 1,
                        Err(_) => skipped_unauthorized_messages += 1,
                    }
                }
                _ => skipped_unauthorized_messages += 1,
            }
        }
    }

    // 3. Selective media restore (dedup via CAS hash + stable ref_id)
    let want_media = matches!(mode, "full" | "images" | "videos" | "audio" | "files" | "single_conversation" | "missing_only");
    if want_media {
        for (entry, data) in &decoded.media_files {
            let m = entry.mime_type.as_str();
            let matches_filter = match mode {
                "images" => m.starts_with("image/"),
                "videos" => m.starts_with("video/"),
                "audio" => m.starts_with("audio/"),
                "files" => !m.starts_with("image/") && !m.starts_with("video/") && !m.starts_with("audio/"),
                _ => true,
            };
            if !matches_filter {
                continue;
            }
            if let Some(ref conv_filter) = opts.conversation_id {
                let s_str = entry.scope_id.map(|u| u.to_string()).unwrap_or_default();
                let st_str = entry.scope_type.as_deref().unwrap_or("");
                if conv_filter != &s_str && conv_filter != &format!("{st_str}:{s_str}") {
                    continue;
                }
            }

            let sha_arr = sha256_bytes(data);
            let existing_ref: Option<Uuid> = if let Some(rid) = entry.ref_id {
                sqlx::query_scalar(
                    "SELECT id FROM media_references WHERE id = $1 AND object_id IS NOT NULL",
                )
                .bind(rid)
                .fetch_optional(pool)
                .await?
            } else {
                None
            };
            let existing_by_sha: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM media_references WHERE owner_id = $1 AND sha256_plain = $2 AND object_id IS NOT NULL LIMIT 1",
            )
            .bind(user_id)
            .bind(&sha_arr[..])
            .fetch_optional(pool)
            .await?;

            if existing_ref.is_some() || existing_by_sha.is_some() {
                skipped_duplicate_media += 1;
                continue;
            }

            let scope_type = entry.scope_type.as_deref().unwrap_or("dm");
            if !matches!(scope_type, "dm" | "group" | "channel" | "avatar" | "banner" | "library" | "sticker") {
                continue;
            }
            // Enforce RBAC if restoring into group or channel
            if let Some(sid) = entry.scope_id {
                if crate::storage::permissions::authorize_write(pool, user_id, scope_type, Some(sid))
                    .await
                    .is_err()
                {
                    skipped_unauthorized_messages += 1;
                    continue;
                }
            }

            let ext = entry.logical_path.rsplit('.').next().unwrap_or("bin");
            let cas_key = LocalFileStorageAdapter::cas_key_for_sha(&sha_arr, ext);
            let cas_scope = format!("{scope_type}:restored:{}:{}", user_id, hex::encode(&sha_arr));
            let stored = storage
                .local_cas
                .put_object(&cas_key, data.clone(), &entry.mime_type, None)
                .await
                .map_err(|_| AppError::Internal)?;

            let obj_id: Uuid = sqlx::query_scalar(
                "INSERT INTO media_objects (cas_scope, sha256, plain_sha256, storage_backend, storage_key, mime_type, file_ext, byte_size, original_byte_size)
                 VALUES ($1, $2, $2, 'local_cas', $3, $4, $5, $6, $6)
                 ON CONFLICT (cas_scope, variant_kind) DO UPDATE SET ref_count = media_objects.ref_count + 1
                 RETURNING id",
            )
            .bind(&cas_scope)
            .bind(&sha_arr[..])
            .bind(&stored.storage_key)
            .bind(&entry.mime_type)
            .bind(ext)
            .bind(data.len() as i64)
            .fetch_one(pool)
            .await?;

            let ref_id = entry.ref_id.unwrap_or_else(Uuid::new_v4);
            let filename = entry.logical_path.rsplit('/').next().unwrap_or("restored.bin");
            sqlx::query(
                "INSERT INTO media_references (id, owner_id, scope_type, scope_id, object_id, original_filename, mime_type, byte_size, sha256_plain, storage_tier, availability_state)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'server_durable', 'backed_up_encrypted')
                 ON CONFLICT (id) DO UPDATE SET object_id = COALESCE(media_references.object_id, EXCLUDED.object_id), availability_state = 'backed_up_encrypted', updated_at = now()",
            )
            .bind(ref_id)
            .bind(user_id)
            .bind(scope_type)
            .bind(entry.scope_id)
            .bind(obj_id)
            .bind(filename)
            .bind(&entry.mime_type)
            .bind(data.len() as i64)
            .bind(&sha_arr[..])
            .execute(pool)
            .await?;

            restored_media += 1;
        }
    }

    if skipped_unauthorized_messages > 0 {
        warnings.push(format!(
            "Ignored {skipped_unauthorized_messages} items that are not owned or do not have restore permissions on the server."
        ));
    }

    Ok(RestoreResultResponse {
        backup_id: decoded.manifest.as_ref().map(|m| m.backup_id),
        mode: opts.mode.clone(),
        restored_messages,
        skipped_duplicate_messages,
        skipped_unauthorized_messages,
        restored_media,
        skipped_duplicate_media,
        restored_settings,
        restored_avatar,
        integrity_verified: true,
        warnings,
    })
}
