//! Spec 016: Threads-style social feed ("Mới") backed by PostgreSQL & WebSocket updates.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::{are_friends, auth_user, is_blocked_either},
    websocket::WsEvent,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_feed).post(create_post))
        .route("/{id}", delete(delete_post))
        .route("/{id}/replies", get(list_replies))
        .route("/{id}/like", post(toggle_like))
}

#[derive(Deserialize)]
pub struct FeedQuery {
    pub filter: Option<String>, // "all" | "friends" | "mine"
}

#[derive(Deserialize)]
pub struct CreatePostReq {
    pub content: String,
    pub media_url: Option<String>,
    pub visibility: Option<String>,
    pub parent_id: Option<Uuid>,
    pub repost_of_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct RepostPreview {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub author_display_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub content: String,
    pub media_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct FeedPostItem {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub author_display_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub author_nameplate: String,
    pub author_custom_nameplate_url: Option<String>,
    pub author_name_effect: String,
    pub author_profile_color: Option<String>,
    pub parent_id: Option<Uuid>,
    pub repost_of_id: Option<Uuid>,
    pub repost_of: Option<RepostPreview>,
    pub content: String,
    pub media_url: Option<String>,
    pub visibility: String,
    pub like_count: i64,
    pub reply_count: i64,
    pub repost_count: i64,
    pub liked_by_me: bool,
    pub created_at: DateTime<Utc>,
}

fn safe_upload(u: &str) -> bool {
    u.starts_with("/uploads/")
        && !u.contains("..")
        && u.len() <= 240
        && u.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

fn clean_content(raw: &str, allow_empty: bool) -> AppResult<String> {
    let trimmed = raw.trim();
    if !allow_empty && trimmed.is_empty() {
        return Err(AppError::BadRequest("empty_post_content"));
    }
    if trimmed.chars().count() > 1000
        || trimmed
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(AppError::BadRequest("invalid_post_content"));
    }
    Ok(trimmed.to_string())
}

fn map_post_row(r: sqlx::postgres::PgRow) -> FeedPostItem {
    let repost_of = r
        .try_get::<Option<Uuid>, _>("rep_id")
        .ok()
        .flatten()
        .map(|rid| RepostPreview {
            id: rid,
            author_id: r.get("rep_author_id"),
            author_username: r.get("rep_username"),
            author_display_name: r.get("rep_display_name"),
            author_avatar_url: r.get("rep_avatar_url"),
            content: r.get("rep_content"),
            media_url: r.get("rep_media_url"),
            created_at: r.get("rep_created_at"),
        });

    FeedPostItem {
        id: r.get("id"),
        author_id: r.get("author_id"),
        author_username: r.get("author_username"),
        author_display_name: r.get("author_display_name"),
        author_avatar_url: r.get("author_avatar_url"),
        author_nameplate: r.get("author_nameplate"),
        author_custom_nameplate_url: r.get("author_custom_nameplate_url"),
        author_name_effect: r.get("author_name_effect"),
        author_profile_color: r.get("author_profile_color"),
        parent_id: r.get("parent_id"),
        repost_of_id: r.get("repost_of_id"),
        repost_of,
        content: r.get("content"),
        media_url: r.get("media_url"),
        visibility: r.get("visibility"),
        like_count: r.get("like_count"),
        reply_count: r.get("reply_count"),
        repost_count: r.get("repost_count"),
        liked_by_me: r.get("liked_by_me"),
        created_at: r.get("created_at"),
    }
}

async fn list_feed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FeedQuery>,
) -> AppResult<Json<Vec<FeedPostItem>>> {
    let me = auth_user(&state, &headers).await?;
    let filter = q.filter.as_deref().unwrap_or("all");

    let rows = sqlx::query(
        r#"
        SELECT
            p.id, p.author_id, p.parent_id, p.repost_of_id, p.content, p.media_url, p.visibility, p.created_at,
            COALESCE(u.username, '') AS author_username,
            u.display_name AS author_display_name,
            u.avatar_url AS author_avatar_url,
            u.nameplate AS author_nameplate,
            u.custom_nameplate_url AS author_custom_nameplate_url,
            u.name_effect AS author_name_effect,
            u.profile_color AS author_profile_color,
            (SELECT COUNT(*) FROM feed_likes l WHERE l.post_id = p.id) AS like_count,
            (SELECT COUNT(*) FROM feed_posts rp WHERE rp.parent_id = p.id) AS reply_count,
            (SELECT COUNT(*) FROM feed_posts rq WHERE rq.repost_of_id = p.id) AS repost_count,
            EXISTS(SELECT 1 FROM feed_likes l WHERE l.post_id = p.id AND l.user_id = $1) AS liked_by_me,
            rp_orig.id AS rep_id,
            rp_orig.author_id AS rep_author_id,
            COALESCE(ru.username, '') AS rep_username,
            ru.display_name AS rep_display_name,
            ru.avatar_url AS rep_avatar_url,
            rp_orig.content AS rep_content,
            rp_orig.media_url AS rep_media_url,
            rp_orig.created_at AS rep_created_at
        FROM feed_posts p
        JOIN users u ON u.id = p.author_id AND u.suspended_at IS NULL AND u.username IS NOT NULL
        LEFT JOIN feed_posts rp_orig ON rp_orig.id = p.repost_of_id
        LEFT JOIN users ru ON ru.id = rp_orig.author_id
        WHERE p.parent_id IS NULL
          AND NOT EXISTS (
              SELECT 1 FROM user_blocks b
              WHERE (b.blocker_id = $1 AND b.blocked_id = p.author_id)
                 OR (b.blocker_id = p.author_id AND b.blocked_id = $1)
          )
          AND (
              CASE
                  WHEN $2 = 'mine' THEN p.author_id = $1
                  WHEN $2 = 'friends' THEN (
                      (p.author_id = $1 AND p.visibility <> 'self')
                      OR (
                          p.visibility IN ('friends', 'public')
                          AND EXISTS (
                              SELECT 1 FROM friendships f
                              WHERE f.user_a = LEAST($1, p.author_id) AND f.user_b = GREATEST($1, p.author_id)
                          )
                      )
                  )
                  ELSE (
                      p.author_id = $1
                      OR p.visibility = 'public'
                      OR (
                          p.visibility = 'friends'
                          AND EXISTS (
                              SELECT 1 FROM friendships f
                              WHERE f.user_a = LEAST($1, p.author_id) AND f.user_b = GREATEST($1, p.author_id)
                          )
                      )
                  )
              END
          )
        ORDER BY p.created_at DESC
        LIMIT 60
        "#,
    )
    .bind(me)
    .bind(filter)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(map_post_row).collect()))
}

async fn list_replies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(post_id): Path<Uuid>,
) -> AppResult<Json<Vec<FeedPostItem>>> {
    let me = auth_user(&state, &headers).await?;

    // Check caller can view parent post
    let parent_meta = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT author_id, visibility FROM feed_posts WHERE id = $1",
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("post_not_found"))?;

    if parent_meta.0 != me {
        if is_blocked_either(&state.db, me, parent_meta.0).await? {
            return Err(AppError::NotFound("post_not_found"));
        }
        if parent_meta.1 == "self" {
            return Err(AppError::Forbidden("post_private"));
        }
        if parent_meta.1 == "friends" && !are_friends(&state.db, me, parent_meta.0).await? {
            return Err(AppError::Forbidden("post_friends_only"));
        }
    }

    let rows = sqlx::query(
        r#"
        SELECT
            p.id, p.author_id, p.parent_id, p.repost_of_id, p.content, p.media_url, p.visibility, p.created_at,
            COALESCE(u.username, '') AS author_username,
            u.display_name AS author_display_name,
            u.avatar_url AS author_avatar_url,
            u.nameplate AS author_nameplate,
            u.custom_nameplate_url AS author_custom_nameplate_url,
            u.name_effect AS author_name_effect,
            u.profile_color AS author_profile_color,
            (SELECT COUNT(*) FROM feed_likes l WHERE l.post_id = p.id) AS like_count,
            (SELECT COUNT(*) FROM feed_posts rp WHERE rp.parent_id = p.id) AS reply_count,
            (SELECT COUNT(*) FROM feed_posts rq WHERE rq.repost_of_id = p.id) AS repost_count,
            EXISTS(SELECT 1 FROM feed_likes l WHERE l.post_id = p.id AND l.user_id = $1) AS liked_by_me,
            NULL::uuid AS rep_id,
            NULL::uuid AS rep_author_id,
            NULL::text AS rep_username,
            NULL::text AS rep_display_name,
            NULL::text AS rep_avatar_url,
            NULL::text AS rep_content,
            NULL::text AS rep_media_url,
            NULL::timestamptz AS rep_created_at
        FROM feed_posts p
        JOIN users u ON u.id = p.author_id AND u.suspended_at IS NULL AND u.username IS NOT NULL
        WHERE p.parent_id = $2
          AND NOT EXISTS (
              SELECT 1 FROM user_blocks b
              WHERE (b.blocker_id = $1 AND b.blocked_id = p.author_id)
                 OR (b.blocker_id = p.author_id AND b.blocked_id = $1)
          )
        ORDER BY p.created_at ASC
        LIMIT 100
        "#,
    )
    .bind(me)
    .bind(post_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(map_post_row).collect()))
}

async fn notify_friends_feed(state: &AppState, actor_id: Uuid, event_data: Value) {
    let ev = WsEvent::FeedUpdated(event_data);
    state.ws_hub.send_to_user(&actor_id, ev.clone()).await;
    if let Ok(friend_ids) = sqlx::query_scalar::<_, Uuid>(
        "SELECT CASE WHEN user_a = $1 THEN user_b ELSE user_a END FROM friendships WHERE user_a = $1 OR user_b = $1",
    )
    .bind(actor_id)
    .fetch_all(&state.db)
    .await
    {
        for fid in friend_ids {
            state.ws_hub.send_to_user(&fid, ev.clone()).await;
        }
    }
}

async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(p): Json<CreatePostReq>,
) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let media_url = p
        .media_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if let Some(ref m) = media_url {
        if !safe_upload(m) {
            return Err(AppError::BadRequest("invalid_media_url"));
        }
    }
    let has_attachment_or_repost = media_url.is_some() || p.repost_of_id.is_some();
    let content = clean_content(&p.content, has_attachment_or_repost)?;

    let mut visibility = p.visibility.as_deref().unwrap_or("friends").to_string();
    if !matches!(visibility.as_str(), "friends" | "public" | "self") {
        return Err(AppError::BadRequest("invalid_visibility"));
    }

    if let Some(parent_id) = p.parent_id {
        let parent_meta = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT author_id, visibility FROM feed_posts WHERE id = $1",
        )
        .bind(parent_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("parent_post_not_found"))?;

        if parent_meta.0 != me {
            if is_blocked_either(&state.db, me, parent_meta.0).await? {
                return Err(AppError::NotFound("parent_post_not_found"));
            }
            if parent_meta.1 == "self" {
                return Err(AppError::Forbidden("cannot_reply_private_post"));
            }
            if parent_meta.1 == "friends" && !are_friends(&state.db, me, parent_meta.0).await? {
                return Err(AppError::Forbidden("post_friends_only"));
            }
        }
        visibility = parent_meta.1;
    }

    if let Some(rep_id) = p.repost_of_id {
        let rep_meta = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT author_id, visibility FROM feed_posts WHERE id = $1",
        )
        .bind(rep_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("repost_target_not_found"))?;

        if rep_meta.0 != me {
            if is_blocked_either(&state.db, me, rep_meta.0).await? {
                return Err(AppError::NotFound("repost_target_not_found"));
            }
            if rep_meta.1 == "self" {
                return Err(AppError::Forbidden("cannot_repost_private_post"));
            }
        }
    }

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO feed_posts (author_id, parent_id, repost_of_id, content, media_url, visibility) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(me)
    .bind(p.parent_id)
    .bind(p.repost_of_id)
    .bind(&content)
    .bind(&media_url)
    .bind(&visibility)
    .fetch_one(&state.db)
    .await?;

    let state_bg = state.clone();
    tokio::spawn(async move {
        notify_friends_feed(
            &state_bg,
            me,
            json!({ "kind": "post.created", "id": id, "parent_id": p.parent_id }),
        )
        .await;
    });

    Ok(Json(json!({ "id": id })))
}

async fn toggle_like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(post_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let meta = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT author_id, visibility FROM feed_posts WHERE id = $1",
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("post_not_found"))?;

    if meta.0 != me {
        if is_blocked_either(&state.db, me, meta.0).await? {
            return Err(AppError::NotFound("post_not_found"));
        }
        if meta.1 == "self" {
            return Err(AppError::Forbidden("post_private"));
        }
        if meta.1 == "friends" && !are_friends(&state.db, me, meta.0).await? {
            return Err(AppError::Forbidden("post_friends_only"));
        }
    }

    let del = sqlx::query("DELETE FROM feed_likes WHERE post_id = $1 AND user_id = $2")
        .bind(post_id)
        .bind(me)
        .execute(&state.db)
        .await?;

    let liked = if del.rows_affected() > 0 {
        false
    } else {
        sqlx::query("INSERT INTO feed_likes (post_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(post_id)
            .bind(me)
            .execute(&state.db)
            .await?;
        true
    };

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feed_likes WHERE post_id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({ "liked": liked, "like_count": count })))
}

async fn delete_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(post_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let res = sqlx::query("DELETE FROM feed_posts WHERE id = $1 AND author_id = $2")
        .bind(post_id)
        .bind(me)
        .execute(&state.db)
        .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("post_not_found"));
    }

    let state_bg = state.clone();
    tokio::spawn(async move {
        notify_friends_feed(&state_bg, me, json!({ "kind": "post.deleted", "id": post_id })).await;
    });

    Ok(Json(json!({ "deleted": true })))
}
