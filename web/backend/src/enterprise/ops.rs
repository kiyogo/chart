use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    auth::AppState,
    community::audit,
    enterprise::{
        models::{clean_opt, clean_req, emit_enterprise, ent_perm, load_ctx, safe_upload_url, safe_webhook_url},
        trigger_workflows,
    },
    error::{AppError, AppResult},
    privacy::auth_user,
};

// ---------------------------------------------------------------- Form Templates & Approvals

#[derive(Serialize, sqlx::FromRow)]
pub struct FormTemplateItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub fields: Value,
    pub workflow_steps: Value,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ApprovalRequestItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub template_id: Option<Uuid>,
    pub title: String,
    pub category: String,
    pub requester_id: Uuid,
    pub requester_name: String,
    pub department_id: Option<Uuid>,
    pub form_data: Value,
    pub attachment_urls: Vec<String>,
    pub status: String,
    pub current_step: i32,
    pub total_steps: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_approvals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let templates: Vec<FormTemplateItem> = sqlx::query_as(
        "SELECT id, server_id, name, category, description, fields, workflow_steps FROM enterprise_form_templates WHERE server_id = $1 ORDER BY created_at ASC",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let can_all = ctx.is_admin() || ctx.has(ent_perm::APPROVE_REQUESTS);
    let requests: Vec<ApprovalRequestItem> = sqlx::query_as(
        r#"
        SELECT r.id, r.server_id, r.template_id, r.title, r.category,
               r.requester_id, u.username AS requester_name, r.department_id,
               r.form_data, r.attachment_urls, r.status, r.current_step, r.total_steps,
               r.created_at, r.updated_at
        FROM enterprise_approval_requests r
        JOIN users u ON u.id = r.requester_id
        WHERE r.server_id = $1 AND ($2::bool OR r.requester_id = $3)
        ORDER BY r.created_at DESC
        LIMIT 200
        "#,
    )
    .bind(server_id)
    .bind(can_all)
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "templates": templates, "requests": requests })))
}

#[derive(Deserialize)]
pub struct CreateTemplateReq {
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub fields: Value,
    pub workflow_steps: Option<Value>,
}

pub async fn create_form_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateTemplateReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_ENTERPRISE)?;

    let name = clean_req(&p.name, 1, 80, "invalid_template_name")?;
    if !["leave", "purchase", "payment", "equipment", "it_support", "travel", "attendance_fix", "project_proposal", "access_request", "custom"].contains(&p.category.as_str()) {
        return Err(AppError::BadRequest("invalid_category"));
    }
    let desc = clean_opt(p.description.as_deref(), 300, "invalid_description")?;
    let steps = p.workflow_steps.unwrap_or_else(|| json!([{ "label": "Manage browsing", "mode": "sequential" }]));
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_form_templates (server_id, name, category, description, fields, workflow_steps, created_by) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(server_id)
    .bind(&name)
    .bind(&p.category)
    .bind(desc)
    .bind(p.fields)
    .bind(steps)
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    audit(&state, server_id, uid, "ent_template_created", &name).await;
    emit_enterprise(&state, server_id, "approvals.updated", json!({ "template_id": id })).await;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct SubmitApprovalReq {
    pub template_id: Option<Uuid>,
    pub title: String,
    pub category: String,
    pub department_id: Option<Uuid>,
    pub form_data: Value,
    pub attachment_urls: Option<Vec<String>>,
}

pub async fn submit_approval(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<SubmitApprovalReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let _ctx = load_ctx(&state.db, server_id, uid).await?;

    let title = clean_req(&p.title, 1, 120, "invalid_title")?;
    if !["leave", "purchase", "payment", "equipment", "it_support", "travel", "attendance_fix", "project_proposal", "access_request", "custom"].contains(&p.category.as_str()) {
        return Err(AppError::BadRequest("invalid_category"));
    }
    let atts: Vec<String> = p
        .attachment_urls
        .unwrap_or_default()
        .into_iter()
        .filter(|u| safe_upload_url(u))
        .take(10)
        .collect();

    let total_steps: i32 = if let Some(tid) = p.template_id {
        let steps: Option<Value> = sqlx::query_scalar(
            "SELECT workflow_steps FROM enterprise_form_templates WHERE id = $1 AND server_id = $2",
        )
        .bind(tid)
        .bind(server_id)
        .fetch_optional(&state.db)
        .await?;
        steps.and_then(|v| v.as_array().map(|a| a.len().max(1) as i32)).unwrap_or(1)
    } else {
        1
    };

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_approval_requests (server_id, template_id, title, category, requester_id, department_id, form_data, attachment_urls, total_steps) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
    )
    .bind(server_id)
    .bind(p.template_id)
    .bind(&title)
    .bind(&p.category)
    .bind(uid)
    .bind(p.department_id)
    .bind(p.form_data)
    .bind(&atts)
    .bind(total_steps)
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_approval_submitted", &title).await;
    emit_enterprise(&state, server_id, "approvals.updated", json!({ "id": id })).await;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct DecideApprovalReq {
    pub decision: String, // "approved" | "rejected" | "revision" | "withdrawn"
    pub comment: Option<String>,
}

pub async fn decide_approval(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, req_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<DecideApprovalReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    if !["approved", "rejected", "revision", "withdrawn"].contains(&p.decision.as_str()) {
        return Err(AppError::BadRequest("invalid_decision"));
    }

    let mut tx = state.db.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, String, i32, i32)>(
        "SELECT requester_id, status, current_step, total_steps FROM enterprise_approval_requests WHERE id = $1 AND server_id = $2",
    )
    .bind(req_id)
    .bind(server_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("approval_not_found"))?;

    if row.1 != "pending" && row.1 != "needs_revision" {
        return Err(AppError::BadRequest("already_closed"));
    }

    if p.decision == "withdrawn" {
        if row.0 != uid {
            return Err(AppError::Forbidden("only_requester_can_withdraw"));
        }
    } else {
        ctx.require(ent_perm::APPROVE_REQUESTS)?;
        if row.0 == uid && !ctx.is_owner {
            return Err(AppError::Forbidden("cannot_approve_own_request"));
        }
    }

    let comment = clean_opt(p.comment.as_deref(), 500, "invalid_comment")?;
    sqlx::query(
        "INSERT INTO enterprise_approval_decisions (request_id, step_index, approver_id, decision, comment) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(req_id)
    .bind(row.2)
    .bind(uid)
    .bind(&p.decision)
    .bind(&comment)
    .execute(&mut *tx)
    .await?;

    let (next_step, next_status) = match p.decision.as_str() {
        "approved" => {
            let nxt = row.2 + 1;
            if nxt >= row.3 {
                (nxt, "approved")
            } else {
                (nxt, "pending")
            }
        }
        "rejected" => (row.2, "rejected"),
        "revision" => (row.2, "needs_revision"),
        _ => (row.2, "withdrawn"),
    };

    sqlx::query(
        "UPDATE enterprise_approval_requests SET current_step = $3, status = $4, updated_at = now() WHERE id = $1 AND server_id = $2",
    )
    .bind(req_id)
    .bind(server_id)
    .bind(next_step)
    .bind(next_status)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    audit(&state, server_id, uid, "ent_approval_decided", &format!("{req_id}:{next_status}")).await;
    emit_enterprise(&state, server_id, "approvals.updated", json!({ "id": req_id, "status": next_status })).await;

    let state_bg = state.clone();
    let st_copy = next_status.to_string();
    tokio::spawn(async move {
        trigger_workflows(&state_bg, server_id, "approval_decided", json!({ "request_id": req_id, "status": st_copy, "requester_id": row.0 }), 0).await;
    });
    Ok(Json(json!({ "status": next_status, "current_step": next_step })))
}

// ---------------------------------------------------------------- Documents

#[derive(Deserialize)]
pub struct DocQuery {
    pub scope: Option<String>,
    pub trashed: Option<bool>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FolderItem {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub scope: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DocumentItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub scope: String,
    pub department_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub owner_name: String,
    pub title: String,
    pub file_url: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub pinned: bool,
    pub version: i32,
    pub trashed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_documents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(q): Query<DocQuery>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    let is_all = ctx.is_admin() || ctx.has(ent_perm::MANAGE_DOCUMENTS);
    let trashed = q.trashed.unwrap_or(false);

    let folders: Vec<FolderItem> = sqlx::query_as(
        "SELECT id, parent_id, scope, name, created_at FROM enterprise_folders WHERE server_id = $1 AND ($2::text IS NULL OR scope = $2) ORDER BY name ASC",
    )
    .bind(server_id)
    .bind(q.scope.as_deref().filter(|s| !s.is_empty()))
    .fetch_all(&state.db)
    .await?;

    let docs: Vec<DocumentItem> = sqlx::query_as(
        r#"
        SELECT d.id, d.server_id, d.folder_id, d.scope, d.department_id, d.project_id,
               d.owner_id, u.username AS owner_name, d.title, d.file_url, d.file_name,
               d.file_size, d.mime_type, d.pinned, d.version, d.trashed_at, d.updated_at
        FROM enterprise_documents d
        JOIN users u ON u.id = d.owner_id
        WHERE d.server_id = $1
          AND (($2::bool AND d.trashed_at IS NOT NULL) OR (NOT $2 AND d.trashed_at IS NULL))
          AND ($3::text IS NULL OR d.scope = $3)
          AND ($4::bool OR d.scope = 'company' OR d.owner_id = $5
               OR (d.scope = 'department' AND d.department_id = ANY($6)))
        ORDER BY d.pinned DESC, d.updated_at DESC
        LIMIT 300
        "#,
    )
    .bind(server_id)
    .bind(trashed)
    .bind(q.scope.as_deref().filter(|s| !s.is_empty()))
    .bind(is_all)
    .bind(uid)
    .bind(&ctx.dept_ids)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "folders": folders, "documents": docs })))
}

#[derive(Deserialize)]
pub struct CreateDocReq {
    pub title: String,
    pub scope: Option<String>,
    pub folder_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub file_url: String,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
}

pub async fn create_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateDocReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let scope = p.scope.as_deref().unwrap_or("company");
    if !["company", "department", "project", "personal"].contains(&scope) {
        return Err(AppError::BadRequest("invalid_scope"));
    }
    if scope != "personal" && !ctx.has(ent_perm::MANAGE_DOCUMENTS) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_document_permissions"));
    }
    if !safe_upload_url(&p.file_url) {
        return Err(AppError::BadRequest("invalid_file_url"));
    }
    let title = clean_req(&p.title, 1, 120, "invalid_title")?;
    let fname = clean_req(&p.file_name, 1, 160, "invalid_file_name")?;
    let fsize = p.file_size.unwrap_or(0).max(0);

    // Check configured storage quota if set
    let policy: Option<Value> = sqlx::query_scalar(
        "SELECT document_policy FROM server_enterprise_configs WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    if let Some(max_mb) = policy.as_ref().and_then(|v| v.get("quota_mb").and_then(|x| x.as_i64())) {
        if max_mb > 0 {
            let used_bytes: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(file_size), 0)::bigint FROM enterprise_documents WHERE server_id = $1 AND trashed_at IS NULL",
            )
            .bind(server_id)
            .fetch_one(&state.db)
            .await?;
            if used_bytes + fsize > max_mb * 1024 * 1024 {
                return Err(AppError::PayloadTooLarge);
            }
        }
    }

    let mut tx = state.db.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_documents (server_id, folder_id, scope, department_id, project_id, owner_id, title, file_url, file_name, file_size, mime_type) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
    )
    .bind(server_id)
    .bind(p.folder_id)
    .bind(scope)
    .bind(p.department_id)
    .bind(p.project_id)
    .bind(uid)
    .bind(&title)
    .bind(p.file_url.trim())
    .bind(&fname)
    .bind(fsize)
    .bind(p.mime_type.as_deref().unwrap_or("application/octet-stream"))
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO enterprise_document_versions (document_id, version, file_url, file_size, uploaded_by) VALUES ($1, 1, $2, $3, $4)")
        .bind(id)
        .bind(p.file_url.trim())
        .bind(fsize)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(&state, server_id, uid, "ent_doc_created", &title).await;
    emit_enterprise(&state, server_id, "documents.updated", json!({ "id": id })).await;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct DocActionReq {
    pub action: String, // "pin" | "unpin" | "trash" | "restore" | "rename" | "share" | "bump_version"
    pub title: Option<String>,
    pub expires_hours: Option<i64>,
    pub file_url: Option<String>,
    pub file_size: Option<i64>,
}

pub async fn update_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, doc_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<DocActionReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let owner: Option<Uuid> = sqlx::query_scalar(
        "SELECT owner_id FROM enterprise_documents WHERE id = $1 AND server_id = $2",
    )
    .bind(doc_id)
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    let owner = owner.ok_or(AppError::NotFound("document_not_found"))?;
    if owner != uid && !ctx.has(ent_perm::MANAGE_DOCUMENTS) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_document_permissions"));
    }

    match p.action.as_str() {
        "pin" => {
            sqlx::query("UPDATE enterprise_documents SET pinned = TRUE, updated_at = now() WHERE id = $1 AND server_id = $2")
                .bind(doc_id).bind(server_id).execute(&state.db).await?;
        }
        "unpin" => {
            sqlx::query("UPDATE enterprise_documents SET pinned = FALSE, updated_at = now() WHERE id = $1 AND server_id = $2")
                .bind(doc_id).bind(server_id).execute(&state.db).await?;
        }
        "trash" => {
            sqlx::query("UPDATE enterprise_documents SET trashed_at = now(), updated_at = now() WHERE id = $1 AND server_id = $2")
                .bind(doc_id).bind(server_id).execute(&state.db).await?;
        }
        "restore" => {
            sqlx::query("UPDATE enterprise_documents SET trashed_at = NULL, updated_at = now() WHERE id = $1 AND server_id = $2")
                .bind(doc_id).bind(server_id).execute(&state.db).await?;
        }
        "rename" => {
            let t = clean_req(p.title.as_deref().unwrap_or(""), 1, 120, "invalid_title")?;
            sqlx::query("UPDATE enterprise_documents SET title = $3, updated_at = now() WHERE id = $1 AND server_id = $2")
                .bind(doc_id).bind(server_id).bind(t).execute(&state.db).await?;
        }
        "bump_version" => {
            if let Some(ref fu) = p.file_url {
                if !safe_upload_url(fu) {
                    return Err(AppError::BadRequest("invalid_file_url"));
                }
            }
            let (next_ver, cur_url, cur_size): (i32, String, i64) = sqlx::query_as(
                r#"
                UPDATE enterprise_documents
                SET version = version + 1,
                    file_url = COALESCE($3, file_url),
                    file_size = COALESCE($4, file_size),
                    updated_at = now()
                WHERE id = $1 AND server_id = $2
                RETURNING version, file_url, file_size
                "#,
            )
            .bind(doc_id)
            .bind(server_id)
            .bind(p.file_url.as_deref())
            .bind(p.file_size)
            .fetch_one(&state.db)
            .await?;

            sqlx::query(
                "INSERT INTO enterprise_document_versions (document_id, version, file_url, file_size, uploaded_by) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(doc_id)
            .bind(next_ver)
            .bind(&cur_url)
            .bind(cur_size)
            .bind(uid)
            .execute(&state.db)
            .await?;
            audit(&state, server_id, uid, "ent_doc_version_bumped", &format!("{doc_id}:v{next_ver}")).await;
        }
        "share" => {
            let hours = p.expires_hours.unwrap_or(24).clamp(1, 24 * 30);
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).map_err(|_| AppError::Internal)?;
            let token = hex::encode(bytes);
            let exp = Utc::now() + Duration::hours(hours);
            sqlx::query("INSERT INTO enterprise_document_shares (token, server_id, document_id, expires_at, created_by) VALUES ($1,$2,$3,$4,$5)")
                .bind(&token).bind(server_id).bind(doc_id).bind(exp).bind(uid).execute(&state.db).await?;
            audit(&state, server_id, uid, "ent_doc_shared", &doc_id.to_string()).await;
            return Ok(Json(json!({ "token": token, "expires_at": exp, "url": format!("/api/enterprise/shared/{token}") })));
        }
        _ => return Err(AppError::BadRequest("invalid_action")),
    }
    emit_enterprise(&state, server_id, "documents.updated", json!({ "id": doc_id })).await;
    Ok(Json(json!({ "ok": true })))
}

pub async fn resolve_shared_document(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let row = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>)>(
        r#"
        SELECT d.id, d.title, d.file_url, s.expires_at
        FROM enterprise_document_shares s
        JOIN enterprise_documents d ON d.id = s.document_id
        WHERE s.token = $1 AND d.trashed_at IS NULL
        "#,
    )
    .bind(token.trim())
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("share_not_found"))?;

    if row.3 <= Utc::now() {
        return Err(AppError::BadRequest("share_expired"));
    }
    Ok(Json(json!({ "document_id": row.0, "title": row.1, "file_url": row.2, "expires_at": row.3 })))
}

// ---------------------------------------------------------------- Announcements

#[derive(Serialize, sqlx::FromRow)]
pub struct AnnouncementItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub scope: String,
    pub author_id: Uuid,
    pub author_name: String,
    pub title: String,
    pub content: String,
    pub priority: String,
    pub pinned: bool,
    pub read_count: i64,
    pub read_by_me: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn list_announcements(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<AnnouncementItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let rows: Vec<AnnouncementItem> = sqlx::query_as(
        r#"
        SELECT a.id, a.server_id, a.scope, a.author_id, u.username AS author_name,
               a.title, a.content, a.priority, a.pinned,
               (SELECT COUNT(*) FROM enterprise_announcement_reads r WHERE r.announcement_id = a.id) AS read_count,
               EXISTS(SELECT 1 FROM enterprise_announcement_reads r WHERE r.announcement_id = a.id AND r.user_id = $2) AS read_by_me,
               a.created_at
        FROM enterprise_announcements a
        JOIN users u ON u.id = a.author_id
        WHERE a.server_id = $1
          AND (a.scope = 'company' OR $4::bool OR (a.department_id IS NOT NULL AND a.department_id = ANY($3)))
        ORDER BY a.pinned DESC, a.created_at DESC
        LIMIT 100
        "#,
    )
    .bind(server_id)
    .bind(uid)
    .bind(&ctx.dept_ids)
    .bind(ctx.is_admin())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateAnnouncementReq {
    pub title: String,
    pub content: String,
    pub scope: Option<String>,
    pub department_id: Option<Uuid>,
    pub priority: Option<String>,
    pub pinned: Option<bool>,
}

pub async fn create_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateAnnouncementReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::POST_ANNOUNCEMENTS)?;

    let title = clean_req(&p.title, 1, 140, "invalid_title")?;
    let content = clean_req(&p.content, 1, 4000, "invalid_content")?;
    let scope = p.scope.as_deref().unwrap_or("company");
    let priority = p.priority.as_deref().unwrap_or("normal");
    if !["company", "department", "project"].contains(&scope) || !["normal", "important", "urgent"].contains(&priority) {
        return Err(AppError::BadRequest("invalid_announcement_params"));
    }

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_announcements (server_id, scope, department_id, author_id, title, content, priority, pinned) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
    )
    .bind(server_id)
    .bind(scope)
    .bind(p.department_id)
    .bind(uid)
    .bind(&title)
    .bind(&content)
    .bind(priority)
    .bind(p.pinned.unwrap_or(false))
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_announcement_posted", &title).await;
    emit_enterprise(&state, server_id, "announcements.updated", json!({ "id": id })).await;

    let state_bg = state.clone();
    let t_copy = title.clone();
    tokio::spawn(async move {
        trigger_workflows(&state_bg, server_id, "announcement_posted", json!({ "id": id, "title": t_copy }), 0).await;
    });
    Ok(Json(json!({ "id": id })))
}

pub async fn mark_announcement_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, ann_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let _ctx = load_ctx(&state.db, server_id, uid).await?;
    sqlx::query("INSERT INTO enterprise_announcement_reads (announcement_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(ann_id)
        .bind(uid)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "read": true })))
}

// ---------------------------------------------------------------- Integrations & Workflows

#[derive(Serialize, sqlx::FromRow)]
pub struct IntegrationItem {
    pub id: Uuid,
    pub provider: String,
    pub name: String,
    pub status: String,
    pub endpoint_url: Option<String>,
    pub has_secret: bool,
    pub events: Vec<String>,
    pub last_tested_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct WorkflowItem {
    pub id: Uuid,
    pub name: String,
    pub trigger_event: String,
    pub action_type: String,
    pub action_config: Value,
    pub enabled: bool,
    pub run_count: i64,
    pub last_run_at: Option<DateTime<Utc>>,
}

pub async fn list_integrations_and_workflows(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    if !ctx.has(ent_perm::MANAGE_INTEGRATIONS) && !ctx.has(ent_perm::MANAGE_AUTOMATION) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    let integrations: Vec<IntegrationItem> = sqlx::query_as(
        "SELECT id, provider, name, status, endpoint_url, has_secret, events, last_tested_at FROM enterprise_integrations WHERE server_id = $1 ORDER BY created_at DESC",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let workflows: Vec<WorkflowItem> = sqlx::query_as(
        "SELECT id, name, trigger_event, action_type, action_config, enabled, run_count, last_run_at FROM enterprise_workflows WHERE server_id = $1 ORDER BY created_at DESC",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "integrations": integrations, "workflows": workflows })))
}

#[derive(Deserialize)]
pub struct CreateIntegrationReq {
    pub provider: String,
    pub name: String,
    pub endpoint_url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<String>>,
}

pub async fn create_integration(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateIntegrationReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_INTEGRATIONS)?;

    if !["webhook", "github", "gitlab", "jira", "google_calendar", "outlook", "google_drive", "onedrive", "rest_api", "crm", "hrm", "erp"].contains(&p.provider.as_str()) {
        return Err(AppError::BadRequest("invalid_provider"));
    }
    let name = clean_req(&p.name, 1, 80, "invalid_name")?;
    let ep = p.endpoint_url.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(u) = ep {
        if !safe_webhook_url(u) {
            return Err(AppError::BadRequest("invalid_endpoint_url"));
        }
    }
    let status = if ep.is_some() { "configured" } else { "unconfigured" };
    let sec = p.secret.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let has_sec = sec.is_some();
    let sec_hash = sec.map(|s| hex::encode(Sha256::digest(s.as_bytes())));

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_integrations (server_id, provider, name, status, endpoint_url, has_secret, secret_hash, events) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
    )
    .bind(server_id)
    .bind(&p.provider)
    .bind(&name)
    .bind(status)
    .bind(ep)
    .bind(has_sec)
    .bind(sec_hash)
    .bind(p.events.unwrap_or_default())
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_integration_added", &name).await;
    Ok(Json(json!({ "id": id, "status": status })))
}

pub async fn test_integration(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, integ_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_INTEGRATIONS)?;

    let row = sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT endpoint_url, provider FROM enterprise_integrations WHERE id = $1 AND server_id = $2",
    )
    .bind(integ_id)
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("integration_not_found"))?;

    let Some(url) = row.0 else {
        return Ok(Json(json!({ "ok": false, "status": "unconfigured" })));
    };
    if !safe_webhook_url(&url) {
        return Err(AppError::BadRequest("invalid_endpoint_url"));
    }

    let ok = match state
        .http_client
        .post(&url)
        .json(&json!({ "event": "chatw.enterprise.ping", "server_id": server_id, "provider": row.1 }))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };
    let st = if ok { "configured" } else { "error" };
    sqlx::query("UPDATE enterprise_integrations SET status = $3, last_tested_at = now() WHERE id = $1 AND server_id = $2")
        .bind(integ_id)
        .bind(server_id)
        .bind(st)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": ok, "status": st })))
}

#[derive(Deserialize)]
pub struct CreateWorkflowReq {
    pub name: String,
    pub trigger_event: String,
    pub action_type: String,
    pub action_config: Value,
}

pub async fn create_workflow(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateWorkflowReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_AUTOMATION)?;

    let name = clean_req(&p.name, 1, 80, "invalid_workflow_name")?;
    if !["task_assigned", "task_completed", "leave_decided", "approval_decided", "member_joined", "announcement_posted"].contains(&p.trigger_event.as_str()) {
        return Err(AppError::BadRequest("invalid_trigger_event"));
    }
    if !["post_channel_message", "notify_user", "webhook"].contains(&p.action_type.as_str()) {
        return Err(AppError::BadRequest("invalid_action_type"));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_workflows (server_id, name, trigger_event, action_type, action_config) VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(server_id)
    .bind(&name)
    .bind(&p.trigger_event)
    .bind(&p.action_type)
    .bind(p.action_config)
    .fetch_one(&state.db)
    .await?;
    audit(&state, server_id, uid, "ent_workflow_created", &name).await;
    Ok(Json(json!({ "id": id })))
}

pub async fn toggle_workflow(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, wf_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_AUTOMATION)?;
    let next: bool = sqlx::query_scalar(
        "UPDATE enterprise_workflows SET enabled = NOT enabled WHERE id = $1 AND server_id = $2 RETURNING enabled",
    )
    .bind(wf_id)
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("workflow_not_found"))?;
    Ok(Json(json!({ "enabled": next })))
}

// ---------------------------------------------------------------- Enterprise AI Assistant

#[derive(Deserialize)]
pub struct AiSummarizeReq {
    pub scope: String, // "overview" | "tasks" | "project"
    pub target_id: Option<Uuid>,
}

pub async fn ai_summarize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<AiSummarizeReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let ai_cfg: Option<Value> = sqlx::query_scalar(
        "SELECT ai_config FROM server_enterprise_configs WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    let enabled = ai_cfg.as_ref().and_then(|v| v.get("enabled").and_then(|b| b.as_bool())).unwrap_or(false);
    if !enabled {
        return Err(AppError::BadRequest("ai_module_disabled"));
    }
    if !ctx.has(ent_perm::VIEW_REPORTS) && !ctx.has(ent_perm::MANAGE_AI) {
        return Err(AppError::Forbidden("insufficient_ai_permissions"));
    }

    let provider = ai_cfg
        .as_ref()
        .and_then(|v| v.get("provider").and_then(|x| x.as_str()))
        .unwrap_or("local");

    let open_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1 AND status NOT IN ('done','archived') AND ($2::uuid IS NULL OR project_id = $2)",
    )
    .bind(server_id)
    .bind(p.target_id)
    .fetch_one(&state.db)
    .await?;
    let done_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1 AND status = 'done' AND ($2::uuid IS NULL OR project_id = $2)",
    )
    .bind(server_id)
    .bind(p.target_id)
    .fetch_one(&state.db)
    .await?;
    let pending_approvals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_approval_requests WHERE server_id = $1 AND status = 'pending'",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_ai_summary", &p.scope).await;
    Ok(Json(json!({
        "provider": provider,
        "e2ee_note": "Only aggregate business data on authorized servers; Private E2EE messages are not decrypted.",
        "summary": format!("Summary ({provider}): There are currently {open_tasks} open jobs, {done_tasks} completed jobs and {pending_approvals} pending applications.")
    })))
}
