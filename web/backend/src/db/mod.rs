use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::time::Duration;

pub async fn connect(url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .connect(url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let migrator = sqlx::migrate!("./migrations");

    // Check applied migrations before invoking `migrator.run(pool)` so a least-privilege
    // runtime role (`chatw_app`) never acquires `pg_advisory_lock` and fails mid-DDL, which
    // would otherwise keep the session-level advisory lock pinned on a pooled connection.
    if let Ok(rows) = sqlx::query("SELECT version FROM _sqlx_migrations WHERE success = true")
        .fetch_all(pool)
        .await
    {
        let applied_versions: std::collections::HashSet<i64> =
            rows.into_iter().filter_map(|r| r.try_get("version").ok()).collect();
        if migrator.iter().all(|m| applied_versions.contains(&m.version)) {
            return Ok(());
        }
    }

    migrator.run(pool).await
}
