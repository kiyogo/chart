use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::get,
};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
};
use chatw_core::AuditLogItem;

pub fn routes() -> Router<AppState> {
    Router::new().route("/logs", get(handle_get_audit_logs))
}

pub async fn log_audit_event(
    db: &sqlx::PgPool,
    user_id: Uuid,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (user_id, action, target_type, target_id, ip_address, user_agent, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        user_id,
        action,
        target_type,
        target_id,
        ip_address,
        user_agent,
        metadata
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn handle_get_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AuditLogItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, user_id, action, target_type, target_id, ip_address, created_at
        FROM audit_logs
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| AuditLogItem {
            id: r.id,
            user_id: r.user_id,
            action: r.action,
            target_type: r.target_type,
            target_id: r.target_id,
            ip_address: r.ip_address,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(items))
}

