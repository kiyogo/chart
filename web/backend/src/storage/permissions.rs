use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::permissions as perm_flags,
    servers::permissions as server_perms,
};

/// Verify whether `user_id` is allowed to read/download media belonging to `(scope_type, scope_id, owner_id)`.
pub async fn authorize_read(
    pool: &PgPool,
    user_id: Uuid,
    scope_type: &str,
    scope_id: Option<Uuid>,
    owner_id: Uuid,
) -> AppResult<()> {
    if user_id == owner_id {
        return Ok(());
    }
    match scope_type {
        "dm" => match scope_id {
            Some(peer_id) if peer_id == user_id => Ok(()),
            Some(_) => Err(AppError::Forbidden("attachment_forbidden")),
            None => authorize_legacy_attachment_read(pool, user_id, owner_id).await,
        },
        "group" => {
            let gid = scope_id.ok_or(AppError::Forbidden("attachment_forbidden"))?;
            let is_member: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
            )
            .bind(gid)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
            if is_member {
                Ok(())
            } else {
                Err(AppError::Forbidden("not_a_group_member"))
            }
        }
        "channel" => {
            let cid = scope_id.ok_or(AppError::Forbidden("attachment_forbidden"))?;
            authorize_channel_access(pool, user_id, cid, perm_flags::VIEW_CHANNELS).await
        }
        "avatar" | "library" | "sticker" => Ok(()),
        "banner" => Err(AppError::Forbidden("attachment_forbidden")),
        _ => Err(AppError::Forbidden("attachment_forbidden")),
    }
}

/// Verify whether `user_id` is authorized to upload/write media inside `(scope_type, scope_id)`.
pub async fn authorize_write(
    pool: &PgPool,
    user_id: Uuid,
    scope_type: &str,
    scope_id: Option<Uuid>,
) -> AppResult<()> {
    match scope_type {
        "dm" => {
            let Some(peer_id) = scope_id else {
                return Ok(());
            };
            if peer_id == user_id {
                return Ok(());
            }
            if crate::privacy::is_blocked_either(pool, user_id, peer_id).await? {
                return Err(AppError::Forbidden("blocked"));
            }
            let u_a = user_id.min(peer_id);
            let u_b = user_id.max(peer_id);
            let friends: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM friendships WHERE user_a = $1 AND user_b = $2)",
            )
            .bind(u_a)
            .bind(u_b)
            .fetch_one(pool)
            .await?;
            if friends {
                Ok(())
            } else {
                Err(AppError::Forbidden("not_friends"))
            }
        }
        "group" => {
            let gid = scope_id.ok_or(AppError::BadRequest("missing_scope_id"))?;
            let is_member: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
            )
            .bind(gid)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
            if is_member {
                Ok(())
            } else {
                Err(AppError::Forbidden("not_a_group_member"))
            }
        }
        "channel" => {
            let cid = scope_id.ok_or(AppError::BadRequest("missing_scope_id"))?;
            authorize_channel_access(pool, user_id, cid, perm_flags::SEND_MESSAGES).await
        }
        "avatar" | "library" | "sticker" => Ok(()),
        "banner" => Err(AppError::Forbidden("attachment_forbidden")),
        _ => Err(AppError::BadRequest("invalid_scope_type")),
    }
}

/// Strict RBAC verification for Discord-style channels (checks server membership, channel privacy, and role perms).
pub async fn authorize_channel_access(
    pool: &PgPool,
    user_id: Uuid,
    channel_id: Uuid,
    required_perm: i64,
) -> AppResult<()> {
    let row = sqlx::query(
        "SELECT server_id, media_retention_policy, is_private FROM channels WHERE id = $1",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("channel_not_found"))?;

    let server_id: Uuid = row.try_get("server_id")?;
    let is_private: bool = row.try_get("is_private").unwrap_or(false);
    let retention: String = row.try_get("media_retention_policy").unwrap_or_else(|_| "durable".into());
    if required_perm == perm_flags::SEND_MESSAGES && retention == "no_media" {
        return Err(AppError::Forbidden("channel_media_disabled"));
    }

    if is_private {
        server_perms::require_server_permission(pool, server_id, user_id, perm_flags::ADMINISTRATOR).await?;
    }
    server_perms::require_server_permission(pool, server_id, user_id, perm_flags::VIEW_CHANNELS).await?;
    if required_perm != perm_flags::VIEW_CHANNELS {
        server_perms::require_server_permission(pool, server_id, user_id, required_perm).await?;
    }
    Ok(())
}

/// Legacy attachment authorization when `attachments.scope_type IS NULL` and `media_references` row does not exist:
/// allows owner OR existing friends of the uploader (never arbitrary unacquainted users!).
pub async fn authorize_legacy_attachment_read(
    pool: &PgPool,
    caller_id: Uuid,
    uploader_id: Uuid,
) -> AppResult<()> {
    if caller_id == uploader_id {
        return Ok(());
    }
    let u_a = caller_id.min(uploader_id);
    let u_b = caller_id.max(uploader_id);
    let share_friend: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM friendships WHERE user_a = $1 AND user_b = $2)",
    )
    .bind(u_a)
    .bind(u_b)
    .fetch_one(pool)
    .await?;
    if share_friend {
        return Ok(());
    }
    let share_group: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM group_members gm1 JOIN group_members gm2 ON gm1.group_id = gm2.group_id WHERE gm1.user_id = $1 AND gm2.user_id = $2)",
    )
    .bind(caller_id)
    .bind(uploader_id)
    .fetch_one(pool)
    .await?;
    if share_group {
        return Ok(());
    }
    let share_server: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM server_members sm1 JOIN server_members sm2 ON sm1.server_id = sm2.server_id WHERE sm1.user_id = $1 AND sm2.user_id = $2)",
    )
    .bind(caller_id)
    .bind(uploader_id)
    .fetch_one(pool)
    .await?;
    if share_server {
        Ok(())
    } else {
        Err(AppError::Forbidden("attachment_forbidden"))
    }
}
