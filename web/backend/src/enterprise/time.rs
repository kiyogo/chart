use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::AppState,
    community::audit,
    enterprise::{
        models::{clean_opt, clean_req, csv_escape, emit_enterprise, ent_perm, load_ctx, valid_ymd},
        trigger_workflows,
    },
    error::{AppError, AppResult},
    privacy::auth_user,
};

fn vn_now() -> DateTime<FixedOffset> {
    let tz = FixedOffset::east_opt(7 * 3600).unwrap();
    Utc::now().with_timezone(&tz)
}

fn parse_hhmm(s: &str, def_h: u32, def_m: u32) -> (u32, u32) {
    let mut parts = s.trim().split(':');
    let h = parts.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(def_h).min(23);
    let m = parts.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(def_m).min(59);
    (h, m)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AttendanceItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub work_date: String,
    pub check_in_at: Option<DateTime<Utc>>,
    pub check_out_at: Option<DateTime<Utc>>,
    pub work_mode: String,
    pub status: String,
    pub late_minutes: i32,
    pub early_minutes: i32,
    pub overtime_minutes: i32,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct AttendanceQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub user_id: Option<Uuid>,
}

pub async fn list_attendance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Query(q): Query<AttendanceQuery>,
) -> AppResult<Json<Vec<AttendanceItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let can_all = ctx.is_admin() || ctx.has(ent_perm::MANAGE_ATTENDANCE);
    let target_uid = if can_all { q.user_id } else { Some(uid) };

    let rows: Vec<AttendanceItem> = sqlx::query_as(
        r#"
        SELECT a.id, a.server_id, a.user_id, u.username, a.work_date,
               a.check_in_at, a.check_out_at, a.work_mode, a.status,
               a.late_minutes, a.early_minutes, a.overtime_minutes, a.note
        FROM enterprise_attendance a
        JOIN users u ON u.id = a.user_id
        WHERE a.server_id = $1
          AND ($2::uuid IS NULL OR a.user_id = $2)
          AND ($3::text IS NULL OR a.work_date >= $3)
          AND ($4::text IS NULL OR a.work_date <= $4)
        ORDER BY a.work_date DESC, u.username ASC
        LIMIT 300
        "#,
    )
    .bind(server_id)
    .bind(target_uid)
    .bind(q.from.as_deref().filter(|s| valid_ymd(s)))
    .bind(q.to.as_deref().filter(|s| valid_ymd(s)))
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CheckReq {
    pub work_mode: Option<String>,
    pub note: Option<String>,
}

pub async fn check_in(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CheckReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let _ctx = load_ctx(&state.db, server_id, uid).await?;

    let work_mode = p.work_mode.as_deref().unwrap_or("onsite");
    if !["onsite", "remote", "hybrid"].contains(&work_mode) {
        return Err(AppError::BadRequest("invalid_work_mode"));
    }
    let note = clean_opt(p.note.as_deref(), 240, "invalid_note")?;

    // Strictly server time
    let now_utc = Utc::now();
    let local = vn_now();
    let work_date = local.format("%Y-%m-%d").to_string();

    let policy: Option<Value> = sqlx::query_scalar(
        "SELECT attendance_policy FROM server_enterprise_configs WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;

    let ws = policy
        .as_ref()
        .and_then(|v| v.get("work_start").and_then(|x| x.as_str()))
        .unwrap_or("08:30");
    let grace = policy
        .as_ref()
        .and_then(|v| v.get("late_grace_min").and_then(|x| x.as_i64()))
        .unwrap_or(10) as i32;
    let (sh, sm) = parse_hhmm(ws, 8, 30);
    let now_min = (local.hour() * 60 + local.minute()) as i32;
    let start_min = (sh * 60 + sm) as i32;
    let diff = now_min - start_min;
    let late_minutes = if diff > grace { diff } else { 0 };
    let status = if work_mode == "remote" {
        "remote"
    } else if late_minutes > 0 {
        "late"
    } else {
        "present"
    };

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO enterprise_attendance (server_id, user_id, work_date, check_in_at, work_mode, status, late_minutes, note)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (server_id, user_id, work_date) DO UPDATE SET
            check_in_at = COALESCE(enterprise_attendance.check_in_at, EXCLUDED.check_in_at),
            work_mode = EXCLUDED.work_mode,
            note = COALESCE(EXCLUDED.note, enterprise_attendance.note)
        RETURNING id
        "#,
    )
    .bind(server_id)
    .bind(uid)
    .bind(&work_date)
    .bind(now_utc)
    .bind(work_mode)
    .bind(status)
    .bind(late_minutes)
    .bind(note)
    .fetch_one(&state.db)
    .await?;

    emit_enterprise(&state, server_id, "attendance.updated", json!({ "user_id": uid, "work_date": work_date })).await;
    Ok(Json(json!({ "id": id, "work_date": work_date, "check_in_at": now_utc, "late_minutes": late_minutes, "status": status })))
}

pub async fn check_out(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CheckReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let _ctx = load_ctx(&state.db, server_id, uid).await?;

    let now_utc = Utc::now();
    let local = vn_now();
    let work_date = local.format("%Y-%m-%d").to_string();

    let policy: Option<Value> = sqlx::query_scalar(
        "SELECT attendance_policy FROM server_enterprise_configs WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    let we = policy
        .as_ref()
        .and_then(|v| v.get("work_end").and_then(|x| x.as_str()))
        .unwrap_or("17:30");
    let (eh, em) = parse_hhmm(we, 17, 30);
    let now_min = (local.hour() * 60 + local.minute()) as i32;
    let end_min = (eh * 60 + em) as i32;
    let early_minutes = if now_min < end_min { end_min - now_min } else { 0 };
    let overtime_minutes = if now_min > end_min + 15 { now_min - end_min } else { 0 };

    let note = clean_opt(p.note.as_deref(), 240, "invalid_note")?;
    let res = sqlx::query(
        r#"
        UPDATE enterprise_attendance
        SET check_out_at = $4,
            early_minutes = $5,
            overtime_minutes = $6,
            status = CASE
                WHEN $6 > 0 THEN 'overtime'
                WHEN $5 > 10 AND status = 'present' THEN 'early_leave'
                ELSE status END,
            note = COALESCE($7, note)
        WHERE server_id = $1 AND user_id = $2 AND work_date = $3
        "#,
    )
    .bind(server_id)
    .bind(uid)
    .bind(&work_date)
    .bind(now_utc)
    .bind(early_minutes)
    .bind(overtime_minutes)
    .bind(note)
    .execute(&state.db)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::BadRequest("must_check_in_first"));
    }
    emit_enterprise(&state, server_id, "attendance.updated", json!({ "user_id": uid, "work_date": work_date })).await;
    Ok(Json(json!({ "work_date": work_date, "check_out_at": now_utc, "early_minutes": early_minutes, "overtime_minutes": overtime_minutes })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AdjustmentItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub work_date: String,
    pub requested_in: Option<DateTime<Utc>>,
    pub requested_out: Option<DateTime<Utc>>,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

pub async fn list_adjustments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<AdjustmentItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    let can_all = ctx.is_admin() || ctx.has(ent_perm::MANAGE_ATTENDANCE);
    let rows: Vec<AdjustmentItem> = sqlx::query_as(
        r#"
        SELECT a.id, a.server_id, a.user_id, u.username, a.work_date,
               a.requested_in, a.requested_out, a.reason, a.status, a.created_at
        FROM enterprise_attendance_adjustments a
        JOIN users u ON u.id = a.user_id
        WHERE a.server_id = $1 AND ($2::bool OR a.user_id = $3)
        ORDER BY a.created_at DESC
        LIMIT 150
        "#,
    )
    .bind(server_id)
    .bind(can_all)
    .bind(uid)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateAdjustmentReq {
    pub work_date: String,
    pub requested_in: Option<DateTime<Utc>>,
    pub requested_out: Option<DateTime<Utc>>,
    pub reason: String,
}

pub async fn request_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateAdjustmentReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let _ctx = load_ctx(&state.db, server_id, uid).await?;
    if !valid_ymd(&p.work_date) {
        return Err(AppError::BadRequest("invalid_work_date"));
    }
    if p.requested_in.is_none() && p.requested_out.is_none() {
        return Err(AppError::BadRequest("missing_adjustment_times"));
    }
    let reason = clean_req(&p.reason, 2, 500, "invalid_reason")?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_attendance_adjustments (server_id, user_id, work_date, requested_in, requested_out, reason) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(server_id)
    .bind(uid)
    .bind(p.work_date.trim())
    .bind(p.requested_in)
    .bind(p.requested_out)
    .bind(reason)
    .fetch_one(&state.db)
    .await?;
    emit_enterprise(&state, server_id, "attendance.updated", json!({ "adjustment_id": id })).await;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct DecideReq {
    pub status: String, // "approved" | "rejected"
    pub note: Option<String>,
}

pub async fn decide_adjustment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, adj_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<DecideReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_ATTENDANCE)?;
    if !["approved", "rejected"].contains(&p.status.as_str()) {
        return Err(AppError::BadRequest("invalid_decision"));
    }

    let mut tx = state.db.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, String, Option<DateTime<Utc>>, Option<DateTime<Utc>>, String)>(
        "SELECT user_id, work_date, requested_in, requested_out, status FROM enterprise_attendance_adjustments WHERE id = $1 AND server_id = $2",
    )
    .bind(adj_id)
    .bind(server_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("adjustment_not_found"))?;

    if row.4 != "pending" {
        return Err(AppError::BadRequest("already_decided"));
    }

    sqlx::query(
        "UPDATE enterprise_attendance_adjustments SET status = $3, reviewer_id = $4, reviewed_at = now() WHERE id = $1 AND server_id = $2",
    )
    .bind(adj_id)
    .bind(server_id)
    .bind(&p.status)
    .bind(uid)
    .execute(&mut *tx)
    .await?;

    if p.status == "approved" {
        sqlx::query(
            r#"
            INSERT INTO enterprise_attendance (server_id, user_id, work_date, check_in_at, check_out_at, status, verified_by, note)
            VALUES ($1, $2, $3, $4, $5, 'present', $6, 'Điều chỉnh công được duyệt')
            ON CONFLICT (server_id, user_id, work_date) DO UPDATE SET
                check_in_at = COALESCE($4, enterprise_attendance.check_in_at),
                check_out_at = COALESCE($5, enterprise_attendance.check_out_at),
                verified_by = $6,
                status = 'present'
            "#,
        )
        .bind(server_id)
        .bind(row.0)
        .bind(&row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    audit(&state, server_id, uid, "ent_att_adj_decided", &format!("{adj_id}:{}", p.status)).await;
    emit_enterprise(&state, server_id, "attendance.updated", json!({ "adj_id": adj_id })).await;
    Ok(Json(json!({ "status": p.status })))
}

pub async fn export_attendance_csv(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Response> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::EXPORT_DATA)?;

    let rows: Vec<AttendanceItem> = sqlx::query_as(
        r#"
        SELECT a.id, a.server_id, a.user_id, u.username, a.work_date,
               a.check_in_at, a.check_out_at, a.work_mode, a.status,
               a.late_minutes, a.early_minutes, a.overtime_minutes, a.note
        FROM enterprise_attendance a
        JOIN users u ON u.id = a.user_id
        WHERE a.server_id = $1
        ORDER BY a.work_date DESC, u.username ASC
        LIMIT 2000
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let mut csv = String::from("\u{FEFF}Ngay,Nhan_vien,Hinh_thuc,Trang_thai,Check_in,Check_out,Di_muon_phut,Ve_som_phut,Tang_ca_phut,Ghi_chu\n");
    for r in rows {
        let cin = r.check_in_at.map(|d| d.to_rfc3339()).unwrap_or_default();
        let cout = r.check_out_at.map(|d| d.to_rfc3339()).unwrap_or_default();
        let line = [
            csv_escape(&r.work_date),
            csv_escape(&r.username),
            csv_escape(&r.work_mode),
            csv_escape(&r.status),
            csv_escape(&cin),
            csv_escape(&cout),
            r.late_minutes.to_string(),
            r.early_minutes.to_string(),
            r.overtime_minutes.to_string(),
            csv_escape(r.note.as_deref().unwrap_or("")),
        ]
        .join(",");
        csv.push_str(&line);
        csv.push('\n');
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"cham_cong.csv\"")
        .body(axum::body::Body::from(csv))
        .unwrap())
}

// ---------------------------------------------------------------- Leave

#[derive(Serialize, sqlx::FromRow)]
pub struct LeaveBalanceItem {
    pub year: i32,
    pub leave_type: String,
    pub total_days: f64,
    pub used_days: f64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct LeaveRequestItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub leave_type: String,
    pub start_date: String,
    pub end_date: String,
    pub half_day: String,
    pub days_count: f64,
    pub reason: String,
    pub status: String,
    pub approver_id: Option<Uuid>,
    pub decision_note: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn list_leave(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let year = Utc::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    sqlx::query(
        r#"
        INSERT INTO enterprise_leave_balances (server_id, user_id, year, leave_type, total_days, used_days)
        VALUES ($1, $2, $3, 'annual', 12.0, 0.0), ($1, $2, $3, 'sick', 5.0, 0.0)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(server_id)
    .bind(uid)
    .bind(year)
    .execute(&state.db)
    .await?;

    let balances: Vec<LeaveBalanceItem> = sqlx::query_as(
        "SELECT year, leave_type, total_days, used_days FROM enterprise_leave_balances WHERE server_id = $1 AND user_id = $2 AND year = $3 ORDER BY leave_type",
    )
    .bind(server_id)
    .bind(uid)
    .bind(year)
    .fetch_all(&state.db)
    .await?;

    let can_all = ctx.is_admin() || ctx.has(ent_perm::APPROVE_LEAVE);
    let requests: Vec<LeaveRequestItem> = sqlx::query_as(
        r#"
        SELECT l.id, l.server_id, l.user_id, u.username, l.leave_type,
               l.start_date, l.end_date, l.half_day, l.days_count, l.reason,
               l.status, l.approver_id, l.decision_note, l.decided_at, l.created_at
        FROM enterprise_leave_requests l
        JOIN users u ON u.id = l.user_id
        WHERE l.server_id = $1 AND ($2::bool OR l.user_id = $3)
        ORDER BY l.created_at DESC
        LIMIT 200
        "#,
    )
    .bind(server_id)
    .bind(can_all)
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "balances": balances, "requests": requests })))
}

#[derive(Deserialize)]
pub struct CreateLeaveReq {
    pub leave_type: String,
    pub start_date: String,
    pub end_date: String,
    pub half_day: Option<String>,
    pub reason: String,
}

pub async fn create_leave_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateLeaveReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let _ctx = load_ctx(&state.db, server_id, uid).await?;

    let lt = clean_req(&p.leave_type, 1, 32, "invalid_leave_type")?;
    let sd = NaiveDate::parse_from_str(p.start_date.trim(), "%Y-%m-%d").map_err(|_| AppError::BadRequest("invalid_start_date"))?;
    let ed = NaiveDate::parse_from_str(p.end_date.trim(), "%Y-%m-%d").map_err(|_| AppError::BadRequest("invalid_end_date"))?;
    if ed < sd {
        return Err(AppError::BadRequest("end_before_start"));
    }
    let hd = p.half_day.as_deref().unwrap_or("full");
    if !["full", "morning", "afternoon"].contains(&hd) {
        return Err(AppError::BadRequest("invalid_half_day"));
    }
    let span_days = (ed - sd).num_days() + 1;
    let days_count = if span_days == 1 && hd != "full" { 0.5 } else { span_days as f64 };
    if days_count > 90.0 {
        return Err(AppError::BadRequest("leave_range_too_large"));
    }
    let reason = clean_req(&p.reason, 2, 500, "invalid_reason")?;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO enterprise_leave_requests (server_id, user_id, leave_type, start_date, end_date, half_day, days_count, reason) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
    )
    .bind(server_id)
    .bind(uid)
    .bind(&lt)
    .bind(p.start_date.trim())
    .bind(p.end_date.trim())
    .bind(hd)
    .bind(days_count)
    .bind(reason)
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_leave_created", &format!("{lt}:{days_count}d")).await;
    emit_enterprise(&state, server_id, "leave.updated", json!({ "id": id })).await;
    Ok(Json(json!({ "id": id, "days_count": days_count })))
}

pub async fn decide_leave_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, req_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<DecideReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let mut tx = state.db.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, String, String, f64, String)>(
        "SELECT user_id, leave_type, start_date, days_count, status FROM enterprise_leave_requests WHERE id = $1 AND server_id = $2",
    )
    .bind(req_id)
    .bind(server_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("leave_request_not_found"))?;

    if row.4 != "pending" {
        return Err(AppError::BadRequest("already_decided"));
    }

    if p.status == "withdrawn" {
        if row.0 != uid {
            return Err(AppError::Forbidden("only_requester_can_withdraw"));
        }
    } else if ["approved", "rejected"].contains(&p.status.as_str()) {
        ctx.require(ent_perm::APPROVE_LEAVE)?;
        if row.0 == uid && !ctx.is_owner {
            return Err(AppError::Forbidden("cannot_approve_own_leave"));
        }
    } else {
        return Err(AppError::BadRequest("invalid_decision"));
    }

    let note = clean_opt(p.note.as_deref(), 240, "invalid_note")?;
    sqlx::query(
        "UPDATE enterprise_leave_requests SET status = $3, approver_id = $4, decision_note = $5, decided_at = now() WHERE id = $1 AND server_id = $2",
    )
    .bind(req_id)
    .bind(server_id)
    .bind(&p.status)
    .bind(uid)
    .bind(&note)
    .execute(&mut *tx)
    .await?;

    if p.status == "approved" {
        let yr: i32 = row.2.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
        sqlx::query(
            r#"
            INSERT INTO enterprise_leave_balances (server_id, user_id, year, leave_type, total_days, used_days)
            VALUES ($1, $2, $3, $4, 12.0, $5)
            ON CONFLICT (server_id, user_id, year, leave_type) DO UPDATE SET
                used_days = enterprise_leave_balances.used_days + EXCLUDED.used_days
            "#,
        )
        .bind(server_id)
        .bind(row.0)
        .bind(yr)
        .bind(&row.1)
        .bind(row.3)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    audit(&state, server_id, uid, "ent_leave_decided", &format!("{req_id}:{}", p.status)).await;
    emit_enterprise(&state, server_id, "leave.updated", json!({ "id": req_id, "status": p.status })).await;

    let state_bg = state.clone();
    let st_copy = p.status.clone();
    tokio::spawn(async move {
        trigger_workflows(&state_bg, server_id, "leave_decided", json!({ "leave_id": req_id, "status": st_copy, "user_id": row.0 }), 0).await;
    });
    Ok(Json(json!({ "status": p.status })))
}

// ---------------------------------------------------------------- Meetings & Calendar

#[derive(Serialize, sqlx::FromRow)]
pub struct MeetingItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub department_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub meeting_type: String,
    pub host_id: Uuid,
    pub host_name: String,
    pub co_host_ids: Vec<Uuid>,
    pub participant_ids: Vec<Uuid>,
    pub guest_emails: Vec<String>,
    pub waiting_room: bool,
    pub recording_allowed: bool,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub timezone: String,
    pub recurrence: String,
    pub notes: String,
    pub minutes: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

pub async fn list_meetings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<MeetingItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    let is_all = ctx.is_admin() || ctx.has(ent_perm::MANAGE_MEETINGS);
    let rows: Vec<MeetingItem> = sqlx::query_as(
        r#"
        SELECT m.id, m.server_id, m.department_id, m.project_id, m.channel_id, m.title,
               m.description, m.meeting_type, m.host_id, u.username AS host_name,
               m.co_host_ids, m.participant_ids, m.guest_emails, m.waiting_room,
               m.recording_allowed, m.starts_at, m.ends_at, m.timezone, m.recurrence,
               m.notes, m.minutes, m.status, m.created_at
        FROM enterprise_meetings m
        JOIN users u ON u.id = m.host_id
        WHERE m.server_id = $1
          AND ($2::bool OR m.host_id = $3 OR $3 = ANY(m.co_host_ids) OR $3 = ANY(m.participant_ids)
               OR (m.department_id IS NOT NULL AND m.department_id = ANY($4))
               OR (m.department_id IS NULL AND cardinality(m.participant_ids) = 0))
        ORDER BY m.starts_at ASC
        LIMIT 150
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
pub struct CreateMeetingReq {
    pub title: String,
    pub description: Option<String>,
    pub department_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub meeting_type: Option<String>,
    pub participant_ids: Option<Vec<Uuid>>,
    pub guest_emails: Option<Vec<String>>,
    pub waiting_room: Option<bool>,
    pub recording_allowed: Option<bool>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub recurrence: Option<String>,
}

pub async fn create_meeting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<CreateMeetingReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;
    ctx.require(ent_perm::MANAGE_MEETINGS)?;

    let title = clean_req(&p.title, 1, 120, "invalid_meeting_title")?;
    let desc = clean_opt(p.description.as_deref(), 1500, "invalid_description")?;
    let mtype = p.meeting_type.as_deref().unwrap_or("video");
    if !["audio", "video", "standup", "review", "all_hands"].contains(&mtype) {
        return Err(AppError::BadRequest("invalid_meeting_type"));
    }
    let rec = p.recurrence.as_deref().unwrap_or("none");
    if !["none", "daily", "weekly", "monthly"].contains(&rec) {
        return Err(AppError::BadRequest("invalid_recurrence"));
    }
    let ends_at = p.ends_at.unwrap_or_else(|| p.starts_at + Duration::minutes(45));
    if ends_at <= p.starts_at {
        return Err(AppError::BadRequest("invalid_meeting_time"));
    }
    if let Some(ch) = p.channel_id {
        let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND server_id = $2)")
            .bind(ch)
            .bind(server_id)
            .fetch_one(&state.db)
            .await?;
        if !ok {
            return Err(AppError::BadRequest("invalid_channel"));
        }
    }

    let conflict: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM enterprise_meetings WHERE server_id = $1 AND host_id = $2 AND status <> 'cancelled' AND starts_at < $4 AND ends_at > $3)",
    )
    .bind(server_id)
    .bind(uid)
    .bind(p.starts_at)
    .bind(ends_at)
    .fetch_one(&state.db)
    .await?;

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO enterprise_meetings (
            server_id, department_id, project_id, channel_id, title, description,
            meeting_type, host_id, participant_ids, guest_emails, waiting_room,
            recording_allowed, starts_at, ends_at, recurrence
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
        RETURNING id
        "#,
    )
    .bind(server_id)
    .bind(p.department_id)
    .bind(p.project_id)
    .bind(p.channel_id)
    .bind(&title)
    .bind(desc)
    .bind(mtype)
    .bind(uid)
    .bind(p.participant_ids.unwrap_or_default())
    .bind(p.guest_emails.unwrap_or_default())
    .bind(p.waiting_room.unwrap_or(false))
    .bind(p.recording_allowed.unwrap_or(false))
    .bind(p.starts_at)
    .bind(ends_at)
    .bind(rec)
    .fetch_one(&state.db)
    .await?;

    audit(&state, server_id, uid, "ent_meeting_created", &title).await;
    emit_enterprise(&state, server_id, "meetings.updated", json!({ "id": id })).await;
    Ok(Json(json!({ "id": id, "conflict_warning": conflict })))
}

#[derive(Deserialize)]
pub struct UpdateMeetingReq {
    pub notes: Option<String>,
    pub minutes: Option<String>,
    pub status: Option<String>,
}

pub async fn update_meeting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, mid)): Path<(Uuid, Uuid)>,
    Json(p): Json<UpdateMeetingReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ctx = load_ctx(&state.db, server_id, uid).await?;

    let host_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT host_id FROM enterprise_meetings WHERE id = $1 AND server_id = $2",
    )
    .bind(mid)
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    let host_id = host_id.ok_or(AppError::NotFound("meeting_not_found"))?;
    if host_id != uid && !ctx.has(ent_perm::MANAGE_MEETINGS) {
        return Err(AppError::Forbidden("insufficient_meeting_permissions"));
    }
    if let Some(ref st) = p.status {
        if !["scheduled", "live", "ended", "cancelled"].contains(&st.as_str()) {
            return Err(AppError::BadRequest("invalid_meeting_status"));
        }
    }
    sqlx::query(
        "UPDATE enterprise_meetings SET notes = COALESCE($3, notes), minutes = COALESCE($4, minutes), status = COALESCE($5, status) WHERE id = $1 AND server_id = $2",
    )
    .bind(mid)
    .bind(server_id)
    .bind(clean_opt(p.notes.as_deref(), 4000, "invalid_notes")?)
    .bind(clean_opt(p.minutes.as_deref(), 4000, "invalid_minutes")?)
    .bind(p.status.as_deref())
    .execute(&state.db)
    .await?;
    emit_enterprise(&state, server_id, "meetings.updated", json!({ "id": mid })).await;
    Ok(Json(json!({ "updated": true })))
}
