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
    enterprise::models::{
        clean_opt, clean_req, emit_enterprise, ent_perm, load_ctx, valid_ymd,
    },
    error::{AppError, AppResult},
    privacy::auth_user,
};

#[derive(Deserialize)]
pub struct EmployeeQuery {
    pub q: Option<String>,
    pub department_id: Option<Uuid>,
    pub status: Option<String>,
    pub kind: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EmployeeItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub presence: String,
    pub employee_code: Option<String>,
    pub member_kind: String,
    pub full_name: String,
    pub work_email: Option<String>,
    pub work_phone: Option<String>,
    pub job_title: Option<String>,
    pub position_level: Option<String>,
    pub department_id: Option<Uuid>,
    pub department_name: Option<String>,
    pub manager_user_id: Option<Uuid>,
    pub branch: Option<String>,
    pub work_location: Option<String>,
    pub work_mode: String,
    pub contract_type: Option<String>,
    pub employment_status: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub timezone: String,
    pub work_schedule: String,
    pub skills: Vec<String>,
    pub internal_notes: Option<String>,
    pub custom_fields: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_employees(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(q): Query<EmployeeQuery>,
) -> AppResult<Json<Vec<EmployeeItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::VIEW_HR)?;

    let can_sensitive = ctx.has(ent_perm::VIEW_SENSITIVE_HR);
    let search = q.q.as_deref().map(str::trim).unwrap_or("").to_lowercase();
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);
    let sort_key = q.sort.as_deref().unwrap_or("name");

    let mut rows: Vec<EmployeeItem> = sqlx::query_as(
        r#"
        SELECT e.id, e.server_id, e.user_id, u.username,
               COALESCE(sm.avatar_url, u.avatar_url) AS avatar_url,
               u.presence, e.employee_code, e.member_kind, e.full_name,
               e.work_email, e.work_phone, e.job_title, e.position_level,
               e.department_id, d.name AS department_name,
               e.manager_user_id, e.branch, e.work_location, e.work_mode,
               e.contract_type, e.employment_status, e.start_date, e.end_date,
               e.timezone, e.work_schedule, e.skills, e.internal_notes,
               e.custom_fields, e.created_at, e.updated_at
        FROM enterprise_employees e
        JOIN users u ON u.id = e.user_id
        LEFT JOIN server_members sm ON sm.server_id = e.server_id AND sm.user_id = e.user_id
        LEFT JOIN enterprise_departments d ON d.id = e.department_id
        WHERE e.server_id = $1
          AND ($2::uuid IS NULL OR e.department_id = $2)
          AND ($3::text IS NULL OR e.employment_status = $3)
          AND ($4::text IS NULL OR e.member_kind = $4)
          AND ($5 = '' OR lower(e.full_name) LIKE '%' || $5 || '%'
               OR lower(u.username) LIKE '%' || $5 || '%'
               OR lower(COALESCE(e.employee_code, '')) LIKE '%' || $5 || '%'
               OR lower(COALESCE(e.job_title, '')) LIKE '%' || $5 || '%')
        ORDER BY
            CASE WHEN $8 = 'recent' THEN e.created_at END DESC,
            CASE WHEN $8 = 'code' THEN COALESCE(e.employee_code, '') END ASC,
            e.full_name ASC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(server_id)
    .bind(q.department_id)
    .bind(q.status.as_deref().filter(|s| !s.is_empty()))
    .bind(q.kind.as_deref().filter(|s| !s.is_empty()))
    .bind(search)
    .bind(limit)
    .bind(offset)
    .bind(sort_key)
    .fetch_all(&state.db)
    .await?;

    if !can_sensitive {
        for r in &mut rows {
            if r.user_id != uid {
                r.work_email = None;
                r.work_phone = None;
                r.contract_type = None;
                r.internal_notes = None;
            }
        }
    }

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct UpsertEmployeeReq {
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub employee_code: Option<String>,
    pub member_kind: Option<String>,
    pub work_email: Option<String>,
    pub work_phone: Option<String>,
    pub job_title: Option<String>,
    pub position_level: Option<String>,
    pub department_id: Option<Uuid>,
    pub manager_user_id: Option<Uuid>,
    pub branch: Option<String>,
    pub work_location: Option<String>,
    pub work_mode: Option<String>,
    pub contract_type: Option<String>,
    pub employment_status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub timezone: Option<String>,
    pub work_schedule: Option<String>,
    pub skills: Option<Vec<String>>,
    pub internal_notes: Option<String>,
    pub custom_fields: Option<Value>,
}

pub async fn upsert_employee(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<UpsertEmployeeReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_HR)?;

    let target_uid = if let Some(u) = p.user_id {
        u
    } else if let Some(ref uname) = p.username {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE lower(username) = lower($1)")
            .bind(uname.trim())
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound("user_not_found"))?
    } else {
        return Err(AppError::BadRequest("missing_target_user"));
    };

    let uname: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(target_uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user_not_found"))?;

    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2)",
    )
    .bind(server_id)
    .bind(target_uid)
    .fetch_one(&state.db)
    .await?;

    let mut tx = state.db.begin().await?;
    if !is_member {
        sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(server_id)
            .bind(target_uid)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(dept_id) = p.department_id {
        let valid_dept: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM enterprise_departments WHERE id = $1 AND server_id = $2)",
        )
        .bind(dept_id)
        .bind(server_id)
        .fetch_one(&mut *tx)
        .await?;
        if !valid_dept {
            return Err(AppError::BadRequest("invalid_department"));
        }
    }

    let full_name = clean_opt(p.full_name.as_deref(), 100, "invalid_full_name")?
        .unwrap_or_else(|| uname.clone());
    let kind = p.member_kind.as_deref().unwrap_or("employee");
    if !["employee", "guest", "partner", "bot"].contains(&kind) {
        return Err(AppError::BadRequest("invalid_member_kind"));
    }
    let work_mode = p.work_mode.as_deref().unwrap_or("onsite");
    if !["onsite", "remote", "hybrid"].contains(&work_mode) {
        return Err(AppError::BadRequest("invalid_work_mode"));
    }
    let emp_status = p.employment_status.as_deref().unwrap_or("active");
    if !["active", "onboarding", "suspended", "offboarded"].contains(&emp_status) {
        return Err(AppError::BadRequest("invalid_employment_status"));
    }
    if let Some(ref sd) = p.start_date {
        if !sd.trim().is_empty() && !valid_ymd(sd) {
            return Err(AppError::BadRequest("invalid_start_date"));
        }
    }
    if let Some(ref ed) = p.end_date {
        if !ed.trim().is_empty() && !valid_ymd(ed) {
            return Err(AppError::BadRequest("invalid_end_date"));
        }
    }

    let emp_code = if let Some(c) = clean_opt(p.employee_code.as_deref(), 32, "invalid_code")? {
        Some(c)
    } else {
        let cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_employees WHERE server_id = $1")
            .bind(server_id)
            .fetch_one(&mut *tx)
            .await?;
        Some(format!("NV-{:04}", cnt + 1))
    };

    let work_email = clean_opt(p.work_email.as_deref(), 120, "invalid_email")?;
    let work_phone = clean_opt(p.work_phone.as_deref(), 40, "invalid_phone")?;
    let job_title = clean_opt(p.job_title.as_deref(), 80, "invalid_job_title")?;
    let pos_level = clean_opt(p.position_level.as_deref(), 40, "invalid_position")?;
    let branch = clean_opt(p.branch.as_deref(), 80, "invalid_branch")?;
    let loc = clean_opt(p.work_location.as_deref(), 100, "invalid_location")?;
    let contract = clean_opt(p.contract_type.as_deref(), 50, "invalid_contract")?;
    let notes = clean_opt(p.internal_notes.as_deref(), 2000, "invalid_notes")?;
    let tz = p.timezone.unwrap_or_else(|| "Asia/Ho_Chi_Minh".into());
    let sched = p.work_schedule.unwrap_or_else(|| "08:30-17:30".into());
    let skills: Vec<String> = p
        .skills
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() <= 40)
        .take(24)
        .collect();
    let custom_fields = p.custom_fields.unwrap_or_else(|| json!({}));

    let emp_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO enterprise_employees (
            server_id, user_id, employee_code, member_kind, full_name,
            work_email, work_phone, job_title, position_level, department_id,
            manager_user_id, branch, work_location, work_mode, contract_type,
            employment_status, start_date, end_date, timezone, work_schedule,
            skills, internal_notes, custom_fields, updated_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23, now())
        ON CONFLICT (server_id, user_id) DO UPDATE SET
            employee_code = COALESCE(EXCLUDED.employee_code, enterprise_employees.employee_code),
            member_kind = EXCLUDED.member_kind,
            full_name = EXCLUDED.full_name,
            work_email = EXCLUDED.work_email,
            work_phone = EXCLUDED.work_phone,
            job_title = EXCLUDED.job_title,
            position_level = EXCLUDED.position_level,
            department_id = EXCLUDED.department_id,
            manager_user_id = EXCLUDED.manager_user_id,
            branch = EXCLUDED.branch,
            work_location = EXCLUDED.work_location,
            work_mode = EXCLUDED.work_mode,
            contract_type = EXCLUDED.contract_type,
            employment_status = EXCLUDED.employment_status,
            start_date = EXCLUDED.start_date,
            end_date = EXCLUDED.end_date,
            timezone = EXCLUDED.timezone,
            work_schedule = EXCLUDED.work_schedule,
            skills = EXCLUDED.skills,
            internal_notes = EXCLUDED.internal_notes,
            custom_fields = EXCLUDED.custom_fields,
            updated_at = now()
        RETURNING id
        "#,
    )
    .bind(server_id)
    .bind(target_uid)
    .bind(&emp_code)
    .bind(kind)
    .bind(&full_name)
    .bind(work_email)
    .bind(work_phone)
    .bind(job_title)
    .bind(pos_level)
    .bind(p.department_id)
    .bind(p.manager_user_id)
    .bind(branch)
    .bind(loc)
    .bind(work_mode)
    .bind(contract)
    .bind(emp_status)
    .bind(p.start_date.filter(|s| !s.trim().is_empty()))
    .bind(p.end_date.filter(|s| !s.trim().is_empty()))
    .bind(tz)
    .bind(sched)
    .bind(&skills)
    .bind(notes)
    .bind(custom_fields)
    .fetch_one(&mut *tx)
    .await?;

    if let Some(dept_id) = p.department_id {
        sqlx::query(
            "INSERT INTO enterprise_department_members (department_id, server_id, user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(dept_id)
        .bind(server_id)
        .bind(target_uid)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    audit(&state, server_id, uid, "ent_employee_saved", &full_name).await;
    emit_enterprise(&state, server_id, "hr.updated", json!({ "user_id": target_uid, "status": emp_status })).await;
    Ok(Json(json!({ "id": emp_id, "user_id": target_uid, "employee_code": emp_code })))
}

#[derive(Deserialize)]
pub struct BulkEmployeesReq {
    pub user_ids: Vec<Uuid>,
    pub department_id: Option<Uuid>,
    pub employment_status: Option<String>,
    pub work_mode: Option<String>,
}

pub async fn bulk_update_employees(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<BulkEmployeesReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_HR)?;

    if p.user_ids.is_empty() || p.user_ids.len() > 200 {
        return Err(AppError::BadRequest("invalid_bulk_selection"));
    }
    if let Some(ref st) = p.employment_status {
        if !["active", "onboarding", "suspended", "offboarded"].contains(&st.as_str()) {
            return Err(AppError::BadRequest("invalid_employment_status"));
        }
    }
    if let Some(ref wm) = p.work_mode {
        if !["onsite", "remote", "hybrid"].contains(&wm.as_str()) {
            return Err(AppError::BadRequest("invalid_work_mode"));
        }
    }

    let mut tx = state.db.begin().await?;
    let mut updated = 0usize;
    for tid in &p.user_ids {
        let res = sqlx::query(
            r#"
            UPDATE enterprise_employees
            SET department_id = COALESCE($3, department_id),
                employment_status = COALESCE($4, employment_status),
                work_mode = COALESCE($5, work_mode),
                updated_at = now()
            WHERE server_id = $1 AND user_id = $2
            "#,
        )
        .bind(server_id)
        .bind(tid)
        .bind(p.department_id)
        .bind(p.employment_status.as_deref())
        .bind(p.work_mode.as_deref())
        .execute(&mut *tx)
        .await?;
        updated += res.rows_affected() as usize;
        if let Some(dept_id) = p.department_id {
            sqlx::query(
                "INSERT INTO enterprise_department_members (department_id, server_id, user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(dept_id)
            .bind(server_id)
            .bind(tid)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    audit(&state, server_id, uid, "ent_employee_bulk", &format!("count={updated}")).await;
    emit_enterprise(&state, server_id, "hr.updated", json!({ "bulk": updated })).await;
    Ok(Json(json!({ "updated": updated })))
}

#[derive(Deserialize)]
pub struct ImportEmployeeRow {
    pub username: String,
    pub full_name: Option<String>,
    pub job_title: Option<String>,
    pub department_code: Option<String>,
    pub member_kind: Option<String>,
    pub work_mode: Option<String>,
}

#[derive(Deserialize)]
pub struct ImportEmployeesReq {
    pub rows: Vec<ImportEmployeeRow>,
}

pub async fn import_employees(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<ImportEmployeesReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_HR)?;

    if p.rows.is_empty() || p.rows.len() > 200 {
        return Err(AppError::BadRequest("invalid_import_batch"));
    }

    let mut tx = state.db.begin().await?;
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let base_cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enterprise_employees WHERE server_id = $1")
        .bind(server_id)
        .fetch_one(&mut *tx)
        .await?;

    for (idx, item) in p.rows.iter().enumerate() {
        let uname_trim = item.username.trim();
        if uname_trim.is_empty() {
            skipped += 1;
            continue;
        }
        let Some((target_uid, actual_uname)) = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, username FROM users WHERE lower(username) = lower($1)",
        )
        .bind(uname_trim)
        .fetch_optional(&mut *tx)
        .await?
        else {
            skipped += 1;
            continue;
        };

        sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(server_id)
            .bind(target_uid)
            .execute(&mut *tx)
            .await?;

        let dept_id: Option<Uuid> = if let Some(ref dc) = item.department_code {
            let dc_t = dc.trim();
            if dc_t.is_empty() {
                None
            } else {
                sqlx::query_scalar(
                    "SELECT id FROM enterprise_departments WHERE server_id = $1 AND (lower(code) = lower($2) OR lower(name) = lower($2)) LIMIT 1",
                )
                .bind(server_id)
                .bind(dc_t)
                .fetch_optional(&mut *tx)
                .await?
            }
        } else {
            None
        };

        let full_name = clean_opt(item.full_name.as_deref(), 100, "invalid_full_name")?
            .unwrap_or_else(|| actual_uname.clone());
        let job_title = clean_opt(item.job_title.as_deref(), 80, "invalid_job_title")?;
        let kind = item.member_kind.as_deref().unwrap_or("employee");
        let safe_kind = if ["employee", "guest", "partner", "bot"].contains(&kind) { kind } else { "employee" };
        let mode = item.work_mode.as_deref().unwrap_or("onsite");
        let safe_mode = if ["onsite", "remote", "hybrid"].contains(&mode) { mode } else { "onsite" };
        let code = format!("NV-{:04}", base_cnt + (idx as i64) + 1);

        sqlx::query(
            r#"
            INSERT INTO enterprise_employees (
                server_id, user_id, employee_code, member_kind, full_name,
                job_title, department_id, work_mode, employment_status, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active', now())
            ON CONFLICT (server_id, user_id) DO UPDATE SET
                full_name = EXCLUDED.full_name,
                job_title = COALESCE(EXCLUDED.job_title, enterprise_employees.job_title),
                department_id = COALESCE(EXCLUDED.department_id, enterprise_employees.department_id),
                member_kind = EXCLUDED.member_kind,
                work_mode = EXCLUDED.work_mode,
                updated_at = now()
            "#,
        )
        .bind(server_id)
        .bind(target_uid)
        .bind(&code)
        .bind(safe_kind)
        .bind(&full_name)
        .bind(job_title)
        .bind(dept_id)
        .bind(safe_mode)
        .execute(&mut *tx)
        .await?;

        if let Some(did) = dept_id {
            sqlx::query(
                "INSERT INTO enterprise_department_members (department_id, server_id, user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(did)
            .bind(server_id)
            .bind(target_uid)
            .execute(&mut *tx)
            .await?;
        }
        imported += 1;
    }

    tx.commit().await?;
    audit(&state, server_id, uid, "ent_employee_import", &format!("imported={imported},skipped={skipped}")).await;
    emit_enterprise(&state, server_id, "hr.updated", json!({ "imported": imported })).await;
    Ok(Json(json!({ "imported": imported, "skipped": skipped })))
}

#[derive(Deserialize)]
pub struct HandoverReq {
    #[serde(alias = "handover_to")]
    pub to_user_id: Uuid,
    #[serde(default)]
    pub mark_offboarded: bool,
}

pub async fn handover_employee(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, from_uid)): Path<(Uuid, Uuid)>,
    Json(p): Json<HandoverReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_HR)?;
    ctx.require(ent_perm::ASSIGN_TASKS)?;

    if from_uid == p.to_user_id {
        return Err(AppError::BadRequest("same_user"));
    }
    let to_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2)",
    )
    .bind(server_id)
    .bind(p.to_user_id)
    .fetch_one(&state.db)
    .await?;
    if !to_member {
        return Err(AppError::NotFound("target_not_member"));
    }

    let mut tx = state.db.begin().await?;
    let tasks = sqlx::query(
        "UPDATE enterprise_tasks SET assignee_id = $3, updated_at = now() WHERE server_id = $1 AND assignee_id = $2 AND status <> 'done' AND status <> 'archived'",
    )
    .bind(server_id)
    .bind(from_uid)
    .bind(p.to_user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let projects = sqlx::query(
        "UPDATE enterprise_projects SET manager_user_id = $3 WHERE server_id = $1 AND manager_user_id = $2",
    )
    .bind(server_id)
    .bind(from_uid)
    .bind(p.to_user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let docs = sqlx::query(
        "UPDATE enterprise_documents SET owner_id = $3 WHERE server_id = $1 AND owner_id = $2 AND trashed_at IS NULL",
    )
    .bind(server_id)
    .bind(from_uid)
    .bind(p.to_user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if p.mark_offboarded {
        sqlx::query(
            "UPDATE enterprise_employees SET employment_status = 'offboarded', updated_at = now() WHERE server_id = $1 AND user_id = $2",
        )
        .bind(server_id)
        .bind(from_uid)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    audit(&state, server_id, uid, "ent_handover", &format!("{from_uid}->{}:t={tasks},p={projects},d={docs}", p.to_user_id)).await;
    emit_enterprise(&state, server_id, "hr.updated", json!({ "handover_to": p.to_user_id })).await;
    Ok(Json(json!({ "tasks": tasks, "projects": projects, "documents": docs })))
}

// ---------------------------------------------------------------- Departments

#[derive(Serialize, sqlx::FromRow)]
pub struct DepartmentItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub manager_user_id: Option<Uuid>,
    pub manager_name: Option<String>,
    pub linked_channel_id: Option<Uuid>,
    pub linked_channel_name: Option<String>,
    pub branch_name: Option<String>,
    pub member_count: i64,
    pub created_at: DateTime<Utc>,
}

pub async fn list_departments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<DepartmentItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::VIEW_HR)?;

    let rows: Vec<DepartmentItem> = sqlx::query_as(
        r#"
        SELECT d.id, d.server_id, d.parent_id, d.name, d.code, d.description,
               d.manager_user_id, u.username AS manager_name,
               d.linked_channel_id, c.name AS linked_channel_name,
               d.branch_name,
               (SELECT COUNT(*) FROM enterprise_department_members dm WHERE dm.department_id = d.id) AS member_count,
               d.created_at
        FROM enterprise_departments d
        LEFT JOIN users u ON u.id = d.manager_user_id
        LEFT JOIN channels c ON c.id = d.linked_channel_id
        WHERE d.server_id = $1
        ORDER BY d.created_at ASC
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateDepartmentReq {
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub manager_user_id: Option<Uuid>,
    pub linked_channel_id: Option<Uuid>,
    pub branch_name: Option<String>,
}

pub async fn create_department(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateDepartmentReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_DEPARTMENTS)?;

    let name = clean_req(&p.name, 1, 80, "invalid_department_name")?;
    let code = clean_opt(p.code.as_deref(), 24, "invalid_department_code")?;
    let desc = clean_opt(p.description.as_deref(), 500, "invalid_description")?;
    let branch = clean_opt(p.branch_name.as_deref(), 80, "invalid_branch")?;

    if let Some(ch_id) = p.linked_channel_id {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND server_id = $2)",
        )
        .bind(ch_id)
        .bind(server_id)
        .fetch_one(&state.db)
        .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_linked_channel"));
        }
    }

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO enterprise_departments (server_id, parent_id, name, code, description, manager_user_id, linked_channel_id, branch_name)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(server_id)
    .bind(p.parent_id)
    .bind(&name)
    .bind(code)
    .bind(desc)
    .bind(p.manager_user_id)
    .bind(p.linked_channel_id)
    .bind(branch)
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_dept_created", &name).await;
    emit_enterprise(&state, server_id, "departments.updated", json!({ "id": id })).await;
    Ok(Json(json!({ "id": id })))
}

pub async fn delete_department(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, dept_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_DEPARTMENTS)?;

    let res = sqlx::query("DELETE FROM enterprise_departments WHERE id = $1 AND server_id = $2")
        .bind(dept_id)
        .bind(server_id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("department_not_found"));
    }
    audit(&state, server_id, uid, "ent_dept_deleted", &dept_id.to_string()).await;
    emit_enterprise(&state, server_id, "departments.updated", json!({ "deleted": dept_id })).await;
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
pub struct DeptMemberReq {
    pub user_id: Uuid,
}

pub async fn add_department_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, dept_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<DeptMemberReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_DEPARTMENTS)?;

    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM enterprise_departments WHERE id = $1 AND server_id = $2)
         AND EXISTS(SELECT 1 FROM server_members WHERE server_id = $2 AND user_id = $3)",
    )
    .bind(dept_id)
    .bind(server_id)
    .bind(p.user_id)
    .fetch_one(&state.db)
    .await?;
    if !valid {
        return Err(AppError::BadRequest("invalid_department_or_member"));
    }

    sqlx::query(
        "INSERT INTO enterprise_department_members (department_id, server_id, user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(dept_id)
    .bind(server_id)
    .bind(p.user_id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE enterprise_employees SET department_id = COALESCE(department_id, $3), updated_at = now() WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(p.user_id)
    .bind(dept_id)
    .execute(&state.db)
    .await?;

    emit_enterprise(&state, server_id, "departments.updated", json!({ "dept_id": dept_id })).await;
    Ok(Json(json!({ "added": true })))
}
