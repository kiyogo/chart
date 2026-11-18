use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::permissions;

pub async fn has_server_permission(
    pool: &PgPool,
    server_id: Uuid,
    user_id: Uuid,
    required_perm: i64,
) -> AppResult<bool> {
    // 1. Check if user is owner of server
    let server_owner = sqlx::query_scalar!(
        "SELECT owner_id FROM servers WHERE id = $1",
        server_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("server_not_found"))?;

    if server_owner == user_id {
        return Ok(true);
    }

    // 2. Check if member
    let is_member = sqlx::query_scalar!(
        r#"SELECT 1 as "one!" FROM server_members WHERE server_id = $1 AND user_id = $2"#,
        server_id,
        user_id
    )
    .fetch_optional(pool)
    .await?
    .is_some();

    if !is_member {
        return Ok(false);
    }

    // 3. Compute effective permissions (everyone + member roles)
    let perms = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(BIT_OR(r.permissions), 0) as "perms!"
        FROM server_roles r
        WHERE r.server_id = $1
          AND (
            r.name = '@everyone'
            OR r.id IN (
              SELECT role_id FROM server_member_roles
              WHERE server_id = $1 AND user_id = $2
            )
          )
        "#,
        server_id,
        user_id
    )
    .fetch_one(pool)
    .await?;

    if (perms & permissions::ADMINISTRATOR) != 0 {
        return Ok(true);
    }

    Ok((perms & required_perm) != 0)
}

pub async fn require_server_permission(
    pool: &PgPool,
    server_id: Uuid,
    user_id: Uuid,
    required_perm: i64,
) -> AppResult<()> {
    if has_server_permission(pool, server_id, user_id, required_perm).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden("insufficient_permissions"))
    }
}

