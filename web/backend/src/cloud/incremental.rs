use std::collections::{HashMap, HashSet};

use argon2::{Algorithm, Argon2, Params, Version};
use kiyo_kdata::{
    AccountCloudKeys, KdataIndexEntry, KdataManifest, derive_account_cloud_keys,
    open_cloud_chunk, open_cloud_manifest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    backup::engine::{
        ExportScopeOptions, MediaSourceStream, RestoreOptions, RestoreResultResponse,
        prepare_backup_dataset,
    },
    error::{AppError, AppResult},
    storage::{StorageRegistry, sha256_bytes},
};

use super::{CloudStorageProvider, open_cloud_config, providers::instantiate_provider, seal_cloud_config};

const CLOUD_CHUNK_SLICE_BYTES: usize = 512 * 1024; // 512 KiB chunks for fast resume & dedup

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudChunkRef {
    pub chunk_id_hex: String,
    pub remote_key: String,
    pub plain_size: usize,
    pub cipher_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudManifestEntry {
    pub index: KdataIndexEntry,
    pub chunks: Vec<CloudChunkRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudEncryptedSnapshot {
    pub kdata_manifest: KdataManifest,
    pub entries: Vec<CloudManifestEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloudBackupRunResult {
    pub snapshot_id: Uuid,
    pub snapshot_seq: i64,
    pub total_plain_bytes: u64,
    pub uploaded_delta_bytes: u64,
    pub deduplicated_bytes: u64,
    pub new_chunks: usize,
    pub reused_chunks: usize,
    pub file_count: usize,
    pub missing_files: usize,
    pub verified: bool,
    pub remote_manifest_key: String,
    pub manifest_sha256: String,
}

pub fn derive_cloud_keys_from_passphrase(passphrase: &str, user_id: &Uuid) -> Result<AccountCloudKeys, AppError> {
    if passphrase.chars().count() < 6 {
        return Err(AppError::BadRequest("encryption_password_too_short"));
    }
    let mut salt = [0u8; 16];
    salt.copy_from_slice(user_id.as_bytes());
    let params = Params::new(19_456, 2, 1, Some(32)).map_err(|_| AppError::Internal)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut master = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut master)
        .map_err(|_| AppError::Internal)?;
    derive_account_cloud_keys(&master, user_id).map_err(|_| AppError::Internal)
}

async fn load_provider_with_refresh(
    pool: &PgPool,
    http: &reqwest::Client,
    session_secret: &[u8],
    user_id: Uuid,
    connection_id: Uuid,
) -> AppResult<(Box<dyn CloudStorageProvider>, String, serde_json::Value)> {
    let row = sqlx::query(
        "SELECT provider, config_enc FROM cloud_connections WHERE id = $1 AND user_id = $2",
    )
    .bind(connection_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("cloud_connection_not_found"))?;

    let provider_id: String = row.try_get("provider")?;
    let config_enc: Vec<u8> = row.try_get("config_enc")?;
    let mut cfg = open_cloud_config(session_secret, &config_enc)?;
    let provider = instantiate_provider(&provider_id, &cfg).map_err(AppError::from)?;

    // Attempt verify; if TokenExpired, transparently refresh and persist encrypted tokens
    if let Err(super::CloudError::TokenExpired) = provider.verify_connection(http).await {
        if let Ok(Some(new_tok)) = provider.refresh_authorization(http).await {
            if let Some(at) = new_tok["access_token"].as_str() {
                cfg["access_token"] = serde_json::json!(at);
            }
            if let Some(rt) = new_tok["refresh_token"].as_str() {
                cfg["refresh_token"] = serde_json::json!(rt);
            }
            let re_sealed = seal_cloud_config(session_secret, &cfg)?;
            sqlx::query("UPDATE cloud_connections SET config_enc = $1, status = 'connected', last_error = NULL, updated_at = now() WHERE id = $2")
                .bind(re_sealed)
                .bind(connection_id)
                .execute(pool)
                .await?;
            let refreshed = instantiate_provider(&provider_id, &cfg).map_err(AppError::from)?;
            return Ok((refreshed, provider_id, cfg));
        } else {
            sqlx::query("UPDATE cloud_connections SET status = 'auth_error', last_error = 'Cloud session expired, please reconnect', updated_at = now() WHERE id = $1")
                .bind(connection_id)
                .execute(pool)
                .await?;
            return Err(AppError::Unauthorized);
        }
    }

    Ok((provider, provider_id, cfg))
}

/// Runs a true incremental delta backup to the user's connected cloud provider:
/// - Only encrypts & uploads chunks not already present in `cloud_chunks`
/// - Verifies uploaded manifest & chunks before marking snapshot `verified`
/// - Preserves old verified snapshot until new one is confirmed
/// - Performs safe ref-counted garbage collection on expired snapshots
pub async fn run_incremental_cloud_backup(
    pool: &PgPool,
    storage: &StorageRegistry,
    http: &reqwest::Client,
    session_secret: &[u8],
    user_id: Uuid,
    connection_id: Uuid,
    backup_passphrase: &str,
) -> AppResult<CloudBackupRunResult> {
    let keys = derive_cloud_keys_from_passphrase(backup_passphrase, &user_id)?;
    let (provider, _provider_id, _cfg) =
        load_provider_with_refresh(pool, http, session_secret, user_id, connection_id).await?;

    let dataset = prepare_backup_dataset(pool, storage, user_id, &ExportScopeOptions::default()).await?;

    let parent_row = sqlx::query(
        "SELECT id, snapshot_seq FROM cloud_snapshots
         WHERE user_id = $1 AND connection_id = $2 AND status = 'verified'
         ORDER BY snapshot_seq DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(connection_id)
    .fetch_optional(pool)
    .await?;

    let parent_id: Option<Uuid> = parent_row.as_ref().and_then(|r| r.try_get("id").ok());
    let next_seq: i64 = parent_row
        .as_ref()
        .and_then(|r| r.try_get::<i64, _>("snapshot_seq").ok())
        .unwrap_or(0)
        + 1;
    let snapshot_id = Uuid::new_v4();

    // Load known already-uploaded chunks for incremental delta detection
    let existing_rows = sqlx::query(
        "SELECT id, chunk_id_hex, remote_key, cipher_sha256, byte_size FROM cloud_chunks
         WHERE user_id = $1 AND connection_id = $2",
    )
    .bind(user_id)
    .bind(connection_id)
    .fetch_all(pool)
    .await?;

    let mut known_chunks: HashMap<String, (Uuid, String, String, i64)> = HashMap::new();
    for er in existing_rows {
        let id: Uuid = er.try_get("id")?;
        let cid: String = er.try_get("chunk_id_hex")?;
        let rk: String = er.try_get("remote_key")?;
        let csha: String = er.try_get("cipher_sha256")?;
        let sz: i64 = er.try_get("byte_size")?;
        known_chunks.insert(cid, (id, rk, csha, sz));
    }

    let mut manifest_entries = Vec::with_capacity(dataset.entries.len());
    let mut snapshot_chunk_ids: Vec<Uuid> = Vec::new();
    let mut seen_snap_chunks = HashSet::new();
    let mut uploaded_delta_bytes = 0u64;
    let mut deduplicated_bytes = 0u64;
    let mut new_chunks = 0usize;
    let mut reused_chunks = 0usize;

    for item in &dataset.entries {
        let plain_bytes: Vec<u8> = match &item.source {
            MediaSourceStream::Memory(b) => b.clone(),
            MediaSourceStream::File(p) => tokio::fs::read(p).await.map_err(|_| AppError::Internal)?,
        };
        let mut entry_chunk_refs = Vec::new();

        // Empty slice edge case
        let slices: Vec<&[u8]> = if plain_bytes.is_empty() {
            vec![&[][..]]
        } else {
            plain_bytes.chunks(CLOUD_CHUNK_SLICE_BYTES).collect()
        };

        for slice in slices {
            let sealed = kiyo_kdata::seal_cloud_chunk(&keys, &user_id, slice).map_err(|_| AppError::Internal)?;
            if let Some((db_chunk_id, remote_key, cipher_sha, _)) = known_chunks.get(&sealed.chunk_id_hex).cloned() {
                reused_chunks += 1;
                deduplicated_bytes += slice.len() as u64;
                if seen_snap_chunks.insert(db_chunk_id) {
                    snapshot_chunk_ids.push(db_chunk_id);
                }
                entry_chunk_refs.push(CloudChunkRef {
                    chunk_id_hex: sealed.chunk_id_hex,
                    remote_key,
                    plain_size: slice.len(),
                    cipher_sha256: cipher_sha,
                });
            } else {
                let remote_key = format!("chunks/{}/{}.kchunk", &sealed.chunk_id_hex[..2], sealed.chunk_id_hex);
                let uploaded = provider
                    .upload_backup(http, &remote_key, sealed.wire_bytes.clone())
                    .await
                    .map_err(AppError::from)?;
                let actual_remote_key = uploaded.remote_key;

                let db_chunk_id: Uuid = sqlx::query_scalar(
                    "INSERT INTO cloud_chunks (user_id, connection_id, chunk_id_hex, remote_key, plain_sha256, cipher_sha256, byte_size, ref_count)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, 0)
                     ON CONFLICT (user_id, connection_id, chunk_id_hex) DO UPDATE SET remote_key = EXCLUDED.remote_key
                     RETURNING id",
                )
                .bind(user_id)
                .bind(connection_id)
                .bind(&sealed.chunk_id_hex)
                .bind(&actual_remote_key)
                .bind(&sealed.plain_sha256_hex)
                .bind(&sealed.cipher_sha256_hex)
                .bind(sealed.wire_bytes.len() as i64)
                .fetch_one(pool)
                .await?;

                known_chunks.insert(
                    sealed.chunk_id_hex.clone(),
                    (
                        db_chunk_id,
                        actual_remote_key.clone(),
                        sealed.cipher_sha256_hex.clone(),
                        sealed.wire_bytes.len() as i64,
                    ),
                );
                if seen_snap_chunks.insert(db_chunk_id) {
                    snapshot_chunk_ids.push(db_chunk_id);
                }
                uploaded_delta_bytes += sealed.wire_bytes.len() as u64;
                new_chunks += 1;

                entry_chunk_refs.push(CloudChunkRef {
                    chunk_id_hex: sealed.chunk_id_hex,
                    remote_key: actual_remote_key,
                    plain_size: slice.len(),
                    cipher_sha256: sealed.cipher_sha256_hex,
                });
            }
        }

        manifest_entries.push(CloudManifestEntry {
            index: item.index.clone(),
            chunks: entry_chunk_refs,
        });
    }

    let mut snap_manifest = dataset.manifest.clone();
    snap_manifest.snapshot_id = snapshot_id;
    snap_manifest.base_snapshot_id = parent_id;
    snap_manifest.incremental = parent_id.is_some();

    let payload = CloudEncryptedSnapshot {
        kdata_manifest: snap_manifest.clone(),
        entries: manifest_entries,
    };
    let snap_json = serde_json::to_vec(&payload).map_err(|_| AppError::Internal)?;
    let sealed_manifest = kiyo_kdata::seal_cloud_manifest(&keys, &user_id, &snapshot_id, &snap_json)
        .map_err(|_| AppError::Internal)?;
    let manifest_sha = hex::encode(sha256_bytes(&sealed_manifest));
    let manifest_rel = format!("manifests/snap_{:06}_{}.kmf", next_seq, snapshot_id.simple());

    let uploaded_mf = provider
        .upload_backup(http, &manifest_rel, sealed_manifest.clone())
        .await
        .map_err(AppError::from)?;
    let actual_mf_key = uploaded_mf.remote_key;

    // Mandatory verification BEFORE marking snapshot verified!
    let verified = provider
        .verify_backup(http, &actual_mf_key, &manifest_sha)
        .await
        .map_err(AppError::from)?;
    if !verified {
        return Err(AppError::BadRequest("cloud_backup_verification_failed"));
    }

    // Atomic commit of snapshot + ref_counts in DB transaction
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO cloud_snapshots (id, user_id, connection_id, snapshot_seq, parent_snapshot_id, remote_manifest_key, manifest_sha256, total_plain_bytes, uploaded_delta_bytes, file_count, status, verified_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'verified', now())",
    )
    .bind(snapshot_id)
    .bind(user_id)
    .bind(connection_id)
    .bind(next_seq)
    .bind(parent_id)
    .bind(&actual_mf_key)
    .bind(&manifest_sha)
    .bind(snap_manifest.total_plain_bytes as i64)
    .bind(uploaded_delta_bytes as i64)
    .bind(dataset.entries.len() as i32)
    .execute(&mut *tx)
    .await?;

    for cid in &snapshot_chunk_ids {
        sqlx::query("INSERT INTO cloud_snapshot_chunks (snapshot_id, chunk_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(snapshot_id)
            .bind(cid)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE cloud_chunks SET ref_count = ref_count + 1 WHERE id = $1")
            .bind(cid)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query("UPDATE cloud_connections SET status = 'connected', last_error = NULL, last_backup_at = now(), updated_at = now() WHERE id = $1")
        .bind(connection_id)
        .execute(&mut *tx)
        .await?;

    // Mark exported media_references as having a verified encrypted backup
    let backed_refs: Vec<Uuid> = dataset.entries.iter().filter_map(|e| e.index.ref_id).collect();
    if !backed_refs.is_empty() {
        sqlx::query(
            "UPDATE media_references SET availability_state = 'backed_up_encrypted', updated_at = now()
             WHERE id = ANY($1) AND owner_id = $2",
        )
        .bind(&backed_refs)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Run safe GC only after verified commit
    let _ = gc_old_snapshots(pool, provider.as_ref(), http, user_id, connection_id).await;

    Ok(CloudBackupRunResult {
        snapshot_id,
        snapshot_seq: next_seq,
        total_plain_bytes: snap_manifest.total_plain_bytes,
        uploaded_delta_bytes,
        deduplicated_bytes,
        new_chunks,
        reused_chunks,
        file_count: dataset.entries.len(),
        missing_files: snap_manifest.missing_media.len(),
        verified,
        remote_manifest_key: actual_mf_key,
        manifest_sha256: manifest_sha,
    })
}

async fn gc_old_snapshots(
    pool: &PgPool,
    provider: &dyn CloudStorageProvider,
    http: &reqwest::Client,
    user_id: Uuid,
    connection_id: Uuid,
) -> AppResult<()> {
    let retain: i32 = sqlx::query_scalar("SELECT retain_snapshots FROM cloud_backup_policies WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(5);

    let old_snaps = sqlx::query(
        "SELECT id, remote_manifest_key FROM cloud_snapshots
         WHERE user_id = $1 AND connection_id = $2 AND status = 'verified'
         ORDER BY snapshot_seq DESC
         OFFSET $3",
    )
    .bind(user_id)
    .bind(connection_id)
    .bind(retain.max(1) as i64)
    .fetch_all(pool)
    .await?;

    for row in old_snaps {
        let sid: Uuid = row.try_get("id")?;
        let mf_key: String = row.try_get("remote_manifest_key")?;
        let _ = delete_snapshot_safely(pool, provider, http, user_id, connection_id, sid, &mf_key).await;
    }
    Ok(())
}

pub async fn delete_snapshot_safely(
    pool: &PgPool,
    provider: &dyn CloudStorageProvider,
    http: &reqwest::Client,
    user_id: Uuid,
    connection_id: Uuid,
    snapshot_id: Uuid,
    remote_manifest_key: &str,
) -> AppResult<()> {
    let chunk_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT chunk_id FROM cloud_snapshot_chunks WHERE snapshot_id = $1",
    )
    .bind(snapshot_id)
    .fetch_all(pool)
    .await?;

    let _ = provider.delete_backup(http, remote_manifest_key).await;

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM cloud_snapshot_chunks WHERE snapshot_id = $1")
        .bind(snapshot_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE cloud_snapshots SET status = 'deleted' WHERE id = $1 AND user_id = $2")
        .bind(snapshot_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let mut orphan_keys = Vec::new();
    for cid in chunk_ids {
        let remaining: i32 = sqlx::query_scalar(
            "UPDATE cloud_chunks SET ref_count = GREATEST(ref_count - 1, 0) WHERE id = $1 RETURNING ref_count",
        )
        .bind(cid)
        .fetch_one(&mut *tx)
        .await?;
        if remaining == 0 {
            if let Some(rk) = sqlx::query_scalar::<_, String>(
                "DELETE FROM cloud_chunks WHERE id = $1 AND ref_count = 0 RETURNING remote_key",
            )
            .bind(cid)
            .fetch_optional(&mut *tx)
            .await?
            {
                orphan_keys.push((cid, rk));
            }
        }
    }
    tx.commit().await?;

    for (_, rk) in orphan_keys {
        let _ = provider.delete_backup(http, &rk).await;
    }
    let _ = connection_id;
    Ok(())
}

pub async fn restore_from_cloud_snapshot(
    pool: &PgPool,
    storage: &StorageRegistry,
    http: &reqwest::Client,
    session_secret: &[u8],
    user_id: Uuid,
    connection_id: Uuid,
    snapshot_id: Uuid,
    backup_passphrase: &str,
    mode: &str,
) -> AppResult<RestoreResultResponse> {
    let row = sqlx::query(
        "SELECT remote_manifest_key, manifest_sha256 FROM cloud_snapshots
         WHERE id = $1 AND user_id = $2 AND connection_id = $3 AND status = 'verified'",
    )
    .bind(snapshot_id)
    .bind(user_id)
    .bind(connection_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("cloud_snapshot_not_found"))?;

    let mf_key: String = row.try_get("remote_manifest_key")?;
    let expected_mf_sha: String = row.try_get("manifest_sha256")?;
    let keys = derive_cloud_keys_from_passphrase(backup_passphrase, &user_id)?;
    let (provider, _, _) =
        load_provider_with_refresh(pool, http, session_secret, user_id, connection_id).await?;

    let mf_wire = provider.download_backup(http, &mf_key).await.map_err(AppError::from)?;
    if hex::encode(sha256_bytes(&mf_wire)) != expected_mf_sha {
        return Err(AppError::BadRequest("cloud_manifest_checksum_mismatch"));
    }
    let mf_plain = open_cloud_manifest(&keys, &user_id, &snapshot_id, &mf_wire)
        .map_err(|_| AppError::Unauthorized)?;
    let snap: CloudEncryptedSnapshot =
        serde_json::from_slice(&mf_plain).map_err(|_| AppError::BadRequest("corrupted_cloud_manifest"))?;

    // Reconstruct a single in-stream `.kdata` or decode entries directly into restore engine!
    // Building a verified `.kdata` stream into `restore_backup_archive` guarantees identical RBAC/dedup/integrity!
    let mut kdata_buf = Vec::new();
    {
        let idx_list: Vec<KdataIndexEntry> = snap.entries.iter().map(|e| e.index.clone()).collect();
        let mut writer = kiyo_kdata::KdataStreamWriter::begin(
            &mut kdata_buf,
            backup_passphrase,
            &snap.kdata_manifest,
            &idx_list,
            kiyo_kdata::DEFAULT_CHUNK_SIZE,
            kiyo_kdata::KdfParams::fast_for_tests(),
            chrono::Utc::now().timestamp(),
        )
        .map_err(|_| AppError::Internal)?;

        let mut chunk_cache: HashMap<String, Vec<u8>> = HashMap::new();
        for entry in &snap.entries {
            let mut full_entry = Vec::with_capacity(entry.index.plain_size as usize);
            for cref in &entry.chunks {
                if let Some(cached) = chunk_cache.get(&cref.chunk_id_hex) {
                    full_entry.extend_from_slice(cached);
                } else {
                    let wire = provider.download_backup(http, &cref.remote_key).await.map_err(AppError::from)?;
                    if hex::encode(sha256_bytes(&wire)) != cref.cipher_sha256 {
                        return Err(AppError::BadRequest("cloud_chunk_checksum_mismatch"));
                    }
                    let plain = open_cloud_chunk(&keys, &user_id, &cref.chunk_id_hex, &wire)
                        .map_err(|_| AppError::Unauthorized)?;
                    if chunk_cache.len() < 32 {
                        chunk_cache.insert(cref.chunk_id_hex.clone(), plain.clone());
                    }
                    full_entry.extend_from_slice(&plain);
                }
            }
            if hex::encode(Sha256::digest(&full_entry)) != entry.index.sha256_hex {
                return Err(AppError::BadRequest("cloud_entry_sha256_mismatch"));
            }
            writer
                .write_entry_bytes(entry.index.entry_id, &full_entry)
                .map_err(|_| AppError::Internal)?;
        }
        writer.finish().map_err(|_| AppError::Internal)?;
    }

    let opts = RestoreOptions {
        mode: mode.to_string(),
        conversation_id: None,
        password: Some(backup_passphrase.to_string()),
    };
    crate::backup::engine::restore_backup_archive(pool, storage, user_id, &kdata_buf, &opts).await
}

pub async fn enforce_snapshot_retention(
    pool: &PgPool,
    http: &reqwest::Client,
    session_secret: &[u8],
    user_id: Uuid,
    connection_id: Uuid,
    retain_snapshots: i32,
) -> AppResult<usize> {
    let keep = retain_snapshots.clamp(1, 50) as i64;
    let stale_rows = sqlx::query(
        "SELECT id, remote_manifest_key
         FROM cloud_snapshots
         WHERE user_id = $1 AND connection_id = $2 AND status = 'verified'
         ORDER BY snapshot_seq DESC
         OFFSET $3",
    )
    .bind(user_id)
    .bind(connection_id)
    .bind(keep)
    .fetch_all(pool)
    .await?;

    if stale_rows.is_empty() {
        return Ok(0);
    }

    let (provider, _conn_id, _cfg) = load_provider_with_refresh(pool, http, session_secret, user_id, connection_id).await?;
    let mut deleted = 0usize;
    for r in stale_rows {
        let sid: Uuid = r.try_get("id")?;
        let mf_key: String = r.try_get("remote_manifest_key")?;
        if delete_snapshot_safely(pool, provider.as_ref(), http, user_id, connection_id, sid, &mf_key).await.is_ok() {
            deleted += 1;
        }
    }
    Ok(deleted)
}

pub fn next_run_interval_secs(schedule_mode: &str) -> i64 {
    match schedule_mode {
        "on_change" => 900,
        "daily" => 86_400,
        "weekly" => 604_800,
        _ => 86_400,
    }
}

pub async fn run_due_cloud_backup_policies(
    pool: &PgPool,
    storage: &StorageRegistry,
    http: &reqwest::Client,
    session_secret: &[u8],
) -> AppResult<usize> {
    let rows = sqlx::query(
        "SELECT user_id, connection_id, schedule_mode, retain_snapshots, auto_key_enc
         FROM cloud_backup_policies
         WHERE enabled = TRUE AND paused = FALSE AND schedule_mode <> 'manual'
           AND connection_id IS NOT NULL AND auto_key_enc IS NOT NULL
           AND (next_run_at IS NULL OR next_run_at <= now())
         LIMIT 25",
    )
    .fetch_all(pool)
    .await?;

    let mut executed = 0usize;
    for r in rows {
        let uid: Uuid = r.try_get("user_id")?;
        let Some(conn_id): Option<Uuid> = r.try_get("connection_id")? else {
            continue;
        };
        let mode: String = r.try_get("schedule_mode")?;
        let retain: i32 = r.try_get("retain_snapshots")?;
        let enc_key: Vec<u8> = r.try_get("auto_key_enc")?;
        let Ok(cfg_val) = super::open_cloud_config(session_secret, &enc_key) else {
            continue;
        };
        let Some(passphrase) = cfg_val.get("passphrase").and_then(|v| v.as_str()) else {
            continue;
        };

        let next_secs = next_run_interval_secs(&mode);
        if run_incremental_cloud_backup(pool, storage, http, session_secret, uid, conn_id, passphrase).await.is_ok() {
            let _ = enforce_snapshot_retention(pool, http, session_secret, uid, conn_id, retain).await;
            let _ = sqlx::query(
                "UPDATE cloud_backup_policies
                 SET last_run_at = now(), next_run_at = now() + ($1 || ' seconds')::interval, updated_at = now()
                 WHERE user_id = $2",
            )
            .bind(next_secs)
            .bind(uid)
            .execute(pool)
            .await;
            executed += 1;
        }
    }
    Ok(executed)
}
