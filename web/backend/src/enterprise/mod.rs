pub mod hr;
pub mod models;
pub mod ops;
pub mod reporting;
pub mod time;
pub mod work;

pub use reporting::{
    AuditQuery, CreateFolderReq, EnterpriseAuditItem, ReportQuery, create_folder,
    enterprise_reports, export_all_json, export_audit_csv, export_report_csv, list_audit_log,
    trigger_workflows,
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, patch, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    auth::AppState,
    community::audit,
    error::{AppError, AppResult},
    models::permissions as base_perm,
    privacy::auth_user,
};
use models::{EnterpriseConfigRow, emit_enterprise, ent_perm, load_ctx};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/servers/{id}/enterprise/config", get(get_config).patch(update_config))
        .route("/servers/{id}/enterprise/overview", get(get_overview))
        .route("/servers/{id}/enterprise/search", get(enterprise_search))
        .route("/servers/{id}/enterprise/reports", get(enterprise_reports))
        .route("/servers/{id}/enterprise/reports/csv", get(export_report_csv))
        .route("/servers/{id}/enterprise/audit", get(list_audit_log))
        .route("/servers/{id}/enterprise/audit/csv", get(export_audit_csv))
        .route("/servers/{id}/enterprise/export-all", get(export_all_json))
        // HR & Departments
        .route("/servers/{id}/enterprise/employees", get(hr::list_employees).post(hr::upsert_employee))
        .route("/servers/{id}/enterprise/employees/bulk", post(hr::bulk_update_employees))
        .route("/servers/{id}/enterprise/employees/import", post(hr::import_employees))
        .route("/servers/{id}/enterprise/employees/{user_id}/handover", post(hr::handover_employee))
        .route("/servers/{id}/enterprise/departments", get(hr::list_departments).post(hr::create_department))
        .route("/servers/{id}/enterprise/departments/{dept_id}", delete(hr::delete_department))
        .route("/servers/{id}/enterprise/departments/{dept_id}/members", post(hr::add_department_member))
        // Projects & Tasks
        .route("/servers/{id}/enterprise/projects", get(work::list_projects).post(work::create_project))
        .route("/servers/{id}/enterprise/projects/{project_id}", patch(work::update_project))
        .route("/servers/{id}/enterprise/tasks", get(work::list_tasks).post(work::create_task))
        .route("/servers/{id}/enterprise/tasks/{task_id}", patch(work::update_task))
        .route("/servers/{id}/enterprise/tasks/{task_id}/comments", get(work::list_task_comments).post(work::add_task_comment))
        // Attendance, Leave & Meetings
        .route("/servers/{id}/enterprise/attendance", get(time::list_attendance))
        .route("/servers/{id}/enterprise/attendance/check-in", post(time::check_in))
        .route("/servers/{id}/enterprise/attendance/check-out", post(time::check_out))
        .route("/servers/{id}/enterprise/attendance/adjustments", get(time::list_adjustments).post(time::request_adjustment))
        .route("/servers/{id}/enterprise/attendance/adjustments/{adj_id}/decide", post(time::decide_adjustment))
        .route("/servers/{id}/enterprise/attendance/csv", get(time::export_attendance_csv))
        .route("/servers/{id}/enterprise/leave", get(time::list_leave).post(time::create_leave_request))
        .route("/servers/{id}/enterprise/leave/{req_id}/decide", post(time::decide_leave_request))
        .route("/servers/{id}/enterprise/meetings", get(time::list_meetings).post(time::create_meeting))
        .route("/servers/{id}/enterprise/meetings/{meeting_id}", patch(time::update_meeting))
        // Approvals, Documents, Announcements, Integrations, Workflows & AI
        .route("/servers/{id}/enterprise/approvals", get(ops::list_approvals).post(ops::submit_approval))
        .route("/servers/{id}/enterprise/approvals/templates", post(ops::create_form_template))
        .route("/servers/{id}/enterprise/approvals/{req_id}/decide", post(ops::decide_approval))
        .route("/servers/{id}/enterprise/documents", get(ops::list_documents).post(ops::create_document))
        .route("/servers/{id}/enterprise/documents/folders", post(create_folder))
        .route("/servers/{id}/enterprise/documents/{doc_id}", patch(ops::update_document))
        .route("/enterprise/shared/{token}", get(ops::resolve_shared_document))
        .route("/servers/{id}/enterprise/announcements", get(ops::list_announcements).post(ops::create_announcement))
        .route("/servers/{id}/enterprise/announcements/{ann_id}/read", post(ops::mark_announcement_read))
        .route("/servers/{id}/enterprise/integrations", get(ops::list_integrations_and_workflows).post(ops::create_integration))
        .route("/servers/{id}/enterprise/integrations/{integ_id}/test", post(ops::test_integration))
        .route("/servers/{id}/enterprise/workflows", post(ops::create_workflow))
        .route("/servers/{id}/enterprise/workflows/{wf_id}/toggle", post(ops::toggle_workflow))
        .route("/servers/{id}/enterprise/ai/summarize", post(ops::ai_summarize))
}

/// Initializes default enterprise metadata inside an existing server creation/conversion transaction.
/// Never alters or removes existing channels, messages, members, roles, or webhooks.
pub async fn init_enterprise_tx(
    tx: &mut Transaction<'_, Postgres>,
    server_id: Uuid,
    owner_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO server_enterprise_configs (server_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(server_id)
        .execute(&mut **tx)
        .await?;

    let owner_name: String = sqlx::query_scalar("SELECT COALESCE(username, 'Quản trị viên') FROM users WHERE id = $1")
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or_else(|| "Administrator".to_string());

    sqlx::query(
        r#"
        INSERT INTO enterprise_employees (server_id, user_id, member_kind, full_name, job_title, employment_status)
        VALUES ($1, $2, 'employee', $3, 'Quản trị viên hệ thống', 'active')
        ON CONFLICT (server_id, user_id) DO NOTHING
        "#,
    )
    .bind(server_id)
    .bind(owner_id)
    .bind(&owner_name)
    .execute(&mut **tx)
    .await?;

    let tpl_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_form_templates WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;

    if tpl_count == 0 {
        let templates = [
            (
                "Proposal for equipment purchase",
                "purchase",
                "Proposal to equip hardware, software or supplies",
                json!([
                    {"key": "item", "label": "Recommended categories", "required": true},
                    {"key": "budget", "label": "Cost estimate (VND)", "required": true},
                    {"key": "reason", "label": "Reason for use", "required": true}
                ]),
                json!(["Direct management", "Accounting/Operations"]),
            ),
            (
                "Offer to pay",
                "payment",
                "Advance payment, supplier invoice or per diem",
                json!([
                    {"key": "amount", "label": "Amount (VND)", "required": true},
                    {"key": "beneficiary", "label": "Beneficiary person/unit", "required": true},
                    {"key": "note", "label": "Interpretation of documents", "required": false}
                ]),
                json!(["Department management", "Financial approval"]),
            ),
            (
                "Request technical support and authorization",
                "it_support",
                "Grant accounts, decentralize system rights or handle technical problems",
                json!([
                    {"key": "system", "label": "System/Equipment", "required": true},
                    {"key": "urgency", "label": "Priority level", "required": true},
                    {"key": "detail", "label": "Detailed description", "required": true}
                ]),
                json!(["System administration"]),
            ),
        ];
        for (name, cat, desc, fields, steps) in templates {
            sqlx::query(
                r#"
                INSERT INTO enterprise_form_templates (server_id, name, category, description, fields, workflow_steps, created_by)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(server_id)
            .bind(name)
            .bind(cat)
            .bind(desc)
            .bind(fields)
            .bind(steps)
            .bind(owner_id)
            .execute(&mut **tx)
            .await?;
        }
    }

    let hr_bits = ent_perm::VIEW_HR
        | ent_perm::MANAGE_HR
        | ent_perm::VIEW_SENSITIVE_HR
        | ent_perm::MANAGE_DEPARTMENTS
        | ent_perm::VIEW_ATTENDANCE
        | ent_perm::MANAGE_ATTENDANCE
        | ent_perm::APPROVE_LEAVE
        | ent_perm::VIEW_REPORTS
        | ent_perm::EXPORT_DATA
        | ent_perm::POST_ANNOUNCEMENTS;
    let pm_bits = ent_perm::VIEW_HR
        | ent_perm::MANAGE_PROJECTS
        | ent_perm::CREATE_TASKS
        | ent_perm::ASSIGN_TASKS
        | ent_perm::APPROVE_TASKS
        | ent_perm::MANAGE_DOCUMENTS
        | ent_perm::APPROVE_REQUESTS
        | ent_perm::VIEW_REPORTS
        | ent_perm::MANAGE_MEETINGS
        | ent_perm::POST_ANNOUNCEMENTS;

    let default_roles = [
        ("Personnel management", "#10b981", 30i32, base_perm::DEFAULT_EVERYONE, hr_bits),
        ("Project management", "#3b82f6", 20i32, base_perm::DEFAULT_EVERYONE | base_perm::MANAGE_CHANNELS, pm_bits),
        ("Staff", "#64748b", 10i32, base_perm::DEFAULT_EVERYONE, ent_perm::DEFAULT_EMPLOYEE),
        ("Guest", "#94a3b8", 5i32, base_perm::VIEW_CHANNELS | base_perm::SEND_MESSAGES, ent_perm::VIEW_HR),
    ];

    for (rname, color, pos, b_perm, e_perm) in default_roles {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM server_roles WHERE server_id = $1 AND name = $2)",
        )
        .bind(server_id)
        .bind(rname)
        .fetch_one(&mut **tx)
        .await?;
        if !exists {
            sqlx::query(
                r#"
                INSERT INTO server_roles (id, server_id, name, color, position, permissions, enterprise_permissions)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(server_id)
            .bind(rname)
            .bind(color)
            .bind(pos)
            .bind(b_perm)
            .bind(e_perm)
            .execute(&mut **tx)
            .await?;
        }
    }

    // 5. Enterprise Categories & Channels
    let ent_cat_id: Uuid = match sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM server_categories WHERE server_id = $1 AND LOWER(name) = 'không gian doanh nghiệp' LIMIT 1",
    )
    .bind(server_id)
    .fetch_optional(&mut **tx)
    .await?
    {
        Some(id) => id,
        None => {
            let cid = Uuid::new_v4();
            sqlx::query("INSERT INTO server_categories (id, server_id, name, position) VALUES ($1, $2, 'Không gian doanh nghiệp', 10)")
                .bind(cid)
                .bind(server_id)
                .execute(&mut **tx)
                .await?;
            cid
        }
    };

    let meet_cat_id: Uuid = match sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM server_categories WHERE server_id = $1 AND LOWER(name) = 'phòng họp trực tuyến' LIMIT 1",
    )
    .bind(server_id)
    .fetch_optional(&mut **tx)
    .await?
    {
        Some(id) => id,
        None => {
            let cid = Uuid::new_v4();
            sqlx::query("INSERT INTO server_categories (id, server_id, name, position) VALUES ($1, $2, 'Phòng họp trực tuyến', 20)")
                .bind(cid)
                .bind(server_id)
                .execute(&mut **tx)
                .await?;
            cid
        }
    };

    let ent_channels = [
        ("Internal announcement", "announcement", ent_cat_id, 1i32),
        ("Executive Board", "text", ent_cat_id, 2i32),
        ("Administrative personnel", "text", ent_cat_id, 3i32),
        ("Product engineering", "text", ent_cat_id, 4i32),
        ("Operational business", "text", ent_cat_id, 5i32),
        ("Automation", "text", ent_cat_id, 6i32),
        ("Executive meeting room", "voice", meet_cat_id, 7i32),
        ("Quick meeting room", "voice", meet_cat_id, 8i32),
    ];

    let mut ch_map = std::collections::HashMap::new();
    for (cname, ctype, cat_id, pos) in ent_channels {
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM channels WHERE server_id = $1 AND name = $2 LIMIT 1",
        )
        .bind(server_id)
        .bind(cname)
        .fetch_optional(&mut **tx)
        .await?;
        let ch_id = match existing {
            Some(id) => id,
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO channels (id, server_id, category_id, name, channel_type, position, is_private) VALUES ($1, $2, $3, $4, $5, $6, FALSE)",
                )
                .bind(id)
                .bind(server_id)
                .bind(cat_id)
                .bind(cname)
                .bind(ctype)
                .bind(pos)
                .execute(&mut **tx)
                .await?;
                if cname == "Internal announcement" {
                    let welcome = "Welcome to Enterprise Space. Departmental channels, approval processes, timekeeping, job boards, and quick commands (/task, /checkin, /checkout, /leave, /meet, /approve, /announce, /workspace) have been enabled.";
                    sqlx::query(
                        "INSERT INTO channel_messages (id, channel_id, sender_id, content) VALUES ($1, $2, $3, $4)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(id)
                    .bind(owner_id)
                    .bind(welcome)
                    .execute(&mut **tx)
                    .await?;
                }
                id
            }
        };
        ch_map.insert(cname, ch_id);
    }

    // 6. Default Departments linked to channels
    let dept_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_departments WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;

    let mut primary_dept_id: Option<Uuid> = None;
    let mut eng_dept_id: Option<Uuid> = None;
    if dept_count == 0 {
        let depts = [
            ("Executive Board", "BOD", "Executive strategy and senior approval", "Head office", ch_map.get("Executive Board").copied()),
            ("Product technical department", "ENG", "Product development, technical infrastructure and security", "Head office", ch_map.get("Product engineering").copied()),
            ("Administrative personnel department", "HR", "Recruitment, personnel records, timekeeping and benefits", "Head office", ch_map.get("Administrative personnel").copied()),
            ("Operational sales department", "BIZ", "Business development, partners and customer care", "Branch", ch_map.get("Operational business").copied()),
        ];
        for (idx, (dname, dcode, ddesc, branch, lch)) in depts.iter().enumerate() {
            let did: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO enterprise_departments (server_id, name, code, description, manager_user_id, linked_channel_id, branch_name)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id
                "#,
            )
            .bind(server_id)
            .bind(dname)
            .bind(dcode)
            .bind(ddesc)
            .bind(owner_id)
            .bind(lch)
            .bind(branch)
            .fetch_one(&mut **tx)
            .await?;
            if idx == 0 {
                primary_dept_id = Some(did);
                sqlx::query(
                    "INSERT INTO enterprise_department_members (department_id, server_id, user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(did)
                .bind(server_id)
                .bind(owner_id)
                .execute(&mut **tx)
                .await?;
                sqlx::query(
                    "UPDATE enterprise_employees SET department_id = COALESCE(department_id, $3) WHERE server_id = $1 AND user_id = $2",
                )
                .bind(server_id)
                .bind(owner_id)
                .bind(did)
                .execute(&mut **tx)
                .await?;
            } else if idx == 1 {
                eng_dept_id = Some(did);
            }
        }
    }

    // 7. Default Projects & Starter Tasks
    let proj_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_projects WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;

    if proj_count == 0 {
        let p1_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO enterprise_projects (server_id, department_id, name, description, status, priority, manager_user_id, linked_channel_id, start_date, due_date, created_by)
            VALUES ($1, $2, 'Vận hành chuyển đổi số doanh nghiệp', 'Chuẩn hóa quy trình làm việc, phân quyền phòng ban và tự động hóa báo cáo nội bộ', 'active', 'high', $3, $4, '2026-01-01', '2026-12-31', $3)
            RETURNING id
            "#,
        )
        .bind(server_id)
        .bind(primary_dept_id)
        .bind(owner_id)
        .bind(ch_map.get("Executive Board").copied())
        .fetch_one(&mut **tx)
        .await?;

        let _p2_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO enterprise_projects (server_id, department_id, name, description, status, priority, manager_user_id, linked_channel_id, start_date, due_date, created_by)
            VALUES ($1, $2, 'Phát triển nền tảng bảo mật hệ thống', 'Nâng cấp kiến trúc bảo mật, hiệu năng thời gian thực và quy trình kiểm thử', 'active', 'medium', $3, $4, '2026-02-01', '2026-12-31', $3)
            RETURNING id
            "#,
        )
        .bind(server_id)
        .bind(eng_dept_id.or(primary_dept_id))
        .bind(owner_id)
        .bind(ch_map.get("Product engineering").copied())
        .fetch_one(&mut **tx)
        .await?;

        let task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1",
        )
        .bind(server_id)
        .fetch_one(&mut **tx)
        .await?;

        if task_count == 0 {
            let starter_tasks = [
                ("Establish departmental structure and decentralize personnel", "Check the list of departments, appoint department heads and assign corresponding roles", "done", "high", "2026-11-20"),
                ("Implement timekeeping and leave approval processes", "Instruct members to use /checkin, /checkout commands and submit applications on the system", "in_progress", "high", "2026-11-25"),
                ("Synchronize regulatory documents and employee handbooks", "Upload work regulations and administrative forms to your corporate document repository", "review", "medium", "2026-11-28"),
                ("Evaluate performance and periodically report operations", "Summary of work completion and attendance indicators in the report section", "todo", "medium", "2026-12-05"),
            ];
            for (title, desc, status, prio, due) in starter_tasks {
                sqlx::query(
                    r#"
                    INSERT INTO enterprise_tasks (server_id, project_id, department_id, title, description, status, priority, assignee_id, due_date, created_by)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $8)
                    "#,
                )
                .bind(server_id)
                .bind(p1_id)
                .bind(primary_dept_id)
                .bind(title)
                .bind(desc)
                .bind(status)
                .bind(prio)
                .bind(owner_id)
                .bind(due)
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    // 8. Default Scheduled Meetings, Folders, Pinned Announcement, and Automation Workflows
    let meet_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_meetings WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;
    if meet_count == 0 {
        let now = Utc::now();
        let m1_start = now + chrono::Duration::hours(2);
        let m1_end = m1_start + chrono::Duration::hours(1);
        let m2_start = now + chrono::Duration::hours(26);
        let m2_end = m2_start + chrono::Duration::minutes(45);

        sqlx::query(
            r#"
            INSERT INTO enterprise_meetings (server_id, department_id, channel_id, title, description, meeting_type, host_id, starts_at, ends_at, recurrence, notes, status)
            VALUES ($1, $2, $3, 'Họp giao ban điều hành tuần', 'Rà soát mục tiêu trọng tâm, tiến độ dự án và phê duyệt tồn đọng', 'review', $4, $5, $6, 'weekly', 'Chuẩn bị số liệu báo cáo tuần của từng phòng ban', 'scheduled')
            "#,
        )
        .bind(server_id)
        .bind(primary_dept_id)
        .bind(ch_map.get("Executive meeting room").copied())
        .bind(owner_id)
        .bind(m1_start)
        .bind(m1_end)
        .execute(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO enterprise_meetings (server_id, department_id, channel_id, title, description, meeting_type, host_id, starts_at, ends_at, recurrence, notes, status)
            VALUES ($1, $2, $3, 'Họp nhanh đầu ngày', 'Cập nhật nhanh công việc trong ngày và xử lý vướng mắc kỹ thuật', 'standup', $4, $5, $6, 'daily', 'Mỗi thành viên cập nhật ngắn gọn 3 phút', 'scheduled')
            "#,
        )
        .bind(server_id)
        .bind(eng_dept_id.or(primary_dept_id))
        .bind(ch_map.get("Quick meeting room").copied())
        .bind(owner_id)
        .bind(m2_start)
        .bind(m2_end)
        .execute(&mut **tx)
        .await?;
    }

    let folder_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_folders WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;
    if folder_count == 0 {
        for fname in ["Regulations and personnel handbook", "Project and technical documents", "Forms and contracts"] {
            sqlx::query(
                "INSERT INTO enterprise_folders (server_id, scope, owner_id, name) VALUES ($1, 'company', $2, $3)",
            )
            .bind(server_id)
            .bind(owner_id)
            .bind(fname)
            .execute(&mut **tx)
            .await?;
        }
    }

    let ann_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_announcements WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;
    if ann_count == 0 {
        sqlx::query(
            r#"
            INSERT INTO enterprise_announcements (server_id, scope, author_id, title, content, priority, pinned)
            VALUES ($1, 'company', $2, 'Chính thức vận hành không gian doanh nghiệp', 'Toàn bộ phòng ban, kênh phối hợp, bảng công việc, chấm công, nghỉ phép, phê duyệt và lệnh nhanh trong kênh chat đã sẵn sàng hoạt động.', 'important', TRUE)
            "#,
        )
        .bind(server_id)
        .bind(owner_id)
        .execute(&mut **tx)
        .await?;
    }

    let wf_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_workflows WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&mut **tx)
    .await?;
    if wf_count == 0 {
        let auto_ch = ch_map.get("Automation").copied();
        let cfg = serde_json::json!({ "channel_id": auto_ch });
        let default_wfs = [
            ("Notification when new job is assigned", "task_assigned", "post_channel_message"),
            ("Updated when work is completed", "task_completed", "post_channel_message"),
            ("Notification of leave approval results", "leave_decided", "post_channel_message"),
            ("Notification of proposal approval results", "approval_decided", "post_channel_message"),
        ];
        for (wname, trig, act) in default_wfs {
            sqlx::query(
                r#"
                INSERT INTO enterprise_workflows (server_id, name, trigger_event, action_type, action_config, enabled)
                VALUES ($1, $2, $3, $4, $5, TRUE)
                "#,
            )
            .bind(server_id)
            .bind(wname)
            .bind(trig)
            .bind(act)
            .bind(&cfg)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------- Config

#[derive(Serialize)]
pub struct ConfigResponse {
    #[serde(flatten)]
    pub config: EnterpriseConfigRow,
    pub retention_locked: bool,
    pub enterprise_disabled_at: Option<DateTime<Utc>>,
    pub my_permissions: i64,
    pub my_enterprise_permissions: i64,
    pub is_owner: bool,
    pub department_ids: Vec<Uuid>,
    pub managed_department_ids: Vec<Uuid>,
}

pub async fn get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<ConfigResponse>> {
    let uid = auth_user(&state, &headers).await?;
    let mut ctx = load_ctx(&state.db, server_id, uid).await?;

    // Guarantee config row exists and auto-backfill starter channels/departments/projects for existing enterprise servers
    let needs_seed: bool = sqlx::query_scalar(
        r#"
        SELECT (SELECT is_enterprise FROM servers WHERE id = $1)
           AND (SELECT COUNT(*) FROM enterprise_departments WHERE server_id = $1) = 0
           AND (SELECT COUNT(*) FROM enterprise_projects WHERE server_id = $1) = 0
        "#,
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    if needs_seed {
        if let Ok(mut tx) = state.db.begin().await {
            let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM servers WHERE id = $1")
                .bind(server_id)
                .fetch_one(&mut *tx)
                .await
                .unwrap_or(uid);
            if init_enterprise_tx(&mut tx, server_id, owner_id).await.is_ok() {
                let _ = tx.commit().await;
                if let Ok(reloaded) = load_ctx(&state.db, server_id, uid).await {
                    ctx = reloaded;
                }
            }
        }
    } else {
        sqlx::query("INSERT INTO server_enterprise_configs (server_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(server_id)
            .execute(&state.db)
            .await?;
    }

    let config: EnterpriseConfigRow = sqlx::query_as(
        "SELECT * FROM server_enterprise_configs WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    let srv_row = sqlx::query("SELECT retention_locked, enterprise_disabled_at FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;
    let retention_locked: bool = srv_row.try_get("retention_locked")?;
    let enterprise_disabled_at: Option<DateTime<Utc>> = srv_row.try_get("enterprise_disabled_at")?;

    Ok(Json(ConfigResponse {
        config,
        retention_locked,
        enterprise_disabled_at,
        my_permissions: ctx.base_bits,
        my_enterprise_permissions: ctx.ent_bits,
        is_owner: ctx.is_owner,
        department_ids: ctx.dept_ids,
        managed_department_ids: ctx.managed_dept_ids,
    }))
}

#[derive(Deserialize)]
pub struct UpdateConfigReq {
    pub enabled_modules: Option<Value>,
    pub module_order: Option<Value>,
    pub general_settings: Option<Value>,
    pub display_settings: Option<Value>,
    pub security_policy: Option<Value>,
    pub hr_policy: Option<Value>,
    pub attendance_policy: Option<Value>,
    pub leave_policy: Option<Value>,
    pub approval_policy: Option<Value>,
    pub task_policy: Option<Value>,
    pub project_policy: Option<Value>,
    pub document_policy: Option<Value>,
    pub notification_policy: Option<Value>,
    pub ai_config: Option<Value>,
    pub privacy_policy: Option<Value>,
    pub retention_locked: Option<bool>,
}

const VALID_MODULES: &[&str] = &[
    "overview",
    "hr",
    "departments",
    "tasks",
    "projects",
    "calendar",
    "meetings",
    "attendance",
    "leave",
    "approvals",
    "documents",
    "announcements",
    "reports",
    "ai",
    "automation",
];

fn sanitize_module_list(val: Option<Value>) -> AppResult<Option<Value>> {
    let Some(v) = val else { return Ok(None) };
    let arr = v.as_array().ok_or(AppError::BadRequest("invalid_modules"))?;
    let mut out = Vec::new();
    for item in arr {
        let s = item.as_str().ok_or(AppError::BadRequest("invalid_modules"))?;
        if VALID_MODULES.contains(&s) && !out.contains(&Value::String(s.to_string())) {
            out.push(Value::String(s.to_string()));
        }
    }
    if out.is_empty() {
        out.push(Value::String("overview".to_string()));
    }
    Ok(Some(Value::Array(out)))
}

pub async fn update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<UpdateConfigReq>,
) -> AppResult<Json<EnterpriseConfigRow>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let touching_sec = p.security_policy.is_some() || p.retention_locked.is_some() || p.privacy_policy.is_some();
    let touching_ai = p.ai_config.is_some();
    if touching_sec && !ctx.has(ent_perm::MANAGE_SECURITY) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_security_permissions"));
    }
    if touching_ai && !ctx.has(ent_perm::MANAGE_AI) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_ai_permissions"));
    }
    if !touching_sec && !touching_ai && !ctx.has(ent_perm::MANAGE_ENTERPRISE) && !ctx.is_admin() {
        return Err(AppError::Forbidden("insufficient_enterprise_permissions"));
    }
    if p.retention_locked.is_some() && !ctx.is_owner && !ctx.has(ent_perm::MANAGE_SECURITY) {
        return Err(AppError::Forbidden("insufficient_security_permissions"));
    }

    let enabled_modules = sanitize_module_list(p.enabled_modules)?;
    let module_order = sanitize_module_list(p.module_order)?;

    sqlx::query("INSERT INTO server_enterprise_configs (server_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(server_id)
        .execute(&state.db)
        .await?;

    let updated: EnterpriseConfigRow = sqlx::query_as(
        r#"
        UPDATE server_enterprise_configs SET
            enabled_modules     = COALESCE($2, enabled_modules),
            module_order        = COALESCE($3, module_order),
            general_settings    = COALESCE($4, general_settings),
            display_settings    = COALESCE($5, display_settings),
            security_policy     = COALESCE($6, security_policy),
            hr_policy           = COALESCE($7, hr_policy),
            attendance_policy   = COALESCE($8, attendance_policy),
            leave_policy        = COALESCE($9, leave_policy),
            approval_policy     = COALESCE($10, approval_policy),
            task_policy         = COALESCE($11, task_policy),
            project_policy      = COALESCE($12, project_policy),
            document_policy     = COALESCE($13, document_policy),
            notification_policy = COALESCE($14, notification_policy),
            ai_config           = COALESCE($15, ai_config),
            privacy_policy      = COALESCE($16, privacy_policy),
            updated_at          = now()
        WHERE server_id = $1
        RETURNING *
        "#,
    )
    .bind(server_id)
    .bind(enabled_modules)
    .bind(module_order)
    .bind(p.general_settings)
    .bind(p.display_settings)
    .bind(p.security_policy)
    .bind(p.hr_policy)
    .bind(p.attendance_policy)
    .bind(p.leave_policy)
    .bind(p.approval_policy)
    .bind(p.task_policy)
    .bind(p.project_policy)
    .bind(p.document_policy)
    .bind(p.notification_policy)
    .bind(p.ai_config)
    .bind(p.privacy_policy)
    .fetch_one(&state.db)
    .await?;

    if let Some(lock) = p.retention_locked {
        sqlx::query("UPDATE servers SET retention_locked = $2 WHERE id = $1")
            .bind(server_id)
            .bind(lock)
            .execute(&state.db)
            .await?;
    }

    audit(&state, server_id, uid, "ent_config_updated", "enterprise_settings").await;
    emit_enterprise(&state, server_id, "config.updated", json!({ "server_id": server_id })).await;
    Ok(Json(updated))
}

// ---------------------------------------------------------------- Overview

pub async fn get_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let today_str = Utc::now().format("%Y-%m-%d").to_string();

    let my_open_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1 AND (assignee_id = $2 OR $2 = ANY(participant_ids)) AND status NOT IN ('done', 'archived')",
    )
    .bind(server_id)
    .bind(uid)
    .fetch_one(&state.db)
    .await?;

    let my_overdue_tasks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1 AND (assignee_id = $2 OR $2 = ANY(participant_ids)) AND status NOT IN ('done', 'archived') AND due_date IS NOT NULL AND due_date < $3",
    )
    .bind(server_id)
    .bind(uid)
    .bind(&today_str)
    .fetch_one(&state.db)
    .await?;

    let total_tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1")
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;

    let done_tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_tasks WHERE server_id = $1 AND status = 'done'")
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;

    let today_meetings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_meetings WHERE server_id = $1 AND status IN ('scheduled', 'live') AND starts_at >= date_trunc('day', now()) AND starts_at < date_trunc('day', now()) + interval '1 day'",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    let unread_announcements: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM enterprise_announcements a
        WHERE a.server_id = $1
          AND (a.scheduled_at IS NULL OR a.scheduled_at <= now())
          AND NOT EXISTS (
              SELECT 1 FROM enterprise_announcement_reads r WHERE r.announcement_id = a.id AND r.user_id = $2
          )
        "#,
    )
    .bind(server_id)
    .bind(uid)
    .fetch_one(&state.db)
    .await?;

    let pending_approvals: i64 = if ctx.is_admin() || ctx.has(ent_perm::APPROVE_REQUESTS) {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM enterprise_approval_requests WHERE server_id = $1 AND status IN ('pending', 'needs_revision')",
        )
        .bind(server_id)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM enterprise_approval_requests WHERE server_id = $1 AND requester_id = $2 AND status IN ('pending', 'needs_revision')",
        )
        .bind(server_id)
        .bind(uid)
        .fetch_one(&state.db)
        .await?
    };

    let pending_leave: i64 = if ctx.is_admin() || ctx.has(ent_perm::APPROVE_LEAVE) {
        sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_leave_requests WHERE server_id = $1 AND status = 'pending'")
            .bind(server_id)
            .fetch_one(&state.db)
            .await?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_leave_requests WHERE server_id = $1 AND user_id = $2 AND status = 'pending'")
            .bind(server_id)
            .bind(uid)
            .fetch_one(&state.db)
            .await?
    };

    let my_attendance = sqlx::query_as::<_, time::AttendanceItem>(
        r#"
        SELECT a.id, a.server_id, a.user_id, u.username, a.work_date,
               a.check_in_at, a.check_out_at, a.work_mode, a.status,
               a.late_minutes, a.early_minutes, a.overtime_minutes, a.note
        FROM enterprise_attendance a
        JOIN users u ON u.id = a.user_id
        WHERE a.server_id = $1 AND a.user_id = $2 AND a.work_date = $3
        "#,
    )
    .bind(server_id)
    .bind(uid)
    .bind(&today_str)
    .fetch_optional(&state.db)
    .await?;

    let active_employees: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_employees WHERE server_id = $1 AND employment_status = 'active'",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    let departments_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_departments WHERE server_id = $1")
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;

    let active_projects: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enterprise_projects WHERE server_id = $1 AND status IN ('planning', 'active')",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "my_open_tasks": my_open_tasks,
        "my_overdue_tasks": my_overdue_tasks,
        "total_tasks": total_tasks,
        "done_tasks": done_tasks,
        "today_meetings": today_meetings,
        "unread_announcements": unread_announcements,
        "pending_approvals": pending_approvals,
        "pending_leave": pending_leave,
        "my_attendance": my_attendance,
        "active_employees": active_employees,
        "departments_count": departments_count,
        "active_projects": active_projects,
    })))
}

// ---------------------------------------------------------------- Search

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub kind: &'static str,
    pub module: &'static str,
    pub id: String,
    pub title: String,
    pub subtitle: String,
}

pub async fn enterprise_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<SearchResultItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let term = query.q.as_deref().map(str::trim).unwrap_or("");
    if term.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let pat = format!("%{}%", term.to_lowercase());
    let mut results = Vec::new();

    if ctx.has(ent_perm::VIEW_HR) || ctx.has(ent_perm::MANAGE_HR) || ctx.is_admin() {
        let emps = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
            "SELECT user_id, full_name, job_title FROM enterprise_employees WHERE server_id = $1 AND (lower(full_name) LIKE $2 OR lower(COALESCE(job_title,'')) LIKE $2) LIMIT 6",
        )
        .bind(server_id)
        .bind(&pat)
        .fetch_all(&state.db)
        .await?;
        for (id, name, title) in emps {
            results.push(SearchResultItem {
                kind: "employee",
                module: "hr",
                id: id.to_string(),
                title: name,
                subtitle: title.unwrap_or_else(|| "Human resources".to_string()),
            });
        }
    }

    let depts = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "SELECT id, name, branch_name FROM enterprise_departments WHERE server_id = $1 AND lower(name) LIKE $2 LIMIT 5",
    )
    .bind(server_id)
    .bind(&pat)
    .fetch_all(&state.db)
    .await?;
    for (id, name, branch) in depts {
        results.push(SearchResultItem {
            kind: "department",
            module: "departments",
            id: id.to_string(),
            title: name,
            subtitle: branch.unwrap_or_else(|| "Departments".to_string()),
        });
    }

    let projs = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, name, status FROM enterprise_projects WHERE server_id = $1 AND lower(name) LIKE $2 LIMIT 5",
    )
    .bind(server_id)
    .bind(&pat)
    .fetch_all(&state.db)
    .await?;
    for (id, name, status) in projs {
        results.push(SearchResultItem {
            kind: "project",
            module: "projects",
            id: id.to_string(),
            title: name,
            subtitle: status,
        });
    }

    let is_task_all = ctx.is_admin() || ctx.has(ent_perm::ASSIGN_TASKS) || ctx.has(ent_perm::VIEW_REPORTS);
    let tasks = sqlx::query_as::<_, (Uuid, String, String)>(
        r#"
        SELECT id, title, status
        FROM enterprise_tasks
        WHERE server_id = $1 AND lower(title) LIKE $2
          AND ($3::bool OR assignee_id = $4 OR created_by = $4 OR $4 = ANY(participant_ids) OR $4 = ANY(watcher_ids))
        LIMIT 8
        "#,
    )
    .bind(server_id)
    .bind(&pat)
    .bind(is_task_all)
    .bind(uid)
    .fetch_all(&state.db)
    .await?;
    for (id, title, status) in tasks {
        results.push(SearchResultItem {
            kind: "task",
            module: "tasks",
            id: id.to_string(),
            title,
            subtitle: status,
        });
    }

    let can_all_docs = ctx.is_admin() || ctx.has(ent_perm::MANAGE_DOCUMENTS);
    let docs = sqlx::query_as::<_, (Uuid, String, String)>(
        r#"
        SELECT id, title, scope
        FROM enterprise_documents
        WHERE server_id = $1 AND trashed_at IS NULL AND lower(title) LIKE $2
          AND (scope = 'company' OR owner_id = $3 OR $4::bool
               OR (scope = 'department' AND department_id = ANY($5)))
        LIMIT 6
        "#,
    )
    .bind(server_id)
    .bind(&pat)
    .bind(uid)
    .bind(can_all_docs)
    .bind(&ctx.dept_ids)
    .fetch_all(&state.db)
    .await?;
    for (id, title, scope) in docs {
        results.push(SearchResultItem {
            kind: "document",
            module: "documents",
            id: id.to_string(),
            title,
            subtitle: scope,
        });
    }

    let anns = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, title, priority FROM enterprise_announcements WHERE server_id = $1 AND (scope = 'company' OR $3::bool OR (department_id IS NOT NULL AND department_id = ANY($4))) AND (lower(title) LIKE $2 OR lower(content) LIKE $2) LIMIT 5",
    )
    .bind(server_id)
    .bind(&pat)
    .bind(ctx.is_admin())
    .bind(&ctx.dept_ids)
    .fetch_all(&state.db)
    .await?;
    for (id, title, priority) in anns {
        results.push(SearchResultItem {
            kind: "announcement",
            module: "announcements",
            id: id.to_string(),
            title,
            subtitle: priority,
        });
    }

    let meetings = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, title, status FROM enterprise_meetings WHERE server_id = $1 AND ($3::bool OR organizer_id = $4 OR $4 = ANY(attendee_ids) OR (department_id IS NOT NULL AND department_id = ANY($5))) AND lower(title) LIKE $2 LIMIT 5",
    )
    .bind(server_id)
    .bind(&pat)
    .bind(ctx.is_admin())
    .bind(uid)
    .bind(&ctx.dept_ids)
    .fetch_all(&state.db)
    .await?;
    for (id, title, status) in meetings {
        results.push(SearchResultItem {
            kind: "meeting",
            module: "meetings",
            id: id.to_string(),
            title,
            subtitle: status,
        });
    }

    Ok(Json(results))
}
