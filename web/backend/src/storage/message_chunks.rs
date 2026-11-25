//! Spec 164: Incremental Zstd-22 Message Chunk Storage, Distributed Leases, OCC Manifests,
//! Mutation/Tombstone Overlays, Versioned Dictionaries, Encrypted Search Index Sync,
//! Hot/Warm/Cold Tier Lifecycle, and Privacy-Safe Telemetry.
//!
//! Security Invariants:
//! - E2EE DM chunks are compressed & encrypted EXCLUSIVELY on authorized user devices; server
//!   never sees plaintext DM content, keys, or search queries.
//! - Plaintext group/channel >=7d messages may be compacted incrementally by server workers
//!   under distributed leases using mandatory Zstd Level 22 (`chatw-msg-store`).
//! - Strict scope RBAC enforced on every endpoint (DM friendship/self-chat, Group membership,
//!   Channel `VIEW_CHANNELS` + account owner isolation for personal search index segments).
//! - Atomic replacement: verify object durability + BLAKE3 + OCC `manifest_version` BEFORE
//!   committing manifest or scheduling superseded cleanup.

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    storage::{StorageAdapter, StorageRegistry},
};
use chatw_msg_store::{
    CompactMessage, DICT_ID_NONE, DICT_ID_PUBLIC_CHAT_V1, DictionaryRegistry,
    MANDATORY_ZSTD_LEVEL, MessageType, ScopeKind, Zstd22Options, compress_zstd22_best_dict,
    public_chat_dict_v1, serialize_chunk_messages,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // Message Chunk Manifest, Eligibility, Leases, Atomic OCC Commit & Retrieval
        .route("/msg-chunks/manifest", get(handle_get_manifest))
        .route("/msg-chunks/eligible", get(handle_list_eligible_messages))
        .route("/msg-chunks/lease/acquire", post(handle_acquire_lease))
        .route("/msg-chunks/lease/renew", post(handle_renew_lease))
        .route("/msg-chunks/lease/release", post(handle_release_lease))
        .route(
            "/msg-chunks/commit",
            post(handle_commit_chunk).layer(DefaultBodyLimit::max(32 * 1024 * 1024)),
        )
        .route("/msg-chunks/{chunk_id}", get(handle_download_chunk))
        // Append-only Mutation & Tombstone Log
        .route(
            "/msg-chunks/mutations",
            get(handle_list_mutations).post(handle_append_mutation),
        )
        // Versioned Zstd Dictionary Registry
        .route(
            "/msg-chunks/dictionaries",
            get(handle_list_dictionaries).post(handle_upload_private_dictionary),
        )
        .route("/msg-chunks/dictionaries/{dict_id}", get(handle_get_dictionary))
        // Encrypted Search Index Multi-Device Sync (Zero-Knowledge Ciphertext Shards)
        .route("/search-index/segments", get(handle_list_search_segments))
        .route(
            "/search-index/segments/{scope_key}/{shard_index}",
            get(handle_get_search_segment)
                .put(handle_put_search_segment)
                .delete(handle_delete_search_segment)
                .layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        )
        // Privacy-safe technical telemetry (Section 32)
        .route("/telemetry", get(handle_get_telemetry).post(handle_post_telemetry))
}

async fn auth_user(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<crate::auth::session::AuthUser> {
    let token = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    authenticate_session(&state.db, &state.config, token).await
}

/// Normalize canonical DM scope ID (sorted pair `min_uuid:max_uuid`) and enforce strict RBAC.
async fn authorize_message_scope(
    pool: &PgPool,
    user_id: Uuid,
    scope_type: &str,
    scope_id_raw: &str,
) -> AppResult<(ScopeKind, String, String)> {
    let kind = ScopeKind::from_str_checked(scope_type)
        .ok_or(AppError::BadRequest("invalid_scope_type"))?;
    match kind {
        ScopeKind::Dm => {
            let peer_id =
                Uuid::parse_str(scope_id_raw).map_err(|_| AppError::BadRequest("invalid_peer_id"))?;
            let is_self = user_id == peer_id;
            let u_a = user_id.min(peer_id);
            let u_b = user_id.max(peer_id);
            if !is_self {
                let is_friend: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM friendships WHERE user_a = $1 AND user_b = $2)",
                )
                .bind(u_a)
                .bind(u_b)
                .fetch_one(pool)
                .await?;
                if !is_friend {
                    return Err(AppError::Forbidden("not_friends"));
                }
            }
            // DM chunks are E2EE encrypted per user's own root key (owner_scope_key = user_id),
            // strictly isolating each user's encrypted vault & preventing cross-account overwrites!
            Ok((kind, format!("{u_a}:{u_b}"), user_id.to_string()))
        }
        ScopeKind::Group => {
            let group_id = Uuid::parse_str(scope_id_raw)
                .map_err(|_| AppError::BadRequest("invalid_group_id"))?;
            let is_member: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
            )
            .bind(group_id)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
            if !is_member {
                return Err(AppError::Forbidden("not_a_group_member"));
            }
            Ok((kind, group_id.to_string(), "shared".to_string()))
        }
        ScopeKind::Channel => {
            let channel_id = Uuid::parse_str(scope_id_raw)
                .map_err(|_| AppError::BadRequest("invalid_channel_id"))?;
            let allowed: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM channels c
                    JOIN server_members sm ON sm.server_id = c.server_id
                    WHERE c.id = $1 AND sm.user_id = $2
                )
                "#,
            )
            .bind(channel_id)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
            if !allowed {
                return Err(AppError::Forbidden("channel_access_denied"));
            }
            Ok((kind, channel_id.to_string(), "shared".to_string()))
        }
    }
}

/// Helper used by `messages/mod.rs` and `groups/mod.rs` to record an append-only mutation/tombstone
/// whenever a message is edited, deleted, recalled, pinned, or reacted to!
pub async fn record_message_mutation(
    pool: &PgPool,
    scope_type: &str,
    scope_id: &str,
    message_id: Uuid,
    op_type: &str,
    actor_id: Uuid,
    payload_enc_b64: Option<&str>,
    payload_plain: Option<serde_json::Value>,
) {
    let chunk_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM message_chunks
        WHERE scope_type = $1 AND scope_id = $2 AND superseded_at IS NULL
        ORDER BY seq_end DESC LIMIT 1
        "#,
    )
    .bind(scope_type)
    .bind(scope_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let _ = sqlx::query(
        r#"
        INSERT INTO message_mutations
            (scope_type, scope_id, message_id, chunk_id, op_type, actor_id, payload_enc_b64, payload_plain)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(scope_type)
    .bind(scope_id)
    .bind(message_id)
    .bind(chunk_id)
    .bind(op_type)
    .bind(actor_id)
    .bind(payload_enc_b64)
    .bind(payload_plain)
    .execute(pool)
    .await;
}

// ── 1. Manifest Query ────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ManifestQuery {
    scope_type: String,
    scope_id: String,
    since_mutation_seq: Option<i64>,
}

async fn handle_get_manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ManifestQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let (kind, norm_scope_id, owner_scope_key) =
        authorize_message_scope(&state.db, user.id, &q.scope_type, &q.scope_id).await?;

    let head = sqlx::query(
        r#"
        SELECT manifest_version, chunk_count, oldest_chunk_seq, newest_chunk_seq,
               last_mutation_seq, total_original_bytes, total_compressed_bytes,
               total_encrypted_bytes, sealed_manifest_b64, updated_at
        FROM conversation_chunk_manifests
        WHERE scope_type = $1 AND scope_id = $2 AND owner_scope_key = $3
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .fetch_optional(&state.db)
    .await?;

    let chunks_rows = sqlx::query(
        r#"
        SELECT id, chunk_version, format_version, access_epoch, compression_algo, compression_level,
               dictionary_id, original_size, serialized_size, compressed_size, padded_size, encrypted_size,
               message_count, seq_start, seq_end, first_created_at, last_created_at,
               encryption_version, key_version, is_e2ee, nonce_b64, plaintext_blake3_hex,
               ciphertext_blake3_hex, storage_tier, sync_state, sealed_meta_b64, created_at, updated_at
        FROM message_chunks
        WHERE scope_type = $1 AND scope_id = $2 AND owner_scope_key = $3 AND superseded_at IS NULL
        ORDER BY seq_start ASC
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .fetch_all(&state.db)
    .await?;

    let mut chunks = Vec::with_capacity(chunks_rows.len());
    for r in chunks_rows {
        let first_at: DateTime<Utc> = r.try_get("first_created_at")?;
        let last_at: DateTime<Utc> = r.try_get("last_created_at")?;
        let created_at: DateTime<Utc> = r.try_get("created_at")?;
        let updated_at: DateTime<Utc> = r.try_get("updated_at")?;
        chunks.push(serde_json::json!({
            "chunk_id": r.try_get::<Uuid, _>("id")?,
            "chunk_version": r.try_get::<i32, _>("chunk_version")?,
            "format_version": r.try_get::<i16, _>("format_version")?,
            "access_epoch": r.try_get::<i32, _>("access_epoch")?,
            "compression_algo": r.try_get::<String, _>("compression_algo")?,
            "compression_level": r.try_get::<i16, _>("compression_level")?,
            "dictionary_id": r.try_get::<i32, _>("dictionary_id")?,
            "original_size": r.try_get::<i64, _>("original_size")?,
            "serialized_size": r.try_get::<i64, _>("serialized_size")?,
            "compressed_size": r.try_get::<i64, _>("compressed_size")?,
            "padded_size": r.try_get::<i64, _>("padded_size")?,
            "encrypted_size": r.try_get::<i64, _>("encrypted_size")?,
            "message_count": r.try_get::<i32, _>("message_count")?,
            "seq_start": r.try_get::<i64, _>("seq_start")?,
            "seq_end": r.try_get::<i64, _>("seq_end")?,
            "first_created_at_ms": first_at.timestamp_millis(),
            "last_created_at_ms": last_at.timestamp_millis(),
            "created_at_ms": created_at.timestamp_millis(),
            "updated_at_ms": updated_at.timestamp_millis(),
            "encryption_version": r.try_get::<i16, _>("encryption_version")?,
            "key_version": r.try_get::<i32, _>("key_version")?,
            "is_e2ee": r.try_get::<bool, _>("is_e2ee")?,
            "nonce_b64": r.try_get::<String, _>("nonce_b64")?,
            "plaintext_blake3_hex": r.try_get::<String, _>("plaintext_blake3_hex")?,
            "ciphertext_blake3_hex": r.try_get::<String, _>("ciphertext_blake3_hex")?,
            "storage_tier": r.try_get::<String, _>("storage_tier")?,
            "sync_state": r.try_get::<String, _>("sync_state")?,
            "sealed_meta_b64": r.try_get::<Option<String>, _>("sealed_meta_b64")?,
        }));
    }

    let since_mut = q.since_mutation_seq.unwrap_or(0);
    let mut_rows = sqlx::query(
        r#"
        SELECT mutation_seq, message_id, chunk_id, op_type, actor_id, payload_enc_b64, payload_plain, created_at
        FROM message_mutations
        WHERE scope_type = $1 AND scope_id = $2 AND mutation_seq > $3
        ORDER BY mutation_seq ASC
        LIMIT 500
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(since_mut)
    .fetch_all(&state.db)
    .await?;

    let mutations: Vec<serde_json::Value> = mut_rows
        .into_iter()
        .map(|r| {
            let ts: DateTime<Utc> = r.try_get("created_at").unwrap_or_else(|_| Utc::now());
            serde_json::json!({
                "mutation_seq": r.try_get::<i64, _>("mutation_seq").unwrap_or(0),
                "message_id": r.try_get::<Uuid, _>("message_id").ok(),
                "chunk_id": r.try_get::<Option<Uuid>, _>("chunk_id").ok().flatten(),
                "op_type": r.try_get::<String, _>("op_type").unwrap_or_default(),
                "actor_id": r.try_get::<Uuid, _>("actor_id").ok(),
                "payload_enc_b64": r.try_get::<Option<String>, _>("payload_enc_b64").ok().flatten(),
                "payload_plain": r.try_get::<Option<serde_json::Value>, _>("payload_plain").ok().flatten(),
                "created_at_ms": ts.timestamp_millis(),
            })
        })
        .collect();

    let manifest_version: i64 = head
        .as_ref()
        .and_then(|h| h.try_get("manifest_version").ok())
        .unwrap_or(0);
    let total_original: i64 = head
        .as_ref()
        .and_then(|h| h.try_get("total_original_bytes").ok())
        .unwrap_or(0);
    let total_compressed: i64 = head
        .as_ref()
        .and_then(|h| h.try_get("total_compressed_bytes").ok())
        .unwrap_or(0);
    let total_encrypted: i64 = head
        .as_ref()
        .and_then(|h| h.try_get("total_encrypted_bytes").ok())
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "scope_type": kind.as_str(),
        "scope_id": norm_scope_id,
        "manifest_version": manifest_version,
        "chunk_count": chunks.len(),
        "total_original_bytes": total_original,
        "total_compressed_bytes": total_compressed,
        "total_encrypted_bytes": total_encrypted,
        "saved_bytes": (total_original - total_encrypted).max(0),
        "chunks": chunks,
        "mutations": mutations,
    })))
}

// ── 2. List Eligible (>= 7 Days Old) Unsealed Messages for Incremental Compression ───────────────

#[derive(Deserialize)]
struct EligibleQuery {
    scope_type: String,
    scope_id: String,
    after_seq: Option<i64>,
    limit: Option<i64>,
}

async fn handle_list_eligible_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<EligibleQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let (kind, norm_scope_id, owner_scope_key) =
        authorize_message_scope(&state.db, user.id, &q.scope_type, &q.scope_id).await?;

    let max_sealed_seq: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(seq_end), 0)
        FROM message_chunks
        WHERE scope_type = $1 AND scope_id = $2 AND owner_scope_key = $3 AND superseded_at IS NULL
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .fetch_one(&state.db)
    .await?;

    let after_seq = q.after_seq.unwrap_or(0).max(max_sealed_seq);
    // Strict per-tick bound (Section 3: never returns millions of messages at once!)
    let limit = q.limit.unwrap_or(500).clamp(1, 2_000);

    match kind {
        ScopeKind::Dm => {
            let peer_id = Uuid::parse_str(&q.scope_id).map_err(|_| AppError::BadRequest("invalid_peer_id"))?;
            let rows = sqlx::query(
                r#"
                SELECT id, seq, sender_id, receiver_id, recipient_envelope, sender_envelope,
                       reply_to_id, is_pinned, is_edited, deleted_for_all, created_at
                FROM messages
                WHERE ((sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1))
                  AND seq > $3
                  AND created_at <= now() - interval '7 days'
                ORDER BY seq ASC
                LIMIT $4
                "#,
            )
            .bind(user.id)
            .bind(peer_id)
            .bind(after_seq)
            .bind(limit)
            .fetch_all(&state.db)
            .await?;

            let mut items = Vec::with_capacity(rows.len());
            for r in rows {
                let sender_id: Uuid = r.try_get("sender_id")?;
                let raw_env: Vec<u8> = if sender_id == user.id {
                    r.try_get("sender_envelope")?
                } else {
                    r.try_get("recipient_envelope")?
                };
                let created_at: DateTime<Utc> = r.try_get("created_at")?;
                items.push(serde_json::json!({
                    "id": r.try_get::<Uuid, _>("id")?,
                    "seq": r.try_get::<i64, _>("seq")?,
                    "sender_id": sender_id,
                    "envelope": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_env),
                    "reply_to_id": r.try_get::<Option<Uuid>, _>("reply_to_id")?,
                    "is_pinned": r.try_get::<bool, _>("is_pinned")?,
                    "is_edited": r.try_get::<bool, _>("is_edited")?,
                    "deleted_for_all": r.try_get::<bool, _>("deleted_for_all")?,
                    "created_at_ms": created_at.timestamp_millis(),
                }));
            }
            Ok(Json(serde_json::json!({
                "scope_type": "dm",
                "scope_id": norm_scope_id,
                "after_seq": after_seq,
                "min_age_days": 7,
                "eligible_count": items.len(),
                "items": items,
            })))
        }
        ScopeKind::Group => {
            let gid = Uuid::parse_str(&norm_scope_id).map_err(|_| AppError::BadRequest("invalid_group_id"))?;
            let rows = sqlx::query(
                r#"
                SELECT id, seq, sender_id, content, reply_to_id, is_pinned, is_edited, created_at
                FROM group_messages
                WHERE group_id = $1 AND seq > $2 AND created_at <= now() - interval '7 days'
                ORDER BY seq ASC
                LIMIT $3
                "#,
            )
            .bind(gid)
            .bind(after_seq)
            .bind(limit)
            .fetch_all(&state.db)
            .await?;

            let items: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    let ts: DateTime<Utc> = r.try_get("created_at").unwrap_or_else(|_| Utc::now());
                    serde_json::json!({
                        "id": r.try_get::<Uuid, _>("id").ok(),
                        "seq": r.try_get::<i64, _>("seq").unwrap_or(0),
                        "sender_id": r.try_get::<Uuid, _>("sender_id").ok(),
                        "text": r.try_get::<String, _>("content").unwrap_or_default(),
                        "reply_to_id": r.try_get::<Option<Uuid>, _>("reply_to_id").ok().flatten(),
                        "is_pinned": r.try_get::<bool, _>("is_pinned").unwrap_or(false),
                        "is_edited": r.try_get::<bool, _>("is_edited").unwrap_or(false),
                        "created_at_ms": ts.timestamp_millis(),
                    })
                })
                .collect();
            Ok(Json(serde_json::json!({
                "scope_type": "group",
                "scope_id": norm_scope_id,
                "after_seq": after_seq,
                "min_age_days": 7,
                "eligible_count": items.len(),
                "items": items,
            })))
        }
        ScopeKind::Channel => {
            let cid = Uuid::parse_str(&norm_scope_id).map_err(|_| AppError::BadRequest("invalid_channel_id"))?;
            let rows = sqlx::query(
                r#"
                SELECT id, seq, sender_id, content, reply_to_id, is_pinned, is_edited, created_at
                FROM channel_messages
                WHERE channel_id = $1 AND seq > $2 AND created_at <= now() - interval '7 days'
                ORDER BY seq ASC
                LIMIT $3
                "#,
            )
            .bind(cid)
            .bind(after_seq)
            .bind(limit)
            .fetch_all(&state.db)
            .await?;

            let items: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    let ts: DateTime<Utc> = r.try_get("created_at").unwrap_or_else(|_| Utc::now());
                    serde_json::json!({
                        "id": r.try_get::<Uuid, _>("id").ok(),
                        "seq": r.try_get::<i64, _>("seq").unwrap_or(0),
                        "sender_id": r.try_get::<Uuid, _>("sender_id").ok(),
                        "text": r.try_get::<String, _>("content").unwrap_or_default(),
                        "reply_to_id": r.try_get::<Option<Uuid>, _>("reply_to_id").ok().flatten(),
                        "is_pinned": r.try_get::<bool, _>("is_pinned").unwrap_or(false),
                        "is_edited": r.try_get::<bool, _>("is_edited").unwrap_or(false),
                        "created_at_ms": ts.timestamp_millis(),
                    })
                })
                .collect();
            Ok(Json(serde_json::json!({
                "scope_type": "channel",
                "scope_id": norm_scope_id,
                "after_seq": after_seq,
                "min_age_days": 7,
                "eligible_count": items.len(),
                "items": items,
            })))
        }
    }
}

// ── 3. Distributed Leases with Monotonic Fencing Tokens & Checkpoints (Section 20) ───────────────

#[derive(Deserialize)]
struct LeaseAcquireReq {
    scope_type: String,
    scope_id: String,
    device_id: String,
    ttl_secs: Option<i64>,
}

async fn handle_acquire_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LeaseAcquireReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let (kind, norm_scope_id, owner_scope_key) =
        authorize_message_scope(&state.db, user.id, &req.scope_type, &req.scope_id).await?;
    let lease_key = format!("chunk:{}:{norm_scope_id}:{owner_scope_key}", kind.as_str());
    let ttl = req.ttl_secs.unwrap_or(120).clamp(15, 600);

    let row = sqlx::query(
        r#"
        INSERT INTO compression_leases (lease_key, holder_device_id, holder_user_id, fencing_token, status, leased_until, updated_at)
        VALUES ($1, $2, $3, 1, 'active', now() + make_interval(secs => $4), now())
        ON CONFLICT (lease_key) DO UPDATE SET
            holder_device_id = EXCLUDED.holder_device_id,
            holder_user_id = EXCLUDED.holder_user_id,
            fencing_token = compression_leases.fencing_token + 1,
            status = 'active',
            leased_until = EXCLUDED.leased_until,
            updated_at = now()
        WHERE compression_leases.leased_until < now()
           OR compression_leases.status = 'released'
           OR (compression_leases.holder_device_id = $2 AND compression_leases.holder_user_id = $3)
        RETURNING fencing_token, checkpoint_json, leased_until
        "#,
    )
    .bind(&lease_key)
    .bind(req.device_id.trim())
    .bind(user.id)
    .bind(ttl as f64)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some(r) => {
            let until: DateTime<Utc> = r.try_get("leased_until")?;
            Ok(Json(serde_json::json!({
                "acquired": true,
                "lease_key": lease_key,
                "fencing_token": r.try_get::<i64, _>("fencing_token")?,
                "checkpoint": r.try_get::<serde_json::Value, _>("checkpoint_json")?,
                "leased_until_ms": until.timestamp_millis(),
            })))
        }
        None => Ok(Json(serde_json::json!({
            "acquired": false,
            "lease_key": lease_key,
            "reason": "lease_held_by_another_device",
        }))),
    }
}

#[derive(Deserialize)]
struct LeaseRenewReq {
    lease_key: String,
    device_id: String,
    fencing_token: i64,
    ttl_secs: Option<i64>,
    checkpoint: Option<serde_json::Value>,
    status: Option<String>,
}

async fn handle_renew_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LeaseRenewReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let ttl = req.ttl_secs.unwrap_or(120).clamp(15, 600);
    let next_status = match req.status.as_deref() {
        Some("paused") => "paused",
        _ => "active",
    };

    let updated = sqlx::query(
        r#"
        UPDATE compression_leases
        SET leased_until = now() + make_interval(secs => $5),
            checkpoint_json = COALESCE($6, checkpoint_json),
            status = $7,
            updated_at = now()
        WHERE lease_key = $1 AND holder_device_id = $2 AND holder_user_id = $3 AND fencing_token = $4
        RETURNING fencing_token, leased_until
        "#,
    )
    .bind(&req.lease_key)
    .bind(req.device_id.trim())
    .bind(user.id)
    .bind(req.fencing_token)
    .bind(ttl as f64)
    .bind(req.checkpoint)
    .bind(next_status)
    .fetch_optional(&state.db)
    .await?;

    match updated {
        Some(r) => {
            let until: DateTime<Utc> = r.try_get("leased_until")?;
            Ok(Json(serde_json::json!({
                "renewed": true,
                "fencing_token": r.try_get::<i64, _>("fencing_token")?,
                "leased_until_ms": until.timestamp_millis(),
            })))
        }
        None => Err(AppError::Conflict("lease_fencing_token_stale")),
    }
}

#[derive(Deserialize)]
struct LeaseReleaseReq {
    lease_key: String,
    device_id: String,
    fencing_token: i64,
    checkpoint: Option<serde_json::Value>,
}

async fn handle_release_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LeaseReleaseReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let res = sqlx::query(
        r#"
        UPDATE compression_leases
        SET status = 'released',
            leased_until = now(),
            checkpoint_json = COALESCE($5, checkpoint_json),
            updated_at = now()
        WHERE lease_key = $1 AND holder_device_id = $2 AND holder_user_id = $3 AND fencing_token = $4
        "#,
    )
    .bind(&req.lease_key)
    .bind(req.device_id.trim())
    .bind(user.id)
    .bind(req.fencing_token)
    .bind(req.checkpoint)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "released": res.rows_affected() > 0 })))
}

// ── 4. Atomic OCC Chunk Commit (Section 25.1) ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CommitChunkReq {
    scope_type: String,
    scope_id: String,
    expected_manifest_version: i64,
    lease_key: Option<String>,
    fencing_token: Option<i64>,
    chunk_id: Option<Uuid>,
    chunk_version: Option<i32>,
    format_version: Option<i16>,
    access_epoch: Option<i32>,
    compression_level: i16,
    dictionary_id: Option<i32>,
    original_size: i64,
    serialized_size: i64,
    compressed_size: i64,
    padded_size: i64,
    message_count: i32,
    seq_start: i64,
    seq_end: i64,
    first_created_at_ms: i64,
    last_created_at_ms: i64,
    encryption_version: Option<i16>,
    key_version: Option<i32>,
    is_e2ee: Option<bool>,
    nonce_b64: String,
    plaintext_blake3_hex: String,
    ciphertext_blake3_hex: String,
    sealed_meta_b64: Option<String>,
    sealed_manifest_b64: Option<String>,
    supersedes_chunk_ids: Option<Vec<Uuid>>,
    payload_b64: String,
}

async fn handle_commit_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CommitChunkReq>,
) -> AppResult<Response> {
    let user = auth_user(&state, &headers).await?;
    let (kind, norm_scope_id, owner_scope_key) =
        authorize_message_scope(&state.db, user.id, &req.scope_type, &req.scope_id).await?;

    // Strict invariants: mandatory Zstd Level 22 & strict 7-day age rule!
    if req.compression_level as i32 != MANDATORY_ZSTD_LEVEL {
        return Err(AppError::BadRequest("mandatory_zstd_level_22_required"));
    }
    if req.message_count <= 0 || req.seq_end < req.seq_start || req.original_size <= 0 {
        return Err(AppError::BadRequest("invalid_chunk_bounds"));
    }
    let cutoff_ms = (Utc::now() - chrono::Duration::days(7)).timestamp_millis();
    if req.last_created_at_ms > cutoff_ms {
        return Err(AppError::BadRequest("messages_younger_than_7_days_cannot_be_compressed"));
    }

    let raw_bytes = B64
        .decode(req.payload_b64.trim())
        .map_err(|_| AppError::BadRequest("invalid_payload_b64"))?;
    if raw_bytes.is_empty() {
        return Err(AppError::BadRequest("empty_chunk_payload"));
    }
    // Never replace storage if chunk did not actually save space!
    if (raw_bytes.len() as i64) >= req.original_size {
        return Err(AppError::BadRequest("chunk_must_reduce_storage_size"));
    }

    // Verify ciphertext BLAKE3 BEFORE touching storage or DB
    let actual_ct_b3 = blake3::hash(&raw_bytes).to_hex().to_string();
    if actual_ct_b3 != req.ciphertext_blake3_hex.trim().to_ascii_lowercase() {
        return Err(AppError::BadRequest("ciphertext_blake3_mismatch"));
    }

    // Verify lease fencing token if provided
    if let (Some(lk), Some(ft)) = (req.lease_key.as_deref(), req.fencing_token) {
        let valid: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM compression_leases
                WHERE lease_key = $1 AND holder_user_id = $2 AND fencing_token = $3 AND leased_until >= now()
            )
            "#,
        )
        .bind(lk)
        .bind(user.id)
        .bind(ft)
        .fetch_one(&state.db)
        .await?;
        if !valid {
            return Err(AppError::Conflict("stale_or_expired_lease"));
        }
    }

    let chunk_id = req.chunk_id.unwrap_or_else(Uuid::new_v4);
    let storage_key = format!(
        "msgchunks/{}/{}/{}.cwec",
        &actual_ct_b3[..2],
        &actual_ct_b3[2..4],
        chunk_id
    );

    // Step 1 (Section 25.1): Store chunk blob durably BEFORE committing manifest transaction
    let (storage_backend, stored_meta) = if state.storage.s3.config().is_ready() {
        let m = state
            .storage
            .s3
            .put_object(&storage_key, raw_bytes.clone(), "application/octet-stream", None)
            .await
            .map_err(|_| AppError::Internal)?;
        ("s3_ready", m)
    } else {
        let m = state
            .storage
            .local_cas
            .put_object(&storage_key, raw_bytes.clone(), "application/octet-stream", None)
            .await
            .map_err(|_| AppError::Internal)?;
        ("local_cas", m)
    };

    // Step 2: Atomic PostgreSQL Transaction with OCC (`manifest_version` Compare-And-Swap!)
    let mut tx = state.db.begin().await?;

    let existing_ver: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT manifest_version FROM conversation_chunk_manifests
        WHERE scope_type = $1 AND scope_id = $2 AND owner_scope_key = $3
        FOR UPDATE
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .fetch_optional(&mut *tx)
    .await?;

    let cur_ver = existing_ver.unwrap_or(0);
    if cur_ver != req.expected_manifest_version {
        tx.rollback().await?;
        // Clean up orphan blob we just uploaded since commit conflicted
        let _ = match storage_backend {
            "s3_ready" => state.storage.s3.delete_object(&stored_meta.storage_key).await,
            _ => state.storage.local_cas.delete_object(&stored_meta.storage_key).await,
        };
        return Ok((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "manifest_version_conflict",
                "expected": req.expected_manifest_version,
                "current_manifest_version": cur_ver,
            })),
        )
            .into_response());
    }

    let first_at = DateTime::from_timestamp_millis(req.first_created_at_ms).unwrap_or_else(Utc::now);
    let last_at = DateTime::from_timestamp_millis(req.last_created_at_ms).unwrap_or_else(Utc::now);

    sqlx::query(
        r#"
        INSERT INTO message_chunks (
            id, scope_type, scope_id, owner_scope_key, owner_id, chunk_version, format_version,
            access_epoch, compression_algo, compression_level, dictionary_id,
            original_size, serialized_size, compressed_size, padded_size, encrypted_size,
            message_count, seq_start, seq_end, first_created_at, last_created_at,
            encryption_version, key_version, is_e2ee, nonce_b64, plaintext_blake3_hex,
            ciphertext_blake3_hex, storage_tier, storage_backend, storage_key, sync_state, sealed_meta_b64
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, 'zstd', $9, $10,
            $11, $12, $13, $14, $15,
            $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25,
            $26, 'warm', $27, $28, 'committed', $29
        )
        "#,
    )
    .bind(chunk_id)
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .bind(user.id)
    .bind(req.chunk_version.unwrap_or(1))
    .bind(req.format_version.unwrap_or(1))
    .bind(req.access_epoch.unwrap_or(1))
    .bind(req.compression_level)
    .bind(req.dictionary_id.unwrap_or(0))
    .bind(req.original_size)
    .bind(req.serialized_size)
    .bind(req.compressed_size)
    .bind(req.padded_size)
    .bind(raw_bytes.len() as i64)
    .bind(req.message_count)
    .bind(req.seq_start)
    .bind(req.seq_end)
    .bind(first_at)
    .bind(last_at)
    .bind(req.encryption_version.unwrap_or(1))
    .bind(req.key_version.unwrap_or(1))
    .bind(req.is_e2ee.unwrap_or(kind == ScopeKind::Dm))
    .bind(req.nonce_b64.trim())
    .bind(req.plaintext_blake3_hex.trim())
    .bind(&actual_ct_b3)
    .bind(storage_backend)
    .bind(&stored_meta.storage_key)
    .bind(req.sealed_meta_b64.as_deref())
    .execute(&mut *tx)
    .await?;

    // Mark superseded chunks as GC-eligible ONLY AFTER new chunk is safely inserted (24h grace!)
    if let Some(ref sup_ids) = req.supersedes_chunk_ids {
        if !sup_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE message_chunks
                SET superseded_at = now(), gc_eligible_at = now() + interval '24 hours', updated_at = now()
                WHERE id = ANY($1) AND scope_type = $2 AND scope_id = $3 AND owner_scope_key = $4
                "#,
            )
            .bind(sup_ids)
            .bind(kind.as_str())
            .bind(&norm_scope_id)
            .bind(&owner_scope_key)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Recompute manifest aggregates cleanly inside transaction
    let agg = sqlx::query(
        r#"
        SELECT
            COUNT(*)::int AS cnt,
            MIN(seq_start) AS oldest_seq,
            MAX(seq_end) AS newest_seq,
            COALESCE(SUM(original_size), 0)::bigint AS tot_orig,
            COALESCE(SUM(compressed_size), 0)::bigint AS tot_comp,
            COALESCE(SUM(encrypted_size), 0)::bigint AS tot_enc
        FROM message_chunks
        WHERE scope_type = $1 AND scope_id = $2 AND owner_scope_key = $3 AND superseded_at IS NULL
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .fetch_one(&mut *tx)
    .await?;

    let next_ver = cur_ver + 1;
    let cnt: i32 = agg.try_get("cnt")?;
    let oldest_seq: Option<i64> = agg.try_get("oldest_seq")?;
    let newest_seq: Option<i64> = agg.try_get("newest_seq")?;
    let tot_orig: i64 = agg.try_get("tot_orig")?;
    let tot_comp: i64 = agg.try_get("tot_comp")?;
    let tot_enc: i64 = agg.try_get("tot_enc")?;

    sqlx::query(
        r#"
        INSERT INTO conversation_chunk_manifests (
            scope_type, scope_id, owner_scope_key, manifest_version, chunk_count,
            oldest_chunk_seq, newest_chunk_seq, total_original_bytes, total_compressed_bytes,
            total_encrypted_bytes, sealed_manifest_b64, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now())
        ON CONFLICT (scope_type, scope_id, owner_scope_key) DO UPDATE SET
            manifest_version = EXCLUDED.manifest_version,
            chunk_count = EXCLUDED.chunk_count,
            oldest_chunk_seq = EXCLUDED.oldest_chunk_seq,
            newest_chunk_seq = EXCLUDED.newest_chunk_seq,
            total_original_bytes = EXCLUDED.total_original_bytes,
            total_compressed_bytes = EXCLUDED.total_compressed_bytes,
            total_encrypted_bytes = EXCLUDED.total_encrypted_bytes,
            sealed_manifest_b64 = COALESCE(EXCLUDED.sealed_manifest_b64, conversation_chunk_manifests.sealed_manifest_b64),
            updated_at = now()
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .bind(next_ver)
    .bind(cnt)
    .bind(oldest_seq)
    .bind(newest_seq)
    .bind(tot_orig)
    .bind(tot_comp)
    .bind(tot_enc)
    .bind(req.sealed_manifest_b64.as_deref())
    .execute(&mut *tx)
    .await?;

    // Bump dictionary reference count if referencing a registered dictionary
    if let Some(did) = req.dictionary_id {
        if did > 0 {
            let _ = sqlx::query(
                "UPDATE compression_dictionaries SET ref_count = ref_count + 1 WHERE dict_id = $1",
            )
            .bind(did)
            .execute(&mut *tx)
            .await;
        }
    }

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "committed": true,
            "chunk_id": chunk_id,
            "manifest_version": next_ver,
            "storage_tier": "warm",
            "encrypted_size": raw_bytes.len(),
            "saved_bytes": req.original_size - (raw_bytes.len() as i64),
        })),
    )
        .into_response())
}

// ── 5. Download Chunk Blob (Warm/Cold + Access Promotion) ────────────────────────────────────────

async fn handle_download_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chunk_id): Path<Uuid>,
) -> AppResult<Response> {
    let user = auth_user(&state, &headers).await?;

    let row = sqlx::query(
        r#"
        SELECT scope_type, scope_id, owner_scope_key, owner_id, chunk_version,
               storage_tier, storage_backend, storage_key, ciphertext_blake3_hex, is_e2ee
        FROM message_chunks
        WHERE id = $1 AND superseded_at IS NULL
        "#,
    )
    .bind(chunk_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("chunk_not_found"))?;

    let scope_type: String = row.try_get("scope_type")?;
    let scope_id: String = row.try_get("scope_id")?;
    let owner_scope_key: String = row.try_get("owner_scope_key")?;
    let owner_id: Uuid = row.try_get("owner_id")?;
    let is_e2ee: bool = row.try_get("is_e2ee")?;

    // Strict RBAC & cross-account isolation check!
    if is_e2ee || scope_type == "dm" {
        if owner_id != user.id || owner_scope_key != user.id.to_string() {
            return Err(AppError::Forbidden("chunk_cross_account_denied"));
        }
    } else {
        authorize_message_scope(&state.db, user.id, &scope_type, &scope_id).await?;
    }

    let storage_backend: String = row.try_get("storage_backend")?;
    let storage_key: String = row.try_get("storage_key")?;
    let tier: String = row.try_get("storage_tier")?;
    let chunk_version: i32 = row.try_get("chunk_version")?;
    let ct_b3: String = row.try_get("ciphertext_blake3_hex")?;

    let bytes = match storage_backend.as_str() {
        "s3_ready" => state
            .storage
            .s3
            .get_object(&storage_key)
            .await
            .map_err(|_| AppError::NotFound("chunk_blob_missing"))?,
        _ => state
            .storage
            .local_cas
            .get_object(&storage_key)
            .await
            .map_err(|_| AppError::NotFound("chunk_blob_missing"))?,
    };

    // Update access timestamp & promote Cold -> Warm on demand!
    let _ = sqlx::query(
        r#"
        UPDATE message_chunks
        SET last_accessed_at = now(),
            storage_tier = CASE WHEN storage_tier = 'cold' THEN 'warm' ELSE storage_tier END,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(chunk_id)
    .execute(&state.db)
    .await;

    let mut resp = (StatusCode::OK, Bytes::from(bytes)).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=31536000, immutable"));
    if let Ok(v) = HeaderValue::from_str(&tier) {
        h.insert("x-chatw-chunk-tier", v);
    }
    if let Ok(v) = HeaderValue::from_str(&chunk_version.to_string()) {
        h.insert("x-chatw-chunk-version", v);
    }
    if let Ok(v) = HeaderValue::from_str(&ct_b3) {
        h.insert("x-chatw-chunk-blake3", v);
    }
    Ok(resp)
}

// ── 6. Mutations & Tombstones Log API ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MutationsQuery {
    scope_type: String,
    scope_id: String,
    since_seq: Option<i64>,
}

async fn handle_list_mutations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<MutationsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let (kind, norm_scope_id, _) =
        authorize_message_scope(&state.db, user.id, &q.scope_type, &q.scope_id).await?;
    let since = q.since_seq.unwrap_or(0);

    let rows = sqlx::query(
        r#"
        SELECT mutation_seq, message_id, chunk_id, op_type, actor_id, payload_enc_b64, payload_plain, created_at
        FROM message_mutations
        WHERE scope_type = $1 AND scope_id = $2 AND mutation_seq > $3
        ORDER BY mutation_seq ASC
        LIMIT 500
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(since)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let ts: DateTime<Utc> = r.try_get("created_at").unwrap_or_else(|_| Utc::now());
            serde_json::json!({
                "mutation_seq": r.try_get::<i64, _>("mutation_seq").unwrap_or(0),
                "message_id": r.try_get::<Uuid, _>("message_id").ok(),
                "chunk_id": r.try_get::<Option<Uuid>, _>("chunk_id").ok().flatten(),
                "op_type": r.try_get::<String, _>("op_type").unwrap_or_default(),
                "actor_id": r.try_get::<Uuid, _>("actor_id").ok(),
                "payload_enc_b64": r.try_get::<Option<String>, _>("payload_enc_b64").ok().flatten(),
                "payload_plain": r.try_get::<Option<serde_json::Value>, _>("payload_plain").ok().flatten(),
                "created_at_ms": ts.timestamp_millis(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "items": items })))
}

#[derive(Deserialize)]
struct AppendMutationReq {
    scope_type: String,
    scope_id: String,
    message_id: Uuid,
    chunk_id: Option<Uuid>,
    op_type: String,
    payload_enc_b64: Option<String>,
    payload_plain: Option<serde_json::Value>,
}

async fn handle_append_mutation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AppendMutationReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let (kind, norm_scope_id, owner_scope_key) =
        authorize_message_scope(&state.db, user.id, &req.scope_type, &req.scope_id).await?;

    if !matches!(
        req.op_type.as_str(),
        "edit" | "delete" | "recall" | "pin" | "unpin" | "reaction_add" | "reaction_remove" | "bookmark" | "unbookmark"
    ) {
        return Err(AppError::BadRequest("invalid_mutation_op"));
    }

    let seq: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO message_mutations
            (scope_type, scope_id, message_id, chunk_id, op_type, actor_id, payload_enc_b64, payload_plain)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING mutation_seq
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(req.message_id)
    .bind(req.chunk_id)
    .bind(&req.op_type)
    .bind(user.id)
    .bind(req.payload_enc_b64.as_deref())
    .bind(req.payload_plain)
    .fetch_one(&state.db)
    .await?;

    let _ = sqlx::query(
        r#"
        UPDATE conversation_chunk_manifests
        SET last_mutation_seq = GREATEST(last_mutation_seq, $4), updated_at = now()
        WHERE scope_type = $1 AND scope_id = $2 AND owner_scope_key = $3
        "#,
    )
    .bind(kind.as_str())
    .bind(&norm_scope_id)
    .bind(&owner_scope_key)
    .bind(seq)
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({ "mutation_seq": seq })))
}

// ── 7. Versioned Zstd Dictionary Distribution (Section 7) ────────────────────────────────────────

async fn handle_list_dictionaries(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let pub_v1 = public_chat_dict_v1();
    let prefix = format!("{}:%", user.id);
    let rows = sqlx::query(
        r#"
        SELECT dict_id, version, is_e2ee, blake3_hex, byte_size, ref_count
        FROM compression_dictionaries
        WHERE owner_scope_key = 'public' OR owner_scope_key LIKE $1
        ORDER BY dict_id ASC, version DESC
        "#,
    )
    .bind(prefix)
    .fetch_all(&state.db)
    .await?;

    let mut list = vec![serde_json::json!({
        "dict_id": pub_v1.dict_id,
        "version": pub_v1.version,
        "is_e2ee": false,
        "blake3_hex": pub_v1.blake3_hex,
        "byte_size": pub_v1.bytes.len(),
        "builtin": true,
    })];
    for r in rows {
        list.push(serde_json::json!({
            "dict_id": r.try_get::<i32, _>("dict_id")?,
            "version": r.try_get::<i32, _>("version")?,
            "is_e2ee": r.try_get::<bool, _>("is_e2ee")?,
            "blake3_hex": r.try_get::<String, _>("blake3_hex")?,
            "byte_size": r.try_get::<i32, _>("byte_size")?,
            "ref_count": r.try_get::<i64, _>("ref_count")?,
            "builtin": false,
        }));
    }
    Ok(Json(serde_json::json!({ "dictionaries": list })))
}

async fn handle_get_dictionary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dict_id): Path<i32>,
) -> AppResult<Response> {
    let user = auth_user(&state, &headers).await?;
    if dict_id == DICT_ID_PUBLIC_CHAT_V1 as i32 {
        let d = public_chat_dict_v1();
        let mut resp = (StatusCode::OK, Bytes::from(d.bytes.clone())).into_response();
        resp.headers_mut()
            .insert("x-chatw-dict-blake3", HeaderValue::from_str(&d.blake3_hex).unwrap());
        return Ok(resp);
    }

    let prefix = format!("{}:%", user.id);
    let row = sqlx::query(
        r#"
        SELECT storage_backend, storage_key, blake3_hex
        FROM compression_dictionaries
        WHERE dict_id = $1 AND (owner_scope_key = 'public' OR owner_scope_key LIKE $2)
        ORDER BY version DESC LIMIT 1
        "#,
    )
    .bind(dict_id)
    .bind(prefix)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("dictionary_not_found"))?;

    let backend: String = row.try_get("storage_backend")?;
    let key: String = row.try_get("storage_key")?;
    let b3: String = row.try_get("blake3_hex")?;
    let bytes = match backend.as_str() {
        "s3_ready" => state
            .storage
            .s3
            .get_object(&key)
            .await
            .map_err(|_| AppError::NotFound("dict_blob_missing"))?,
        _ => state
            .storage
            .local_cas
            .get_object(&key)
            .await
            .map_err(|_| AppError::NotFound("dict_blob_missing"))?,
    };
    let mut resp = (StatusCode::OK, Bytes::from(bytes)).into_response();
    if let Ok(v) = HeaderValue::from_str(&b3) {
        resp.headers_mut().insert("x-chatw-dict-blake3", v);
    }
    Ok(resp)
}

#[derive(Deserialize)]
struct UploadPrivateDictReq {
    dict_id: i32,
    scope_id: String,
    version: i32,
    sealed_dict_b64: String,
}

async fn handle_upload_private_dictionary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UploadPrivateDictReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    if req.dict_id < 1000 {
        return Err(AppError::BadRequest("private_dict_id_must_be_gte_1000"));
    }
    let bytes = B64
        .decode(req.sealed_dict_b64.trim())
        .map_err(|_| AppError::BadRequest("invalid_dict_b64"))?;
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(AppError::BadRequest("invalid_dict_size"));
    }
    // Enforce E2EE private dictionary magic `CWDC1` so plaintext private dicts are NEVER accepted!
    if !bytes.starts_with(b"CWDC1") {
        return Err(AppError::BadRequest("private_dictionary_must_be_e2ee_sealed"));
    }
    let b3 = blake3::hash(&bytes).to_hex().to_string();
    let owner_scope = format!("{}:{}", user.id, req.scope_id.trim());
    let skey = format!("msgdicts/{}/{}_{}.cwdc", &b3[..2], user.id, req.dict_id);

    let (backend, stored) = if state.storage.s3.config().is_ready() {
        (
            "s3_ready",
            state
                .storage
                .s3
                .put_object(&skey, bytes.clone(), "application/octet-stream", None)
                .await
                .map_err(|_| AppError::Internal)?,
        )
    } else {
        (
            "local_cas",
            state
                .storage
                .local_cas
                .put_object(&skey, bytes.clone(), "application/octet-stream", None)
                .await
                .map_err(|_| AppError::Internal)?,
        )
    };

    sqlx::query(
        r#"
        INSERT INTO compression_dictionaries
            (dict_id, owner_scope_key, version, is_e2ee, blake3_hex, byte_size, storage_backend, storage_key)
        VALUES ($1, $2, $3, TRUE, $4, $5, $6, $7)
        ON CONFLICT (dict_id, owner_scope_key, version) DO UPDATE SET
            blake3_hex = EXCLUDED.blake3_hex,
            byte_size = EXCLUDED.byte_size,
            storage_backend = EXCLUDED.storage_backend,
            storage_key = EXCLUDED.storage_key
        "#,
    )
    .bind(req.dict_id)
    .bind(&owner_scope)
    .bind(req.version)
    .bind(&b3)
    .bind(bytes.len() as i32)
    .bind(backend)
    .bind(&stored.storage_key)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "stored": true,
        "dict_id": req.dict_id,
        "version": req.version,
        "blake3_hex": b3,
    })))
}

// ── 8. Encrypted Search Index Multi-Device Sync (Section 14 — Zero-Knowledge!) ───────────────────

async fn handle_list_search_segments(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT scope_key, shard_index, segment_version, format_version, key_version,
               doc_count, min_seq, max_seq, byte_size, blake3_hex, updated_at
        FROM encrypted_search_segments
        WHERE user_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let ts: DateTime<Utc> = r.try_get("updated_at").unwrap_or_else(|_| Utc::now());
            serde_json::json!({
                "scope_key": r.try_get::<String, _>("scope_key").unwrap_or_default(),
                "shard_index": r.try_get::<i32, _>("shard_index").unwrap_or(0),
                "segment_version": r.try_get::<i64, _>("segment_version").unwrap_or(0),
                "format_version": r.try_get::<i16, _>("format_version").unwrap_or(1),
                "key_version": r.try_get::<i32, _>("key_version").unwrap_or(1),
                "doc_count": r.try_get::<i32, _>("doc_count").unwrap_or(0),
                "min_seq": r.try_get::<i64, _>("min_seq").unwrap_or(0),
                "max_seq": r.try_get::<i64, _>("max_seq").unwrap_or(0),
                "byte_size": r.try_get::<i32, _>("byte_size").unwrap_or(0),
                "blake3_hex": r.try_get::<String, _>("blake3_hex").unwrap_or_default(),
                "updated_at_ms": ts.timestamp_millis(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "segments": items })))
}

#[derive(Deserialize)]
struct PutSearchSegmentReq {
    expected_version: i64,
    key_version: Option<i32>,
    doc_count: i32,
    min_seq: i64,
    max_seq: i64,
    blake3_hex: String,
    sealed_shard_b64: String,
}

async fn handle_put_search_segment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((scope_key, shard_index)): Path<(String, i32)>,
    Json(req): Json<PutSearchSegmentReq>,
) -> AppResult<Response> {
    let user = auth_user(&state, &headers).await?;
    let bytes = B64
        .decode(req.sealed_shard_b64.trim())
        .map_err(|_| AppError::BadRequest("invalid_shard_b64"))?;
    // Strictly enforce encrypted `CWSE1` header so plaintext search indices are NEVER accepted!
    if bytes.len() < 32 || !bytes.starts_with(b"CWSE1") {
        return Err(AppError::BadRequest("search_segment_must_be_cwse1_encrypted"));
    }
    let b3 = blake3::hash(&bytes).to_hex().to_string();
    if b3 != req.blake3_hex.trim().to_ascii_lowercase() {
        return Err(AppError::BadRequest("search_segment_blake3_mismatch"));
    }

    let safe_scope: String = scope_key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' { c } else { '_' })
        .take(120)
        .collect();
    let skey = format!(
        "searchidx/{}/{}_{}_{}.cwse",
        &b3[..2],
        user.id,
        blake3::hash(safe_scope.as_bytes()).to_hex(),
        shard_index
    );

    let mut tx = state.db.begin().await?;
    let cur_ver: i64 = sqlx::query_scalar(
        r#"
        SELECT segment_version FROM encrypted_search_segments
        WHERE user_id = $1 AND scope_key = $2 AND shard_index = $3
        FOR UPDATE
        "#,
    )
    .bind(user.id)
    .bind(&safe_scope)
    .bind(shard_index)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(0);

    if cur_ver != req.expected_version {
        tx.rollback().await?;
        return Ok((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "search_segment_version_conflict",
                "current_version": cur_ver,
            })),
        )
            .into_response());
    }

    let (backend, stored) = if state.storage.s3.config().is_ready() {
        (
            "s3_ready",
            state
                .storage
                .s3
                .put_object(&skey, bytes.clone(), "application/octet-stream", None)
                .await
                .map_err(|_| AppError::Internal)?,
        )
    } else {
        (
            "local_cas",
            state
                .storage
                .local_cas
                .put_object(&skey, bytes.clone(), "application/octet-stream", None)
                .await
                .map_err(|_| AppError::Internal)?,
        )
    };

    let next_ver = cur_ver + 1;
    sqlx::query(
        r#"
        INSERT INTO encrypted_search_segments (
            user_id, scope_key, shard_index, segment_version, format_version, key_version,
            doc_count, min_seq, max_seq, byte_size, blake3_hex, storage_backend, storage_key, updated_at
        ) VALUES ($1, $2, $3, $4, 1, $5, $6, $7, $8, $9, $10, $11, $12, now())
        ON CONFLICT (user_id, scope_key, shard_index) DO UPDATE SET
            segment_version = EXCLUDED.segment_version,
            key_version = EXCLUDED.key_version,
            doc_count = EXCLUDED.doc_count,
            min_seq = EXCLUDED.min_seq,
            max_seq = EXCLUDED.max_seq,
            byte_size = EXCLUDED.byte_size,
            blake3_hex = EXCLUDED.blake3_hex,
            storage_backend = EXCLUDED.storage_backend,
            storage_key = EXCLUDED.storage_key,
            updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(&safe_scope)
    .bind(shard_index)
    .bind(next_ver)
    .bind(req.key_version.unwrap_or(1))
    .bind(req.doc_count)
    .bind(req.min_seq)
    .bind(req.max_seq)
    .bind(bytes.len() as i32)
    .bind(&b3)
    .bind(backend)
    .bind(&stored.storage_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "saved": true,
            "segment_version": next_ver,
            "blake3_hex": b3,
            "byte_size": bytes.len(),
        })),
    )
        .into_response())
}

async fn handle_get_search_segment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((scope_key, shard_index)): Path<(String, i32)>,
) -> AppResult<Response> {
    let user = auth_user(&state, &headers).await?;
    let row = sqlx::query(
        r#"
        SELECT storage_backend, storage_key, segment_version, blake3_hex
        FROM encrypted_search_segments
        WHERE user_id = $1 AND scope_key = $2 AND shard_index = $3
        "#,
    )
    .bind(user.id)
    .bind(&scope_key)
    .bind(shard_index)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("search_segment_not_found"))?;

    let backend: String = row.try_get("storage_backend")?;
    let key: String = row.try_get("storage_key")?;
    let ver: i64 = row.try_get("segment_version")?;
    let b3: String = row.try_get("blake3_hex")?;

    let bytes = match backend.as_str() {
        "s3_ready" => state
            .storage
            .s3
            .get_object(&key)
            .await
            .map_err(|_| AppError::NotFound("search_blob_missing"))?,
        _ => state
            .storage
            .local_cas
            .get_object(&key)
            .await
            .map_err(|_| AppError::NotFound("search_blob_missing"))?,
    };

    let mut resp = (StatusCode::OK, Bytes::from(bytes)).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    if let Ok(v) = HeaderValue::from_str(&ver.to_string()) {
        h.insert("x-chatw-segment-version", v);
    }
    if let Ok(v) = HeaderValue::from_str(&b3) {
        h.insert("x-chatw-segment-blake3", v);
    }
    Ok(resp)
}

async fn handle_delete_search_segment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((scope_key, shard_index)): Path<(String, i32)>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let row = sqlx::query(
        r#"
        DELETE FROM encrypted_search_segments
        WHERE user_id = $1 AND scope_key = $2 AND shard_index = $3
        RETURNING storage_backend, storage_key
        "#,
    )
    .bind(user.id)
    .bind(&scope_key)
    .bind(shard_index)
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = row {
        let backend: String = r.try_get("storage_backend")?;
        let key: String = r.try_get("storage_key")?;
        let _ = match backend.as_str() {
            "s3_ready" => state.storage.s3.delete_object(&key).await,
            _ => state.storage.local_cas.delete_object(&key).await,
        };
        Ok(Json(serde_json::json!({ "deleted": true })))
    } else {
        Ok(Json(serde_json::json!({ "deleted": false })))
    }
}

// ── 9. Privacy-Safe Device Storage Telemetry (Section 32) ────────────────────────────────────────

#[derive(Deserialize)]
struct PostTelemetryReq {
    device_id: String,
    platform: Option<String>,
    storage_mode: Option<String>,
    compression_ratio: Option<f64>,
    compression_queue_size: Option<i64>,
    compression_failures: Option<i64>,
    decompression_failures: Option<i64>,
    cache_hit_rate: Option<f64>,
    storage_usage_bytes: Option<i64>,
    storage_budget_bytes: Option<i64>,
    total_saved_bytes: Option<i64>,
    indexed_message_count: Option<i64>,
    last_search_latency_ms: Option<f64>,
    last_sync_latency_ms: Option<f64>,
    chunk_count: Option<i64>,
}

async fn handle_post_telemetry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PostTelemetryReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    sqlx::query(
        r#"
        INSERT INTO device_storage_telemetry (
            user_id, device_id, platform, storage_mode, compression_ratio,
            compression_queue_size, compression_failures, decompression_failures,
            cache_hit_rate, storage_usage_bytes, storage_budget_bytes, total_saved_bytes,
            indexed_message_count, last_search_latency_ms, last_sync_latency_ms, chunk_count, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, now())
        ON CONFLICT (user_id, device_id) DO UPDATE SET
            platform = EXCLUDED.platform,
            storage_mode = EXCLUDED.storage_mode,
            compression_ratio = EXCLUDED.compression_ratio,
            compression_queue_size = EXCLUDED.compression_queue_size,
            compression_failures = EXCLUDED.compression_failures,
            decompression_failures = EXCLUDED.decompression_failures,
            cache_hit_rate = EXCLUDED.cache_hit_rate,
            storage_usage_bytes = EXCLUDED.storage_usage_bytes,
            storage_budget_bytes = EXCLUDED.storage_budget_bytes,
            total_saved_bytes = EXCLUDED.total_saved_bytes,
            indexed_message_count = EXCLUDED.indexed_message_count,
            last_search_latency_ms = EXCLUDED.last_search_latency_ms,
            last_sync_latency_ms = EXCLUDED.last_sync_latency_ms,
            chunk_count = EXCLUDED.chunk_count,
            updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(req.device_id.trim())
    .bind(req.platform.as_deref().unwrap_or("web"))
    .bind(req.storage_mode.as_deref().unwrap_or("auto"))
    .bind(req.compression_ratio.unwrap_or(1.0))
    .bind(req.compression_queue_size.unwrap_or(0))
    .bind(req.compression_failures.unwrap_or(0))
    .bind(req.decompression_failures.unwrap_or(0))
    .bind(req.cache_hit_rate.unwrap_or(0.0).clamp(0.0, 1.0))
    .bind(req.storage_usage_bytes.unwrap_or(0).max(0))
    .bind(req.storage_budget_bytes.unwrap_or(0).max(0))
    .bind(req.total_saved_bytes.unwrap_or(0).max(0))
    .bind(req.indexed_message_count.unwrap_or(0).max(0))
    .bind(req.last_search_latency_ms.unwrap_or(0.0).max(0.0))
    .bind(req.last_sync_latency_ms.unwrap_or(0.0).max(0.0))
    .bind(req.chunk_count.unwrap_or(0).max(0))
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "recorded": true })))
}

async fn handle_get_telemetry(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_user(&state, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT device_id, platform, storage_mode, compression_ratio, compression_queue_size,
               compression_failures, decompression_failures, cache_hit_rate, storage_usage_bytes,
               storage_budget_bytes, total_saved_bytes, indexed_message_count,
               last_search_latency_ms, last_sync_latency_ms, chunk_count, updated_at
        FROM device_storage_telemetry
        WHERE user_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let chunk_summary = sqlx::query(
        r#"
        SELECT
            COUNT(*)::bigint AS total_chunks,
            COUNT(*) FILTER (WHERE storage_tier = 'warm')::bigint AS warm_chunks,
            COUNT(*) FILTER (WHERE storage_tier = 'cold')::bigint AS cold_chunks,
            COALESCE(SUM(original_size), 0)::bigint AS total_orig_bytes,
            COALESCE(SUM(encrypted_size), 0)::bigint AS total_stored_bytes
        FROM message_chunks
        WHERE owner_id = $1 AND superseded_at IS NULL
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let devices: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "device_id": r.try_get::<String, _>("device_id").unwrap_or_default(),
                "platform": r.try_get::<String, _>("platform").unwrap_or_default(),
                "storage_mode": r.try_get::<String, _>("storage_mode").unwrap_or_default(),
                "compression_ratio": r.try_get::<f64, _>("compression_ratio").unwrap_or(1.0),
                "compression_queue_size": r.try_get::<i64, _>("compression_queue_size").unwrap_or(0),
                "cache_hit_rate": r.try_get::<f64, _>("cache_hit_rate").unwrap_or(0.0),
                "storage_usage_bytes": r.try_get::<i64, _>("storage_usage_bytes").unwrap_or(0),
                "storage_budget_bytes": r.try_get::<i64, _>("storage_budget_bytes").unwrap_or(0),
                "total_saved_bytes": r.try_get::<i64, _>("total_saved_bytes").unwrap_or(0),
                "indexed_message_count": r.try_get::<i64, _>("indexed_message_count").unwrap_or(0),
                "last_search_latency_ms": r.try_get::<f64, _>("last_search_latency_ms").unwrap_or(0.0),
                "chunk_count": r.try_get::<i64, _>("chunk_count").unwrap_or(0),
            })
        })
        .collect();

    let orig_b: i64 = chunk_summary.try_get("total_orig_bytes")?;
    let stored_b: i64 = chunk_summary.try_get("total_stored_bytes")?;

    Ok(Json(serde_json::json!({
        "devices": devices,
        "server_chunks": {
            "total_chunks": chunk_summary.try_get::<i64, _>("total_chunks")?,
            "warm_chunks": chunk_summary.try_get::<i64, _>("warm_chunks")?,
            "cold_chunks": chunk_summary.try_get::<i64, _>("cold_chunks")?,
            "total_original_bytes": orig_b,
            "total_stored_bytes": stored_b,
            "saved_bytes": (orig_b - stored_b).max(0),
        }
    })))
}

// ── 10. Server Background Incremental Compaction (Plaintext Groups/Channels >= 7d) + Cold Tiering ─

#[derive(Clone, Debug, Default, Serialize)]
pub struct BackgroundTierReport {
    pub compacted_chunks: u64,
    pub compacted_messages: u64,
    pub bytes_saved: u64,
    pub moved_to_cold: u64,
    pub gc_deleted_chunks: u64,
}

/// Lease-protected bounded incremental background tick (Sections 3, 8, 25.1):
/// 1. Acquires a short server-wide distributed lease so multiple backend replicas NEVER collide.
/// 2. Compacts at most `max_chunks_per_tick` (default 2) plaintext >=7-day group/channel slices
///    using mandatory Zstandard Level 22 (`chatw-msg-store`), NEVER touching E2EE DMs!
/// 3. Moves Warm chunks untouched for >= 30 days to Cold tier (`s3_ready` with `STANDARD_IA` when S3
///    is configured, or `cold` tier metadata).
/// 4. Safe GC of superseded chunks whose 24h grace period has elapsed.
pub async fn run_incremental_server_tiering_tick(
    pool: &PgPool,
    storage: &StorageRegistry,
    max_chunks_per_tick: usize,
) -> Result<BackgroundTierReport, sqlx::Error> {
    let mut report = BackgroundTierReport::default();
    let worker_id = format!("server-worker-{}", std::process::id());

    // Distributed server-replica lease (300s TTL)
    let acquired: Option<i64> = sqlx::query_scalar(
        r#"
        INSERT INTO compression_leases (lease_key, holder_device_id, fencing_token, status, leased_until, updated_at)
        VALUES ('server:tiering_compaction', $1, 1, 'active', now() + interval '300 seconds', now())
        ON CONFLICT (lease_key) DO UPDATE SET
            holder_device_id = EXCLUDED.holder_device_id,
            fencing_token = compression_leases.fencing_token + 1,
            status = 'active',
            leased_until = EXCLUDED.leased_until,
            updated_at = now()
        WHERE compression_leases.leased_until < now() OR compression_leases.status = 'released'
        RETURNING fencing_token
        "#,
    )
    .bind(&worker_id)
    .fetch_optional(pool)
    .await?;

    let Some(token) = acquired else {
        return Ok(report);
    };

    // Phase A: Bounded incremental Zstd-22 archival of plaintext >= 7d `group_messages` (<= max_chunks_per_tick)
    let dict_reg = DictionaryRegistry::new();
    let candidate_groups = sqlx::query(
        r#"
        SELECT gm.group_id, g.owner_id, COUNT(*)::bigint AS eligible_cnt
        FROM group_messages gm
        JOIN groups g ON g.id = gm.group_id
        LEFT JOIN conversation_chunk_manifests m
          ON m.scope_type = 'group' AND m.scope_id = gm.group_id::text AND m.owner_scope_key = 'shared'
        WHERE gm.created_at <= now() - interval '7 days'
          AND gm.seq > COALESCE(m.newest_chunk_seq, 0)
        GROUP BY gm.group_id, g.owner_id
        HAVING COUNT(*) >= 16
        ORDER BY eligible_cnt DESC
        LIMIT $1
        "#,
    )
    .bind(max_chunks_per_tick.clamp(1, 4) as i64)
    .fetch_all(pool)
    .await?;

    for grp in candidate_groups {
        let gid: Uuid = grp.try_get("group_id")?;
        let owner_id: Uuid = grp.try_get("owner_id")?;
        let sid = gid.to_string();

        let newest_sealed: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq_end), 0) FROM message_chunks WHERE scope_type = 'group' AND scope_id = $1 AND owner_scope_key = 'shared' AND superseded_at IS NULL",
        )
        .bind(&sid)
        .fetch_one(pool)
        .await?;

        let rows = sqlx::query(
            r#"
            SELECT id, seq, sender_id, content, reply_to_id, is_pinned, is_edited, created_at
            FROM group_messages
            WHERE group_id = $1 AND seq > $2 AND created_at <= now() - interval '7 days'
            ORDER BY seq ASC
            LIMIT 1000
            "#,
        )
        .bind(gid)
        .bind(newest_sealed)
        .fetch_all(pool)
        .await?;

        if rows.len() < 16 {
            continue;
        }

        let compact_msgs: Vec<CompactMessage> = rows
            .iter()
            .map(|r| {
                let ts: DateTime<Utc> = r.try_get("created_at").unwrap_or_else(|_| Utc::now());
                CompactMessage {
                    id: r.try_get::<Uuid, _>("id").map(|u| u.to_string()).unwrap_or_default(),
                    seq: r.try_get::<i64, _>("seq").unwrap_or(0),
                    sender_id: r.try_get::<Uuid, _>("sender_id").map(|u| u.to_string()).unwrap_or_default(),
                    created_at_ms: ts.timestamp_millis(),
                    msg_type: MessageType::Text,
                    text: r.try_get::<String, _>("content").unwrap_or_default(),
                    reply_to_id: r.try_get::<Option<Uuid>, _>("reply_to_id").ok().flatten().map(|u| u.to_string()),
                    is_edited: r.try_get::<bool, _>("is_edited").unwrap_or(false),
                    deleted_for_all: false,
                    is_pinned: r.try_get::<bool, _>("is_pinned").unwrap_or(false),
                    is_bookmarked: false,
                    status_code: 0,
                    reactions: Vec::new(),
                    attachments: Vec::new(),
                    extra_meta: None,
                }
            })
            .collect();

        let raw_json = match serde_json::to_vec(&compact_msgs) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let serialized = match serialize_chunk_messages(&compact_msgs) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let pt_b3 = blake3::hash(&serialized).to_hex().to_string();

        // Compress with mandatory Zstandard Level 22 on blocking thread pool so Tokio runtime never stalls!
        let ser_clone = serialized.clone();
        let dict_clone = dict_reg.clone();
        let comp_res = tokio::task::spawn_blocking(move || {
            compress_zstd22_best_dict(
                &ser_clone,
                DICT_ID_PUBLIC_CHAT_V1,
                &dict_clone,
                Some(Zstd22Options::default()),
            )
        })
        .await;

        let Ok(Ok((compressed, chosen_dict))) = comp_res else {
            continue;
        };
        if compressed.len() >= raw_json.len() {
            continue;
        }

        // Verify decompression round-trip BEFORE storing!
        let dict_opt = if chosen_dict == DICT_ID_NONE {
            None
        } else {
            dict_reg.get(chosen_dict).ok().flatten().map(|d| d.bytes.as_slice())
        };
        let verified = chatw_msg_store::decompress_zstd_bounded(&compressed, dict_opt, None)
            .ok()
            .is_some_and(|d| d == serialized);
        if !verified {
            continue;
        }

        let ct_b3 = blake3::hash(&compressed).to_hex().to_string();
        let chunk_id = Uuid::new_v4();
        let skey = format!("msgchunks/plain/{}/{}.zst22", &ct_b3[..2], chunk_id);
        let stored_ok = storage
            .local_cas
            .put_object(&skey, compressed.clone(), "application/zstd", None)
            .await;
        let Ok(stored_meta) = stored_ok else {
            continue;
        };

        let first_at = DateTime::from_timestamp_millis(compact_msgs[0].created_at_ms).unwrap_or_else(Utc::now);
        let last_at = DateTime::from_timestamp_millis(compact_msgs.last().unwrap().created_at_ms).unwrap_or_else(Utc::now);

        let mut tx = pool.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO message_chunks (
                id, scope_type, scope_id, owner_scope_key, owner_id, chunk_version, format_version,
                access_epoch, compression_algo, compression_level, dictionary_id,
                original_size, serialized_size, compressed_size, padded_size, encrypted_size,
                message_count, seq_start, seq_end, first_created_at, last_created_at,
                encryption_version, key_version, is_e2ee, nonce_b64, plaintext_blake3_hex,
                ciphertext_blake3_hex, storage_tier, storage_backend, storage_key, sync_state
            ) VALUES (
                $1, 'group', $2, 'shared', $3, 1, 1,
                1, 'zstd', 22, $4,
                $5, $6, $7, $7, $7,
                $8, $9, $10, $11, $12,
                0, 0, FALSE, '', $13,
                $14, 'warm', 'local_cas', $15, 'committed'
            )
            "#,
        )
        .bind(chunk_id)
        .bind(&sid)
        .bind(owner_id)
        .bind(chosen_dict as i32)
        .bind(raw_json.len() as i64)
        .bind(serialized.len() as i64)
        .bind(compressed.len() as i64)
        .bind(compact_msgs.len() as i32)
        .bind(compact_msgs[0].seq)
        .bind(compact_msgs.last().unwrap().seq)
        .bind(first_at)
        .bind(last_at)
        .bind(&pt_b3)
        .bind(&ct_b3)
        .bind(&stored_meta.storage_key)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO conversation_chunk_manifests (
                scope_type, scope_id, owner_scope_key, manifest_version, chunk_count,
                oldest_chunk_seq, newest_chunk_seq, total_original_bytes, total_compressed_bytes,
                total_encrypted_bytes, updated_at
            ) VALUES ('group', $1, 'shared', 1, 1, $2, $3, $4, $5, $5, now())
            ON CONFLICT (scope_type, scope_id, owner_scope_key) DO UPDATE SET
                manifest_version = conversation_chunk_manifests.manifest_version + 1,
                chunk_count = conversation_chunk_manifests.chunk_count + 1,
                oldest_chunk_seq = LEAST(COALESCE(conversation_chunk_manifests.oldest_chunk_seq, EXCLUDED.oldest_chunk_seq), EXCLUDED.oldest_chunk_seq),
                newest_chunk_seq = GREATEST(COALESCE(conversation_chunk_manifests.newest_chunk_seq, EXCLUDED.newest_chunk_seq), EXCLUDED.newest_chunk_seq),
                total_original_bytes = conversation_chunk_manifests.total_original_bytes + EXCLUDED.total_original_bytes,
                total_compressed_bytes = conversation_chunk_manifests.total_compressed_bytes + EXCLUDED.total_compressed_bytes,
                total_encrypted_bytes = conversation_chunk_manifests.total_encrypted_bytes + EXCLUDED.total_encrypted_bytes,
                updated_at = now()
            "#,
        )
        .bind(&sid)
        .bind(compact_msgs[0].seq)
        .bind(compact_msgs.last().unwrap().seq)
        .bind(raw_json.len() as i64)
        .bind(compressed.len() as i64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        report.compacted_chunks += 1;
        report.compacted_messages += compact_msgs.len() as u64;
        report.bytes_saved += (raw_json.len() - compressed.len()) as u64;
    }

    // Phase B: Warm -> Cold tier transition for chunks untouched >= 30 days (bounded 20/tick)
    let cold_candidates = sqlx::query(
        r#"
        SELECT id, storage_backend, storage_key
        FROM message_chunks
        WHERE storage_tier = 'warm'
          AND superseded_at IS NULL
          AND last_accessed_at <= now() - interval '30 days'
        ORDER BY last_accessed_at ASC
        LIMIT 20
        "#,
    )
    .fetch_all(pool)
    .await?;

    for c in cold_candidates {
        let cid: Uuid = c.try_get("id")?;
        let backend: String = c.try_get("storage_backend")?;
        let key: String = c.try_get("storage_key")?;
        if backend == "local_cas" && storage.s3.config().is_ready() {
            if let Ok(bytes) = storage.local_cas.get_object(&key).await {
                let cold_key = format!("cold/{key}");
                if storage
                    .s3
                    .put_object_with_storage_class(&cold_key, bytes, "application/octet-stream", "STANDARD_IA")
                    .await
                    .is_ok()
                {
                    let _ = sqlx::query(
                        "UPDATE message_chunks SET storage_tier = 'cold', storage_backend = 's3_ready', storage_key = $2, updated_at = now() WHERE id = $1",
                    )
                    .bind(cid)
                    .bind(&cold_key)
                    .execute(pool)
                    .await;
                    let _ = storage.local_cas.delete_object(&key).await;
                    report.moved_to_cold += 1;
                    continue;
                }
            }
        }
        let _ = sqlx::query(
            "UPDATE message_chunks SET storage_tier = 'cold', updated_at = now() WHERE id = $1",
        )
        .bind(cid)
        .execute(pool)
        .await;
        report.moved_to_cold += 1;
    }

    // Phase C: Safe GC of superseded chunks past their 24h grace window (bounded 20/tick)
    let gc_rows = sqlx::query(
        r#"
        SELECT id, storage_backend, storage_key, dictionary_id
        FROM message_chunks
        WHERE gc_eligible_at IS NOT NULL AND gc_eligible_at <= now()
        LIMIT 20
        "#,
    )
    .fetch_all(pool)
    .await?;

    for g in gc_rows {
        let cid: Uuid = g.try_get("id")?;
        let backend: String = g.try_get("storage_backend")?;
        let key: String = g.try_get("storage_key")?;
        let did: i32 = g.try_get("dictionary_id")?;
        let _ = match backend.as_str() {
            "s3_ready" => storage.s3.delete_object(&key).await,
            _ => storage.local_cas.delete_object(&key).await,
        };
        if did > 0 {
            let _ = sqlx::query(
                "UPDATE compression_dictionaries SET ref_count = GREATEST(ref_count - 1, 0) WHERE dict_id = $1",
            )
            .bind(did)
            .execute(pool)
            .await;
        }
        let _ = sqlx::query("DELETE FROM message_chunks WHERE id = $1")
            .bind(cid)
            .execute(pool)
            .await;
        report.gc_deleted_chunks += 1;
    }

    // Release server lease cleanly
    let _ = sqlx::query(
        "UPDATE compression_leases SET status = 'released', leased_until = now(), updated_at = now() WHERE lease_key = 'server:tiering_compaction' AND fencing_token = $1",
    )
    .bind(token)
    .execute(pool)
    .await;

    Ok(report)
}
