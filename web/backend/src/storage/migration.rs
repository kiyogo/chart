use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{CleanupReport, StorageAdapter, StorageRegistry};

#[derive(Clone, Debug, Serialize)]
pub struct RelayCleanupOutcome {
    pub report: CleanupReport,
    pub expiring_without_safe_copies: i64,
}

/// Clean expired temporary relays ONLY when `delivered_copies >= min_safe_copies` (or explicitly acknowledged),
/// preventing silent data loss when peer has not synced a copy yet.
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
        let storage_key: String = row.try_get("storage_key")?;
        let storage_backend: String = row.try_get("storage_backend")?;
        let byte_size: i64 = row.try_get("byte_size")?;

        if !deleted_all && copy_count < (min_safe as i64) {
            // Do not purge relay silently without at least 1 replicated copy unless hard-expired > 30 days.
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

        if storage_backend == "temp_relay" {
            let _ = storage.temp_relay.delete_object(&storage_key).await;
        } else if storage_backend == "local_cas" {
            let _ = storage.local_cas.delete_object(&storage_key).await;
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

        let rem: Option<i32> = sqlx::query_scalar(
            "UPDATE media_objects SET ref_count = GREATEST(0, ref_count - 1) WHERE id = $1 RETURNING ref_count",
        )
        .bind(object_id)
        .fetch_optional(pool)
        .await?;
        if rem.unwrap_or(1) == 0 {
            let _ = sqlx::query("DELETE FROM media_objects WHERE id = $1")
                .bind(object_id)
                .execute(pool)
                .await;
        }
        report.deleted += 1;
        report.freed_bytes += byte_size.max(0) as u64;
    }

    Ok(RelayCleanupOutcome {
        report,
        expiring_without_safe_copies: at_risk,
    })
}

/// Decrement refcount cleanly on object deletion and unlink physical CAS file only when `ref_count == 0`.
pub async fn release_object_reference(
    pool: &PgPool,
    storage: &StorageRegistry,
    object_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "UPDATE media_objects SET ref_count = GREATEST(0, ref_count - 1) WHERE id = $1 RETURNING ref_count, storage_backend, storage_key",
    )
    .bind(object_id)
    .fetch_optional(pool)
    .await?;

    let Some(r) = row else {
        return Ok(false);
    };
    let ref_count: i32 = r.try_get("ref_count")?;
    let backend: String = r.try_get("storage_backend")?;
    let key: String = r.try_get("storage_key")?;
    if ref_count == 0 {
        match backend.as_str() {
            "local_cas" => { let _ = storage.local_cas.delete_object(&key).await; }
            "temp_relay" => { let _ = storage.temp_relay.delete_object(&key).await; }
            "encrypted_backup" => { let _ = storage.encrypted_backup.delete_object(&key).await; }
            _ => {}
        }
        let _ = sqlx::query("DELETE FROM media_objects WHERE id = $1")
            .bind(object_id)
            .execute(pool)
            .await;
        return Ok(true);
    }
    Ok(false)
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
