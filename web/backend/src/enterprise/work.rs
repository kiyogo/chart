use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::AppState,
    community::audit,
    enterprise::{
        models::{clean_opt, clean_req, emit_enterprise, ent_perm, load_ctx, valid_ymd},
        trigger_workflows,
    },
    error::{AppError, AppResult},
    models::permissions as base_perm,
    privacy::auth_user,
    servers::permissions::has_server_permission,
};

#[derive(Serialize, sqlx::FromRow)]
pub struct ProjectItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub department_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub manager_user_id: Option<Uuid>,
    pub manager_name: Option<String>,
    pub linked_channel_id: Option<Uuid>,
    pub linked_channel_name: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub milestones: Value,
    pub member_ids: Vec<Uuid>,
    pub task_total: i64,
    pub task_done: i64,
    pub created_at: DateTime<Utc>,
}

pub async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProjectItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let is_all = ctx.is_admin() || ctx.has(ent_perm::MANAGE_PROJECTS) || ctx.has(ent_perm::VIEW_REPORTS);
    let rows: Vec<ProjectItem> = sqlx::query_as(
        r#"
        SELECT p.id, p.server_id, p.department_id, p.name, p.description,
               p.status, p.priority, p.manager_user_id, u.username AS manager_name,
               p.linked_channel_id, c.name AS linked_channel_name,
               p.start_date, p.due_date, p.milestones, p.member_ids,
               (SELECT COUNT(*) FROM enterprise_tasks t WHERE t.project_id = p.id AND t.status <> 'archived') AS task_total,
               (SELECT COUNT(*) FROM enterprise_tasks t WHERE t.project_id = p.id AND t.status = 'done') AS task_done,
               p.created_at
        FROM enterprise_projects p
        LEFT JOIN users u ON u.id = p.manager_user_id
        LEFT JOIN channels c ON c.id = p.linked_channel_id
        WHERE p.server_id = $1
          AND ($2::bool OR p.manager_user_id = $3 OR p.created_by = $3 OR $3 = ANY(p.member_ids)
               OR (p.department_id IS NOT NULL AND p.department_id = ANY($4)))
        ORDER BY p.created_at DESC
        "#,
    )
    .bind(server_id)
    .bind(is_all)
    .bind(uid)
    .bind(&ctx.dept_ids)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateProjectReq {
    pub name: String,
    pub description: Option<String>,
    pub department_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub manager_user_id: Option<Uuid>,
    pub linked_channel_id: Option<Uuid>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub milestones: Option<Value>,
    pub member_ids: Option<Vec<Uuid>>,
}

pub async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateProjectReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_PROJECTS)?;

    let name = clean_req(&p.name, 1, 120, "invalid_project_name")?;
    let desc = clean_opt(p.description.as_deref(), 2000, "invalid_description")?;
    let status = p.status.as_deref().unwrap_or("active");
    if !["planning", "active", "on_hold", "completed", "archived"].contains(&status) {
        return Err(AppError::BadRequest("invalid_project_status"));
    }
    let priority = p.priority.as_deref().unwrap_or("medium");
    if !["low", "medium", "high", "urgent"].contains(&priority) {
        return Err(AppError::BadRequest("invalid_priority"));
    }
    if let Some(ch_id) = p.linked_channel_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND server_id = $2)",
        )
        .bind(ch_id)
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_channel"));
        }
    }

    if let Some(dept_id) = p.department_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM enterprise_departments WHERE id = $1 AND server_id = $2)",
        )
        .bind(dept_id)
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_department"));
        }
    }
    if let Some(mgr_id) = p.manager_user_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2)",
        )
        .bind(server_id)
        .bind(mgr_id)
        .fetch_one(&state.db)
        .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_manager"));
        }
    }
    if let Some(ref sd) = p.start_date {
        if !sd.trim().is_empty() && !valid_ymd(sd) {
            return Err(AppError::BadRequest("invalid_start_date"));
        }
    }
    if let Some(ref dd) = p.due_date {
        if !dd.trim().is_empty() && !valid_ymd(dd) {
            return Err(AppError::BadRequest("invalid_due_date"));
        }
    }

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO enterprise_projects (
            server_id, department_id, name, description, status, priority,
            manager_user_id, linked_channel_id, start_date, due_date,
            milestones, member_ids, created_by
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
        RETURNING id
        "#,
    )
    .bind(server_id)
    .bind(p.department_id)
    .bind(&name)
    .bind(desc)
    .bind(status)
    .bind(priority)
    .bind(p.manager_user_id.or(Some(uid)))
    .bind(p.linked_channel_id)
    .bind(p.start_date.filter(|s| !s.trim().is_empty()))
    .bind(p.due_date.filter(|s| !s.trim().is_empty()))
    .bind(p.milestones.unwrap_or_else(|| json!([])))
    .bind(p.member_ids.unwrap_or_else(|| vec![uid]))
    .bind(uid)
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_project_created", &name).await;
    emit_enterprise(&state, server_id, "projects.updated", json!({ "id": id })).await;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct UpdateProjectReq {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub milestones: Option<Value>,
}

pub async fn update_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, proj_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<UpdateProjectReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_PROJECTS)?;
    if let Some(ref st) = p.status {
        if !["planning", "active", "on_hold", "completed", "archived"].contains(&st.as_str()) {
            return Err(AppError::BadRequest("invalid_project_status"));
        }
    }
    if let Some(ref pr) = p.priority {
        if !["low", "medium", "high", "urgent"].contains(&pr.as_str()) {
            return Err(AppError::BadRequest("invalid_priority"));
        }
    }
    let res = sqlx::query(
        r#"
        UPDATE enterprise_projects
        SET status = COALESCE($3, status),
            priority = COALESCE($4, priority),
            milestones = COALESCE($5, milestones)
        WHERE id = $1 AND server_id = $2
        "#,
    )
    .bind(proj_id)
    .bind(server_id)
    .bind(p.status.as_deref())
    .bind(p.priority.as_deref())
    .bind(p.milestones)
    .execute(&state.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("project_not_found"));
    }
    emit_enterprise(&state, server_id, "projects.updated", json!({ "id": proj_id })).await;
    Ok(Json(json!({ "updated": true })))
}

// ---------------------------------------------------------------- Tasks

#[derive(Deserialize)]
pub struct TaskQuery {
    pub view: Option<String>, // "all" | "mine" | "assigned" | "overdue"
    pub status: Option<String>,
    pub project_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TaskItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub project_id: Option<Uuid>,
    pub project_name: Option<String>,
    pub department_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee_id: Option<Uuid>,
    pub assignee_name: Option<String>,
    pub participant_ids: Vec<Uuid>,
    pub watcher_ids: Vec<Uuid>,
    pub labels: Vec<String>,
    pub checklist: Value,
    pub depends_on_ids: Vec<Uuid>,
    pub due_date: Option<String>,
    pub recurrence: String,
    pub source_channel_id: Option<Uuid>,
    pub source_message_id: Option<Uuid>,
    pub source_channel_name: Option<String>,
    pub source_message_preview: Option<String>,
    pub created_by: Option<Uuid>,
    pub comment_count: i64,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(q): Query<TaskQuery>,
) -> AppResult<Json<Vec<TaskItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let is_all = ctx.is_admin() || ctx.has(ent_perm::ASSIGN_TASKS) || ctx.has(ent_perm::APPROVE_TASKS) || ctx.has(ent_perm::VIEW_REPORTS);
    let view = q.view.as_deref().unwrap_or("all");
    let today_str = Utc::now().format("%Y-%m-%d").to_string();
    let search = q.q.as_deref().map(str::trim).unwrap_or("").to_lowercase();
    let limit = q.limit.unwrap_or(150).clamp(1, 500);

    let mut rows: Vec<TaskItem> = sqlx::query_as(
        r#"
        SELECT t.id, t.server_id, t.project_id, p.name AS project_name,
               t.department_id, t.parent_task_id, t.title, t.description,
               t.status, t.priority, t.assignee_id, ua.username AS assignee_name,
               t.participant_ids, t.watcher_ids, t.labels, t.checklist, t.depends_on_ids,
               t.due_date, t.recurrence, t.source_channel_id, t.source_message_id,
               c.name AS source_channel_name,
               CASE WHEN c.is_private IS FALSE OR c.is_private IS NULL THEN left(cm.content, 160) ELSE NULL END AS source_message_preview,
               t.created_by,
               (SELECT COUNT(*) FROM enterprise_task_comments tc WHERE tc.task_id = t.id) AS comment_count,
               t.completed_at, t.created_at, t.updated_at
        FROM enterprise_tasks t
        LEFT JOIN enterprise_projects p ON p.id = t.project_id
        LEFT JOIN users ua ON ua.id = t.assignee_id
        LEFT JOIN channels c ON c.id = t.source_channel_id
        LEFT JOIN channel_messages cm ON cm.id = t.source_message_id
        WHERE t.server_id = $1
          AND ($2::bool OR t.assignee_id = $3 OR t.created_by = $3 OR $3 = ANY(t.participant_ids) OR $3 = ANY(t.watcher_ids)
               OR (t.department_id IS NOT NULL AND t.department_id = ANY($4)))
          AND ($5 = 'all'
               OR ($5 = 'mine' AND (t.assignee_id = $3 OR $3 = ANY(t.participant_ids)))
               OR ($5 = 'assigned' AND t.created_by = $3)
               OR ($5 = 'overdue' AND t.due_date IS NOT NULL AND t.due_date < $6 AND t.status NOT IN ('done', 'archived')))
          AND ($7::text IS NULL OR t.status = $7)
          AND ($8::uuid IS NULL OR t.project_id = $8)
          AND ($9::uuid IS NULL OR t.department_id = $9)
          AND ($10 = '' OR lower(t.title) LIKE '%' || $10 || '%' OR lower(COALESCE(t.description,'')) LIKE '%' || $10 || '%')
        ORDER BY
            CASE t.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END,
            t.created_at DESC
        LIMIT $11
        "#,
    )
    .bind(server_id)
    .bind(is_all)
    .bind(uid)
    .bind(&ctx.dept_ids)
    .bind(view)
    .bind(&today_str)
    .bind(q.status.as_deref().filter(|s| !s.is_empty()))
    .bind(q.project_id)
    .bind(q.department_id)
    .bind(search)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let can_view_channels = has_server_permission(&state.db, server_id, uid, base_perm::VIEW_CHANNELS)
        .await
        .unwrap_or(false);
    if !can_view_channels {
        for r in &mut rows {
            r.source_message_preview = None;
            r.source_channel_name = None;
        }
    }

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateTaskReq {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub participant_ids: Option<Vec<Uuid>>,
    pub watcher_ids: Option<Vec<Uuid>>,
    pub labels: Option<Vec<String>>,
    pub checklist: Option<Value>,
    pub due_date: Option<String>,
    pub recurrence: Option<String>,
    pub source_channel_id: Option<Uuid>,
    pub source_message_id: Option<Uuid>,
}

pub async fn create_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateTaskReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::CREATE_TASKS)?;

    if let Some(assignee) = p.assignee_id {
        if assignee != uid {
            ctx.require(ent_perm::ASSIGN_TASKS)?;
        }
    }

    let title = clean_req(&p.title, 1, 160, "invalid_task_title")?;
    let desc = clean_opt(p.description.as_deref(), 4000, "invalid_description")?;
    let status = p.status.as_deref().unwrap_or("todo");
    if !["todo", "in_progress", "review", "done", "archived"].contains(&status) {
        return Err(AppError::BadRequest("invalid_task_status"));
    }
    let priority = p.priority.as_deref().unwrap_or("medium");
    if !["low", "medium", "high", "urgent"].contains(&priority) {
        return Err(AppError::BadRequest("invalid_priority"));
    }
    let rec = p.recurrence.as_deref().unwrap_or("none");
    if !["none", "daily", "weekly", "monthly"].contains(&rec) {
        return Err(AppError::BadRequest("invalid_recurrence"));
    }
    if let Some(ref dd) = p.due_date {
        if !dd.trim().is_empty() && !valid_ymd(dd) {
            return Err(AppError::BadRequest("invalid_due_date"));
        }
    }

    if let Some(proj_id) = p.project_id {
        let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM enterprise_projects WHERE id = $1 AND server_id = $2)")
            .bind(proj_id)
            .bind(server_id)
            .fetch_one(&state.db)
            .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_project_id"));
        }
    }
    if let Some(dept_id) = p.department_id {
        let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM enterprise_departments WHERE id = $1 AND server_id = $2)")
            .bind(dept_id)
            .bind(server_id)
            .fetch_one(&state.db)
            .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_department_id"));
        }
    }
    if let Some(parent_id) = p.parent_task_id {
        let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM enterprise_tasks WHERE id = $1 AND server_id = $2)")
            .bind(parent_id)
            .bind(server_id)
            .fetch_one(&state.db)
            .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_parent_task_id"));
        }
    }
    if let Some(ch_id) = p.source_channel_id {
        let ch_srv: Option<Uuid> = sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
            .bind(ch_id)
            .fetch_optional(&state.db)
            .await?;
        if ch_srv != Some(server_id) {
            return Err(AppError::BadRequest("invalid_source_channel"));
        }
        crate::storage::permissions::authorize_channel_access(&state.db, uid, ch_id, base_perm::VIEW_CHANNELS).await?;
    }
    if let Some(msg_id) = p.source_message_id {
        let msg_ch: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT cm.channel_id, c.server_id, c.is_private FROM channel_messages cm JOIN channels c ON c.id = cm.channel_id WHERE cm.id = $1",
        )
        .bind(msg_id)
        .fetch_optional(&state.db)
        .await?;
        let Some((msg_ch_id, msg_srv_id, _is_priv)) = msg_ch else {
            return Err(AppError::BadRequest("invalid_source_message"));
        };
        if msg_srv_id != server_id || p.source_channel_id.is_some_and(|cid| cid != msg_ch_id) {
            return Err(AppError::BadRequest("invalid_source_message"));
        }
        crate::storage::permissions::authorize_channel_access(&state.db, uid, msg_ch_id, base_perm::VIEW_CHANNELS).await?;
    }

    let mut tx = state.db.begin().await?;
    let tid: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO enterprise_tasks (
            server_id, project_id, department_id, parent_task_id, title, description,
            status, priority, assignee_id, participant_ids, watcher_ids, labels,
            checklist, due_date, recurrence, source_channel_id, source_message_id, created_by
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
        RETURNING id
        "#,
    )
    .bind(server_id)
    .bind(p.project_id)
    .bind(p.department_id)
    .bind(p.parent_task_id)
    .bind(&title)
    .bind(desc)
    .bind(status)
    .bind(priority)
    .bind(p.assignee_id)
    .bind(p.participant_ids.unwrap_or_default())
    .bind(p.watcher_ids.unwrap_or_default())
    .bind(p.labels.unwrap_or_default())
    .bind(p.checklist.unwrap_or_else(|| json!([])))
    .bind(p.due_date.filter(|s| !s.trim().is_empty()))
    .bind(rec)
    .bind(p.source_channel_id)
    .bind(p.source_message_id)
    .bind(uid)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO enterprise_task_activity (task_id, actor_id, action, details) VALUES ($1, $2, 'created', $3)")
        .bind(tid)
        .bind(uid)
        .bind(&title)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(&state, server_id, uid, "ent_task_created", &title).await;
    emit_enterprise(&state, server_id, "tasks.updated", json!({ "id": tid, "assignee_id": p.assignee_id })).await;

    if p.assignee_id.is_some() {
        let state_bg = state.clone();
        let t_copy = title.clone();
        tokio::spawn(async move {
            trigger_workflows(&state_bg, server_id, "task_assigned", json!({ "task_id": tid, "title": t_copy, "assignee_id": p.assignee_id }), 0).await;
        });
    }
    Ok(Json(json!({ "id": tid })))
}

#[derive(Deserialize)]
pub struct UpdateTaskReq {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub due_date: Option<String>,
    pub checklist: Option<Value>,
}

pub async fn update_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, task_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<UpdateTaskReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let existing = sqlx::query_as::<_, (Option<Uuid>, Option<Uuid>, String)>(
        "SELECT assignee_id, created_by, status FROM enterprise_tasks WHERE id = $1 AND server_id = $2",
    )
    .bind(task_id)
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("task_not_found"))?;

    let is_involved = existing.0 == Some(uid) || existing.1 == Some(uid);
    if !is_involved && !ctx.has(ent_perm::ASSIGN_TASKS) && !ctx.has(ent_perm::APPROVE_TASKS) {
        return Err(AppError::Forbidden("insufficient_task_permissions"));
    }
    if p.assignee_id.is_some() && p.assignee_id != existing.0 && !ctx.has(ent_perm::ASSIGN_TASKS) {
        return Err(AppError::Forbidden("missing_assign_permission"));
    }
    if let Some(ref st) = p.status {
        if !["todo", "in_progress", "review", "done", "archived"].contains(&st.as_str()) {
            return Err(AppError::BadRequest("invalid_task_status"));
        }
    }
    if let Some(ref pr) = p.priority {
        if !["low", "medium", "high", "urgent"].contains(&pr.as_str()) {
            return Err(AppError::BadRequest("invalid_priority"));
        }
    }

    if let Some(ref dd) = p.due_date
        && !dd.trim().is_empty()
        && !valid_ymd(dd)
    {
        return Err(AppError::BadRequest("invalid_due_date"));
    }
    let next_done = p.status.as_deref() == Some("done");
    let clear_completed = p.status.as_deref().is_some_and(|s| s != "done");
    sqlx::query(
        r#"
        UPDATE enterprise_tasks
        SET title = COALESCE($3, title),
            description = COALESCE($4, description),
            status = COALESCE($5, status),
            priority = COALESCE($6, priority),
            assignee_id = COALESCE($7, assignee_id),
            due_date = COALESCE($8, due_date),
            checklist = COALESCE($9, checklist),
            completed_at = CASE WHEN $10 THEN COALESCE(completed_at, now()) WHEN $11 THEN NULL ELSE completed_at END,
            updated_at = now()
        WHERE id = $1 AND server_id = $2
        "#,
    )
    .bind(task_id)
    .bind(server_id)
    .bind(clean_opt(p.title.as_deref(), 160, "invalid_task_title")?)
    .bind(clean_opt(p.description.as_deref(), 4000, "invalid_description")?)
    .bind(p.status.as_deref())
    .bind(p.priority.as_deref())
    .bind(p.assignee_id)
    .bind(p.due_date.filter(|d| !d.trim().is_empty()))
    .bind(p.checklist)
    .bind(next_done)
    .bind(clear_completed)
    .execute(&state.db)
    .await?;

    let detail = p.status.clone().unwrap_or_else(|| "updated".into());
    sqlx::query("INSERT INTO enterprise_task_activity (task_id, actor_id, action, details) VALUES ($1, $2, 'updated', $3)")
        .bind(task_id)
        .bind(uid)
        .bind(&detail)
        .execute(&state.db)
        .await?;

    emit_enterprise(&state, server_id, "tasks.updated", json!({ "id": task_id, "status": detail })).await;
    if next_done && existing.2 != "done" {
        let state_bg = state.clone();
        tokio::spawn(async move {
            trigger_workflows(&state_bg, server_id, "task_completed", json!({ "task_id": task_id }), 0).await;
        });
    }
    Ok(Json(json!({ "updated": true })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TaskCommentItem {
    pub id: Uuid,
    pub task_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub async fn list_task_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, task_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Vec<TaskCommentItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    let is_all = ctx.is_admin() || ctx.has(ent_perm::ASSIGN_TASKS) || ctx.has(ent_perm::APPROVE_TASKS) || ctx.has(ent_perm::VIEW_REPORTS);
    let can_see: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM enterprise_tasks t WHERE t.id = $1 AND t.server_id = $2 AND ($3::bool OR t.assignee_id = $4 OR t.created_by = $4 OR $4 = ANY(t.participant_ids) OR $4 = ANY(t.watcher_ids) OR (t.department_id IS NOT NULL AND t.department_id = ANY($5))))",
    )
    .bind(task_id)
    .bind(server_id)
    .bind(is_all)
    .bind(uid)
    .bind(&ctx.dept_ids)
    .fetch_one(&state.db)
    .await?;
    if !can_see {
        return Err(AppError::Forbidden("task_access_denied"));
    }
    let rows: Vec<TaskCommentItem> = sqlx::query_as(
        r#"
        SELECT c.id, c.task_id, c.author_id, u.username AS author_name, c.content, c.created_at
        FROM enterprise_task_comments c
        JOIN users u ON u.id = c.author_id
        JOIN enterprise_tasks t ON t.id = c.task_id
        WHERE c.task_id = $1 AND t.server_id = $2
        ORDER BY c.created_at ASC
        "#,
    )
    .bind(task_id)
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct NewCommentReq {
    pub content: String,
}

pub async fn add_task_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, task_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<NewCommentReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    let is_all = ctx.is_admin() || ctx.has(ent_perm::ASSIGN_TASKS) || ctx.has(ent_perm::APPROVE_TASKS) || ctx.has(ent_perm::VIEW_REPORTS);
    let can_see: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM enterprise_tasks t WHERE t.id = $1 AND t.server_id = $2 AND ($3::bool OR t.assignee_id = $4 OR t.created_by = $4 OR $4 = ANY(t.participant_ids) OR $4 = ANY(t.watcher_ids) OR (t.department_id IS NOT NULL AND t.department_id = ANY($5))))",
    )
    .bind(task_id)
    .bind(server_id)
    .bind(is_all)
    .bind(uid)
    .bind(&ctx.dept_ids)
    .fetch_one(&state.db)
    .await?;
    if !can_see {
        return Err(AppError::Forbidden("task_access_denied"));
    }
    let content = clean_req(&p.content, 1, 2000, "invalid_comment")?;

    let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM enterprise_tasks WHERE id = $1 AND server_id = $2)")
        .bind(task_id)
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;
    if !ok {
        return Err(AppError::NotFound("task_not_found"));
    }
    let cid: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_task_comments (task_id, author_id, content) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(task_id)
    .bind(uid)
    .bind(&content)
    .fetch_one(&state.db)
    .await?;
    sqlx::query("INSERT INTO enterprise_task_activity (task_id, actor_id, action, details) VALUES ($1, $2, 'commented', 'new_comment')")
        .bind(task_id)
        .bind(uid)
        .execute(&state.db)
        .await?;
    emit_enterprise(&state, server_id, "tasks.updated", json!({ "id": task_id })).await;
    Ok(Json(json!({ "id": cid })))
}
