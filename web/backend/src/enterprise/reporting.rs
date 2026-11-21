//! Enterprise Reporting, CSV/JSON Data Export, Audit Log, Document Folders & Workflow Automation Runner.
//! Extracted from `enterprise/mod.rs` as a dedicated domain submodule (Spec 051).

use std::future::Future;
use std::pin::Pin;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::AppState,
    community::audit,
    enterprise::{
        hr,
        models::{self, clean_req, csv_escape, emit_enterprise, ent_perm, load_ctx},
        ops, work,
    },
    error::{AppError, AppResult},
    models::ChannelMessageItem,
    privacy::auth_user,
    websocket::WsEvent,
};

// ---------------------------------------------------------------- Reports & CSV

#[derive(Deserialize)]
pub struct ReportQuery {
    pub department_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn enterprise_reports(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ReportQuery>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    if !ctx.has(ent_perm::VIEW_REPORTS) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_report_permissions"));
    }

    let today_str = Utc::now().format("%Y-%m-%d").to_string();

    let task_by_status = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT status, COUNT(*)::bigint
        FROM enterprise_tasks
        WHERE server_id = $1 AND ($2::uuid IS NULL OR department_id = $2)
        GROUP BY status
        "#,
    )
    .bind(server_id)
    .bind(q.department_id)
    .fetch_all(&state.db)
    .await?;

    let overdue_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1 AND ($2::uuid IS NULL OR department_id = $2) AND due_date IS NOT NULL AND due_date < $3 AND status NOT IN ('done', 'archived')",
    )
    .bind(server_id)
    .bind(q.department_id)
    .bind(&today_str)
    .fetch_one(&state.db)
    .await?;

    let projects_by_status = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*)::bigint FROM enterprise_projects WHERE server_id = $1 AND ($2::uuid IS NULL OR department_id = $2) GROUP BY status",
    )
    .bind(server_id)
    .bind(q.department_id)
    .fetch_all(&state.db)
    .await?;

    let attendance_summary = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*)::bigint FROM enterprise_attendance WHERE server_id = $1 GROUP BY status",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let leave_summary = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*)::bigint FROM enterprise_leave_requests WHERE server_id = $1 GROUP BY status",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let approval_summary = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*)::bigint FROM enterprise_approval_requests WHERE server_id = $1 AND ($2::uuid IS NULL OR department_id = $2) GROUP BY status",
    )
    .bind(server_id)
    .bind(q.department_id)
    .fetch_all(&state.db)
    .await?;

    let total_doc_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(file_size), 0)::bigint FROM enterprise_documents WHERE server_id = $1 AND trashed_at IS NULL",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    let active_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_employees WHERE server_id = $1 AND employment_status = 'active' AND ($2::uuid IS NULL OR department_id = $2)",
    )
    .bind(server_id)
    .bind(q.department_id)
    .fetch_one(&state.db)
    .await?;

    let dept_workload = sqlx::query_as::<_, (Uuid, String, i64, i64)>(
        r#"
        SELECT d.id, d.name,
               (SELECT COUNT(*) FROM enterprise_employees e WHERE e.server_id = $1 AND e.department_id = d.id AND e.employment_status = 'active') AS emp_count,
               (SELECT COUNT(*) FROM enterprise_tasks t WHERE t.server_id = $1 AND t.department_id = d.id AND t.status NOT IN ('done', 'archived')) AS open_tasks
        FROM enterprise_departments d
        WHERE d.server_id = $1
        ORDER BY d.name ASC
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "tasks_by_status": task_by_status,
        "overdue_tasks": overdue_tasks,
        "projects_by_status": projects_by_status,
        "attendance_summary": attendance_summary,
        "leave_summary": leave_summary,
        "approval_summary": approval_summary,
        "total_doc_bytes": total_doc_bytes,
        "active_members": active_members,
        "department_workload": dept_workload.into_iter().map(|(id, name, emp, tasks)| json!({
            "id": id,
            "name": name,
            "employees": emp,
            "open_tasks": tasks,
        })).collect::<Vec<_>>(),
    })))
}

pub async fn export_report_csv(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    if !ctx.has(ent_perm::EXPORT_DATA) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_export_permissions"));
    }

    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
        r#"
        SELECT t.title, t.status, t.priority, u.username, t.due_date
        FROM enterprise_tasks t
        LEFT JOIN users u ON u.id = t.assignee_id
        WHERE t.server_id = $1
        ORDER BY t.created_at DESC
        LIMIT 2000
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let mut csv = String::from("TieuDe,TrangThai,UuTien,NguoiPhuTrach,HanHoanThanh\n");
    for (title, st, prio, assignee, due) in rows {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_escape(&title),
            csv_escape(&st),
            csv_escape(&prio),
            csv_escape(assignee.as_deref().unwrap_or("")),
            csv_escape(due.as_deref().unwrap_or("")),
        ));
    }

    audit(&state, server_id, uid, "ent_report_exported", "csv").await;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"enterprise-report.csv\""),
        ],
        csv,
    ))
}

// ---------------------------------------------------------------- Audit & Export All

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
    pub q: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EnterpriseAuditItem {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn list_audit_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(q): Query<AuditQuery>,
) -> AppResult<Json<Vec<EnterpriseAuditItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    if !ctx.has(ent_perm::VIEW_AUDIT) && !ctx.has(ent_perm::MANAGE_SECURITY) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_audit_permissions"));
    }

    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let term = q.q.as_deref().map(str::trim).unwrap_or("").to_lowercase();

    let rows: Vec<EnterpriseAuditItem> = sqlx::query_as(
        r#"
        SELECT l.id, l.actor_id, u.username AS actor_name, l.action, l.target, l.created_at
        FROM server_audit_log l
        LEFT JOIN users u ON u.id = l.actor_id
        WHERE l.server_id = $1
          AND ($2 = '' OR lower(l.action) LIKE '%' || $2 || '%' OR lower(COALESCE(l.target,'')) LIKE '%' || $2 || '%')
        ORDER BY l.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(server_id)
    .bind(term)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

pub async fn export_audit_csv(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    if !ctx.has(ent_perm::EXPORT_DATA) && !ctx.has(ent_perm::VIEW_AUDIT) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_export_permissions"));
    }

    let rows: Vec<EnterpriseAuditItem> = sqlx::query_as(
        r#"
        SELECT l.id, l.actor_id, u.username AS actor_name, l.action, l.target, l.created_at
        FROM server_audit_log l
        LEFT JOIN users u ON u.id = l.actor_id
        WHERE l.server_id = $1
        ORDER BY l.created_at DESC
        LIMIT 2000
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let mut csv = String::from("ThoiGian,NguoiThucHien,HanhDong,DoiTuong\n");
    for r in rows {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            csv_escape(&r.created_at.to_rfc3339()),
            csv_escape(r.actor_name.as_deref().unwrap_or("system")),
            csv_escape(&r.action),
            csv_escape(r.target.as_deref().unwrap_or("")),
        ));
    }

    audit(&state, server_id, uid, "ent_audit_exported", "csv").await;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"enterprise-audit.csv\""),
        ],
        csv,
    ))
}

pub async fn export_all_json(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let uid = auth_user(&state, &headers).await?;
    let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("server_not_found"))?;

    if owner_id != uid {
        let ctx = load_ctx(&state.db, server_id, uid).await?;
        if !ctx.has(ent_perm::EXPORT_DATA) && !ctx.is_admin() {
            return Err(AppError::Forbidden("insufficient_export_permissions"));
        }
    }

    let employees: Vec<hr::EmployeeItem> = sqlx::query_as(
        r#"
        SELECT e.*, u.username, u.avatar_url, u.presence, d.name AS department_name
        FROM enterprise_employees e
        JOIN users u ON u.id = e.user_id
        LEFT JOIN enterprise_departments d ON d.id = e.department_id
        WHERE e.server_id = $1
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let departments: Vec<hr::DepartmentItem> = sqlx::query_as(
        r#"
        SELECT d.id, d.server_id, d.parent_id, d.name, d.code, d.description,
               d.manager_user_id, u.username AS manager_name,
               d.linked_channel_id, c.name AS linked_channel_name, d.branch_name,
               (SELECT COUNT(*) FROM enterprise_department_members dm WHERE dm.department_id = d.id) AS member_count,
               d.created_at
        FROM enterprise_departments d
        LEFT JOIN users u ON u.id = d.manager_user_id
        LEFT JOIN channels c ON c.id = d.linked_channel_id
        WHERE d.server_id = $1
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let tasks: Vec<work::TaskItem> = sqlx::query_as(
        r#"
        SELECT t.id, t.server_id, t.project_id, p.name AS project_name,
               t.department_id, t.parent_task_id, t.title, t.description,
               t.status, t.priority, t.assignee_id, ua.username AS assignee_name,
               t.participant_ids, t.watcher_ids, t.labels, t.checklist, t.depends_on_ids,
               t.due_date, t.recurrence, t.source_channel_id, t.source_message_id,
               c.name AS source_channel_name, NULL::text AS source_message_preview,
               t.created_by,
               (SELECT COUNT(*) FROM enterprise_task_comments tc WHERE tc.task_id = t.id) AS comment_count,
               t.completed_at, t.created_at, t.updated_at
        FROM enterprise_tasks t
        LEFT JOIN enterprise_projects p ON p.id = t.project_id
        LEFT JOIN users ua ON ua.id = t.assignee_id
        LEFT JOIN channels c ON c.id = t.source_channel_id
        WHERE t.server_id = $1
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let docs: Vec<ops::DocumentItem> = sqlx::query_as(
        r#"
        SELECT d.id, d.server_id, d.folder_id, d.scope, d.department_id, d.project_id,
               d.owner_id, u.username AS owner_name, d.title, d.file_url, d.file_name,
               d.file_size, d.mime_type, d.pinned, d.version, d.trashed_at, d.created_at, d.updated_at
        FROM enterprise_documents d
        JOIN users u ON u.id = d.owner_id
        WHERE d.server_id = $1
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_full_exported", "json").await;
    let bundle = json!({
        "server_id": server_id,
        "exported_at": Utc::now(),
        "employees": employees,
        "departments": departments,
        "tasks": tasks,
        "documents": docs,
    });
    let body = serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| "{}".to_string());
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"enterprise-export.json\""),
        ],
        body,
    ))
}

// ---------------------------------------------------------------- Document Folders

#[derive(Deserialize)]
pub struct CreateFolderReq {
    pub name: String,
    pub scope: Option<String>,
    pub parent_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}

pub async fn create_folder(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateFolderReq>,
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
    let name = clean_req(&p.name, 1, 80, "invalid_folder_name")?;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_folders (server_id, parent_id, scope, department_id, project_id, owner_id, name) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(server_id)
    .bind(p.parent_id)
    .bind(scope)
    .bind(p.department_id)
    .bind(p.project_id)
    .bind(uid)
    .bind(&name)
    .fetch_one(&state.db)
    .await?;

    emit_enterprise(&state, server_id, "documents.updated", json!({ "folder_id": id })).await;
    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------- Workflow Automation Runner

pub fn trigger_workflows<'a>(
    state: &'a AppState,
    server_id: Uuid,
    event: &'a str,
    payload: Value,
    depth: u8,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if depth >= 2 {
            return;
        }
        let Ok(rows) = sqlx::query_as::<_, (Uuid, String, Value)>(
            "SELECT id, action_type, action_config FROM enterprise_workflows WHERE server_id = $1 AND trigger_event = $2 AND enabled = TRUE",
        )
        .bind(server_id)
        .bind(event)
        .fetch_all(&state.db)
        .await
        else {
            return;
        };

        let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM servers WHERE id = $1")
            .bind(server_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();

        for (wf_id, action_type, cfg) in rows {
            let mut status = "ok";
            let mut detail = String::new();

            match action_type.as_str() {
                "post_channel_message" => {
                    if let (Some(ch_s), Some(sender)) = (cfg.get("channel_id").and_then(|v| v.as_str()), owner_id) {
                        if let Ok(channel_id) = Uuid::parse_str(ch_s) {
                            let valid_ch: bool = sqlx::query_scalar(
                                "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND server_id = $2)",
                            )
                            .bind(channel_id)
                            .bind(server_id)
                            .fetch_one(&state.db)
                            .await
                            .unwrap_or(false);

                            if valid_ch {
                                let template = cfg
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Cập nhật tự động từ không gian doanh nghiệp");
                                let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
                                let content = if title.is_empty() {
                                    format!("[Tự động hóa · {}] {}", event, template)
                                } else {
                                    format!("[Tự động hóa · {}] {}: {}", event, template, title)
                                };
                                let msg_id = Uuid::new_v4();
                                if let Ok(r) = sqlx::query_as::<_, (i64, DateTime<Utc>, bool, bool)>(
                                    "INSERT INTO channel_messages (id, channel_id, sender_id, content) VALUES ($1, $2, $3, $4) RETURNING seq, created_at, is_pinned, is_edited",
                                )
                                .bind(msg_id)
                                .bind(channel_id)
                                .bind(sender)
                                .bind(&content)
                                .fetch_one(&state.db)
                                .await
                                {
                                    if let Ok(ids) = sqlx::query_scalar::<_, Uuid>(
                                        "SELECT user_id FROM server_members WHERE server_id = $1",
                                    )
                                    .bind(server_id)
                                    .fetch_all(&state.db)
                                    .await
                                    {
                                        let item = ChannelMessageItem {
                                            id: msg_id,
                                            seq: r.0,
                                            channel_id,
                                            sender_id: sender,
                                            sender_name: "Tự động hóa".to_string(),
                                            sender_avatar_url: None,
                                            content,
                                            reply_to_id: None,
                                            is_pinned: r.2,
                                            is_edited: r.3,
                                            created_at: r.1,
                                        };
                                        state.ws_hub.send_to_users(&ids, WsEvent::ChannelMessageNew { message: item }).await;
                                    }
                                    detail = format!("posted to {}", channel_id);
                                } else {
                                    status = "error";
                                    detail = "db insert failed".to_string();
                                }
                            } else {
                                status = "skipped";
                                detail = "channel not in server".to_string();
                            }
                        } else {
                            status = "skipped";
                            detail = "invalid channel_id".to_string();
                        }
                    } else {
                        status = "skipped";
                        detail = "missing channel_id".to_string();
                    }
                }
                "notify_user" => {
                    if let Some(uid_s) = cfg.get("user_id").and_then(|v| v.as_str()) {
                        if let Ok(target_uid) = Uuid::parse_str(uid_s) {
                            state
                                .ws_hub
                                .send_to_user(
                                    &target_uid,
                                    WsEvent::EnterpriseEvent(json!({
                                        "server_id": server_id,
                                        "kind": format!("workflow.{}", event),
                                        "payload": payload,
                                        "at": Utc::now(),
                                    })),
                                )
                                .await;
                            detail = format!("notified {}", target_uid);
                        } else {
                            status = "skipped";
                            detail = "invalid user_id".to_string();
                        }
                    } else {
                        status = "skipped";
                        detail = "missing user_id".to_string();
                    }
                }
                "webhook" => {
                    if let Some(url) = cfg.get("url").and_then(|v| v.as_str()) {
                        if models::safe_webhook_url(url) {
                            let client = reqwest::Client::builder()
                                .redirect(reqwest::redirect::Policy::none())
                                .timeout(std::time::Duration::from_secs(4))
                                .build();
                            if let Ok(cli) = client {
                                let res = cli
                                    .post(url)
                                    .json(&json!({ "event": event, "server_id": server_id, "payload": payload }))
                                    .send()
                                    .await;
                                match res {
                                    Ok(r) if r.status().is_success() => detail = format!("http {}", r.status()),
                                    Ok(r) => {
                                        status = "error";
                                        detail = format!("http {}", r.status());
                                    }
                                    Err(e) => {
                                        status = "error";
                                        detail = e.to_string();
                                    }
                                }
                            }
                        } else {
                            status = "skipped";
                            detail = "unsafe webhook url".to_string();
                        }
                    } else {
                        status = "skipped";
                        detail = "missing url".to_string();
                    }
                }
                _ => {
                    status = "skipped";
                    detail = "unknown action".to_string();
                }
            }

            let _ = sqlx::query("UPDATE enterprise_workflows SET run_count = run_count + 1, last_run_at = now() WHERE id = $1")
                .bind(wf_id)
                .execute(&state.db)
                .await;
            let _ = sqlx::query(
                "INSERT INTO enterprise_workflow_runs (workflow_id, server_id, status, detail) VALUES ($1, $2, $3, $4)",
            )
            .bind(wf_id)
            .bind(server_id)
            .bind(status)
            .bind(&detail)
            .execute(&state.db)
            .await;
        }
    })
}
