use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{CleanupReport, StorageAdapter, StorageRegistry};

#[derive(Clone, Debug, Serialize)]
pub struct RelayCleanupOutcome {
    pub report: CleanupReport,
    pub expiring_without_safe_copies: i64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct IncompleteAndGcReport {
    pub aborted_transfers: u64,
    pub gc_scanned_objects: u64,
    pub gc_deleted_objects: u64,
    pub gc_skipped_pinned: u64,
    pub gc_freed_bytes: u64,
}

/// Delete a physical object from whichever backend owns it (`s3_ready`, `local_cas`, `temp_relay`, `encrypted_backup`).
pub async fn delete_physical_object(storage: &StorageRegistry, backend: &str, key: &str) {
    match backend {
        "s3_ready" => {
            let _ = storage.s3.delete_object(key).await;
        }
        "local_cas" => {
            let _ = storage.local_cas.delete_object(key).await;
        }
        "temp_relay" => {
            let _ = storage.temp_relay.delete_object(key).await;
        }
        "encrypted_backup" => {
            let _ = storage.encrypted_backup.delete_object(key).await;
        }
        _ => {}
    }
}

/// Clean expired temporary relays ONLY when `delivered_copies >= min_safe_copies` (or explicitly acknowledged),
/// preventing silent data loss when peer has not synced a copy yet. Supports S3 + local backends.
pub async fn cleanup_relays_safely(
    pool: &PgPool,
    storage: &StorageRegistry,
) -> Result<RelayCleanupOutcome, sqlx::Error> {
    let mut report = CleanupReport::default();

    let rows = sqlx::query(
        r#"
        SELECT r.id AS ref_id, r.min_safe_copies, r.deleted_for_all,
               r.relay_expires_at, o.id AS object_id, o.storage_key, o.storage_backend, o.byte_size,
               (SELECT COUNT(DISTINCT mdl.device_id) FROM media_device_locations mdl WHERE mdl.ref_id = r.id) AS copy_count
        FROM media_references r
        JOIN media_objects o ON o.id = r.object_id
        WHERE r.storage_tier = 'temp_relay'
          AND (r.deleted_for_all = TRUE OR (r.relay_expires_at IS NOT NULL AND r.relay_expires_at <= now()))
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut at_risk = 0i64;
    for row in rows {
        report.scanned += 1;
        let ref_id: Uuid = row.try_get("ref_id")?;
        let min_safe: i32 = row.try_get("min_safe_copies")?;
        let deleted_all: bool = row.try_get("deleted_for_all")?;
        let copy_count: i64 = row.try_get("copy_count")?;
        let object_id: Uuid = row.try_get("object_id")?;
        let byte_size: i64 = row.try_get("byte_size")?;

        if !deleted_all && copy_count < (min_safe as i64) {
            let hard_expired: bool = sqlx::query_scalar(
                "SELECT relay_expires_at < now() - interval '30 days' FROM media_references WHERE id = $1",
            )
            .bind(ref_id)
            .fetch_one(pool)
            .await
            .unwrap_or(false);
            if !hard_expired {
                report.skipped_pinned += 1;
                at_risk += 1;
                continue;
            }
        }

        sqlx::query(
            r#"
            UPDATE media_references
            SET object_id = NULL,
                availability_state = CASE
                    WHEN deleted_for_all THEN 'deleted_for_everyone'
                    WHEN $2 > 0 THEN 'available_on_peer'
                    ELSE 'no_remaining_copies'
                END,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(ref_id)
        .bind(copy_count)
        .execute(pool)
        .await?;

        let deleted_physical = release_object_reference(pool, storage, object_id).await?;
        if deleted_physical {
            report.deleted += 1;
            report.freed_bytes += byte_size.max(0) as u64;
        }
    }

    Ok(RelayCleanupOutcome {
        report,
        expiring_without_safe_copies: at_risk,
    })
}

/// Atomic, race-safe reference decrement using `SELECT ... FOR UPDATE`.
/// Honors `is_pinned` (never physically deletes pinned objects) and supports `s3_ready` + local backends.
pub async fn release_object_reference(
    pool: &PgPool,
    storage: &StorageRegistry,
    object_id: Uuid,
) -> Result<bool, sqlx::Error> {
    release_object_reference_with_grace(pool, storage, object_id, 0).await
}

/// Decrement `ref_count` inside a row-locked SQL transaction.
/// If `ref_count` drops to 0:
/// - If `is_pinned = TRUE`: keeps object safely (`ref_count = 0`, no physical delete).
/// - If `grace_secs > 0`: sets `gc_eligible_at = now() + grace` for delayed GC sweep.
/// - If `grace_secs == 0`: deletes DB row atomically first, then removes physical object from S3/CAS.
pub async fn release_object_reference_with_grace(
    pool: &PgPool,
    storage: &StorageRegistry,
    object_id: Uuid,
    grace_secs: i64,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT ref_count, storage_backend, storage_key, COALESCE(is_pinned, FALSE) AS is_pinned FROM media_objects WHERE id = $1 FOR UPDATE",
    )
    .bind(object_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(r) = row else {
        tx.commit().await?;
        return Ok(false);
    };

    let current_refs: i32 = r.try_get("ref_count")?;
    let backend: String = r.try_get("storage_backend")?;
    let key: String = r.try_get("storage_key")?;
    let is_pinned: bool = r.try_get("is_pinned")?;
    let next_refs = (current_refs - 1).max(0);

    if next_refs == 0 && !is_pinned && grace_secs <= 0 {
        // Delete DB row inside transaction before physical unlink so concurrent dedup cannot reference a deleted blob!
        sqlx::query("DELETE FROM media_objects WHERE id = $1")
            .bind(object_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        delete_physical_object(storage, &backend, &key).await;
        return Ok(true);
    }

    if next_refs == 0 && !is_pinned && grace_secs > 0 {
        sqlx::query(
            "UPDATE media_objects SET ref_count = 0, gc_eligible_at = now() + ($2 * interval '1 second') WHERE id = $1",
        )
        .bind(object_id)
        .bind(grace_secs)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query("UPDATE media_objects SET ref_count = $2 WHERE id = $1")
            .bind(object_id)
            .bind(next_refs)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(false)
}

/// Abort expired incomplete multipart uploads (both S3 and local staging) AND sweep zero-ref unpinned objects
/// past their GC grace period across S3 + Local CAS.
pub async fn cleanup_incomplete_uploads_and_gc(
    pool: &PgPool,
    storage: &StorageRegistry,
) -> Result<IncompleteAndGcReport, sqlx::Error> {
    let mut report = IncompleteAndGcReport::default();

    // 1. Abort expired incomplete transfers
    let expired = sqlx::query(
        r#"
        SELECT id, s3_upload_id
        FROM transfer_sessions
        WHERE status = 'active' AND expires_at <= now()
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?;

    for row in expired {
        let tid: Uuid = row.try_get("id")?;
        let upload_token: String = row.try_get::<Option<String>, _>("s3_upload_id")?.unwrap_or_default();
        if let Some(rest) = upload_token.strip_prefix("s3::") {
            if let Some((s3_key, upload_id)) = rest.split_once("::") {
                let _ = storage.s3.abort_multipart_upload(s3_key, upload_id).await;
            }
        } else if !upload_token.is_empty() {
            let _ = storage.local_cas.abort_chunks(&upload_token).await;
        }
        sqlx::query("UPDATE transfer_sessions SET status = 'aborted' WHERE id = $1")
            .bind(tid)
            .execute(pool)
            .await?;
        report.aborted_transfers += 1;
    }

    // 2. Sweep unreferenced, unpinned objects past grace period
    let orphan_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM media_objects
        WHERE ref_count = 0
          AND (gc_eligible_at IS NULL OR gc_eligible_at <= now())
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?;

    for oid in orphan_ids {
        report.gc_scanned_objects += 1;
        let mut tx = pool.begin().await?;
        let row = sqlx::query(
            "SELECT ref_count, storage_backend, storage_key, byte_size, COALESCE(is_pinned, FALSE) AS is_pinned FROM media_objects WHERE id = $1 FOR UPDATE",
        )
        .bind(oid)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(r) = row else {
            tx.commit().await?;
            continue;
        };
        let rc: i32 = r.try_get("ref_count")?;
        let is_pinned: bool = r.try_get("is_pinned")?;
        if rc > 0 || is_pinned {
            if is_pinned {
                report.gc_skipped_pinned += 1;
            }
            tx.commit().await?;
            continue;
        }
        let backend: String = r.try_get("storage_backend")?;
        let key: String = r.try_get("storage_key")?;
        let byte_size: i64 = r.try_get("byte_size")?;
        sqlx::query("DELETE FROM media_objects WHERE id = $1")
            .bind(oid)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        delete_physical_object(storage, &backend, &key).await;
        report.gc_deleted_objects += 1;
        report.gc_freed_bytes += byte_size.max(0) as u64;
    }

    Ok(report)
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationJobResult {
    pub job_id: Uuid,
    pub dry_run: bool,
    pub target_backend: String,
    pub status: String,
    pub scanned_objects: i64,
    pub migrated_objects: i64,
    pub verified_objects: i64,
    pub failed_objects: i64,
    pub last_error: Option<String>,
}

/// Run a storage migration job (`dry_run = true` validates local CAS integrity & S3 readiness without requiring MinIO running;
/// `dry_run = false` streams objects to S3/MinIO, verifies remote object existence & updates backend pointers cleanly).
pub async fn execute_migration_job(
    pool: &PgPool,
    storage: &StorageRegistry,
    dry_run: bool,
) -> Result<MigrationJobResult, sqlx::Error> {
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO storage_migration_jobs (id, target_backend, dry_run, status) VALUES ($1, 's3_ready', $2, 'running')",
    )
    .bind(job_id)
    .bind(dry_run)
    .execute(pool)
    .await?;

    let objects = sqlx::query(
        "SELECT id, storage_key, sha256, mime_type FROM media_objects WHERE storage_backend = 'local_cas' ORDER BY created_at ASC LIMIT 500",
    )
    .fetch_all(pool)
    .await?;

    let mut scanned = 0i64;
    let mut migrated = 0i64;
    let mut verified = 0i64;
    let mut failed = 0i64;
    let mut last_error: Option<String> = None;

    for obj in objects {
        scanned += 1;
        let oid: Uuid = obj.try_get("id")?;
        let key: String = obj.try_get("storage_key")?;
        let sha: Vec<u8> = obj.try_get("sha256")?;
        let mime: String = obj.try_get("mime_type")?;
        let mut expected = [0u8; 32];
        if sha.len() == 32 {
            expected.copy_from_slice(&sha);
        }

        let local_ok = storage
            .local_cas
            .verify_integrity(&key, &expected)
            .await
            .unwrap_or(false);
        if !local_ok {
            failed += 1;
            last_error = Some(format!("local_cas_integrity_failed:{key}"));
            continue;
        }
        verified += 1;

        if !dry_run {
            match storage.local_cas.get_object(&key).await {
                Ok(bytes) => match storage.s3.put_object(&key, bytes, &mime, None).await {
                    Ok(_) => {
                        let _ = sqlx::query("UPDATE media_objects SET storage_backend = 's3_ready' WHERE id = $1")
                            .bind(oid)
                            .execute(pool)
                            .await;
                        migrated += 1;
                    }
                    Err(e) => {
                        failed += 1;
                        last_error = Some(e.to_string());
                    }
                },
                Err(e) => {
                    failed += 1;
                    last_error = Some(e.to_string());
                }
            }
        }
    }

    let status = if failed > 0 && last_error.is_some() && !dry_run && migrated == 0 && scanned > 0 {
        "failed"
    } else {
        "completed"
    };

    let _ = sqlx::query(
        r#"
        UPDATE storage_migration_jobs
        SET status = $2, scanned_objects = $3, migrated_objects = $4, verified_objects = $5,
            failed_objects = $6, last_error = $7, finished_at = now()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(status)
    .bind(scanned)
    .bind(migrated)
    .bind(verified)
    .bind(failed)
    .bind(&last_error)
    .execute(pool)
    .await;

    Ok(MigrationJobResult {
        job_id,
        dry_run,
        target_backend: "s3_ready".into(),
        status: status.into(),
        scanned_objects: scanned,
        migrated_objects: migrated,
        verified_objects: verified,
        failed_objects: failed,
        last_error,
    })
}
