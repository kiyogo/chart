use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    models::permissions as base_perm,
    websocket::WsEvent,
};

pub mod ent_perm {
    pub const VIEW_HR: i64 = 1 << 0;
    pub const MANAGE_HR: i64 = 1 << 1;
    pub const VIEW_SENSITIVE_HR: i64 = 1 << 2;
    pub const MANAGE_DEPARTMENTS: i64 = 1 << 3;
    pub const MANAGE_PROJECTS: i64 = 1 << 4;
    pub const CREATE_TASKS: i64 = 1 << 5;
    pub const ASSIGN_TASKS: i64 = 1 << 6;
    pub const APPROVE_TASKS: i64 = 1 << 7;
    pub const VIEW_ATTENDANCE: i64 = 1 << 8;
    pub const MANAGE_ATTENDANCE: i64 = 1 << 9;
    pub const APPROVE_LEAVE: i64 = 1 << 10;
    pub const MANAGE_DOCUMENTS: i64 = 1 << 11;
    pub const APPROVE_REQUESTS: i64 = 1 << 12;
    pub const VIEW_REPORTS: i64 = 1 << 13;
    pub const EXPORT_DATA: i64 = 1 << 14;
    pub const MANAGE_SECURITY: i64 = 1 << 15;
    pub const MANAGE_ENTERPRISE: i64 = 1 << 16;
    pub const VIEW_AUDIT: i64 = 1 << 17;
    pub const MANAGE_INTEGRATIONS: i64 = 1 << 18;
    pub const MANAGE_AI: i64 = 1 << 19;
    pub const MANAGE_AUTOMATION: i64 = 1 << 20;
    pub const POST_ANNOUNCEMENTS: i64 = 1 << 21;
    pub const MANAGE_MEETINGS: i64 = 1 << 22;

    pub const DEFAULT_EMPLOYEE: i64 = VIEW_HR | CREATE_TASKS | VIEW_ATTENDANCE | VIEW_REPORTS;
    pub const ALL: i64 = (1 << 23) - 1;
}

#[derive(Clone, Debug)]
pub struct EnterpriseCtx {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub is_owner: bool,
    pub base_bits: i64,
    pub ent_bits: i64,
    pub dept_ids: Vec<Uuid>,
    pub managed_dept_ids: Vec<Uuid>,
    pub employee_status: Option<String>,
}

impl EnterpriseCtx {
    pub fn has(&self, bit: i64) -> bool {
        self.is_owner || (self.base_bits & base_perm::ADMINISTRATOR != 0) || (self.ent_bits & bit != 0)
    }

    pub fn require(&self, bit: i64) -> AppResult<()> {
        if self.has(bit) {
            Ok(())
        } else {
            Err(AppError::Forbidden("insufficient_enterprise_permissions"))
        }
    }

    pub fn is_admin(&self) -> bool {
        self.is_owner || (self.base_bits & base_perm::ADMINISTRATOR != 0) || self.has(ent_perm::MANAGE_ENTERPRISE)
    }
}

pub async fn load_ctx(db: &PgPool, server_id: Uuid, user_id: Uuid) -> AppResult<EnterpriseCtx> {
    let srv = sqlx::query("SELECT owner_id, is_enterprise FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound("server_not_found"))?;

    let owner_id: Uuid = srv.try_get("owner_id")?;
    let is_enterprise: bool = srv.try_get("is_enterprise")?;
    if !is_enterprise {
        return Err(AppError::BadRequest("enterprise_not_enabled"));
    }

    let is_owner = owner_id == user_id;
    let member_row = sqlx::query("SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?;

    if !is_owner && member_row.is_none() {
        return Err(AppError::Forbidden("not_server_member"));
    }

    let emp_status: Option<String> = sqlx::query_scalar(
        "SELECT employment_status FROM enterprise_employees WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    if !is_owner && matches!(emp_status.as_deref(), Some("suspended") | Some("offboarded")) {
        return Err(AppError::Forbidden("employee_access_revoked"));
    }

    let (base_bits, ent_bits) = if is_owner {
        (base_perm::ALL, ent_perm::ALL)
    } else {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(BIT_OR(r.permissions), 0) AS b,
                   COALESCE(BIT_OR(r.enterprise_permissions), 0) AS e
            FROM server_roles r
            WHERE r.server_id = $1
              AND (r.name = '@everyone' OR r.id IN (
                  SELECT role_id FROM server_member_roles WHERE server_id = $1 AND user_id = $2
              ))
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .fetch_one(db)
        .await?;
        let b: i64 = row.try_get("b")?;
        let e: i64 = row.try_get("e")?;
        if b & base_perm::ADMINISTRATOR != 0 {
            (base_perm::ALL, ent_perm::ALL)
        } else {
            (b, e)
        }
    };

    let dept_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT department_id FROM enterprise_department_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_all(db)
    .await?;

    let managed_dept_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM enterprise_departments WHERE server_id = $1 AND manager_user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(EnterpriseCtx {
        server_id,
        user_id,
        is_owner,
        base_bits,
        ent_bits,
        dept_ids,
        managed_dept_ids,
        employee_status: emp_status,
    })
}

pub fn clean_opt(raw: Option<&str>, max: usize, code: &'static str) -> AppResult<Option<String>> {
    let Some(s) = raw else { return Ok(None) };
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if t.chars().count() > max || t.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Err(AppError::BadRequest(code));
    }
    Ok(Some(t.to_string()))
}

pub fn clean_req(raw: &str, min: usize, max: usize, code: &'static str) -> AppResult<String> {
    let t = raw.trim();
    let len = t.chars().count();
    if len < min || len > max || t.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Err(AppError::BadRequest(code));
    }
    Ok(t.to_string())
}

pub fn valid_ymd(s: &str) -> bool {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").is_ok()
}

pub fn safe_upload_url(u: &str) -> bool {
    let t = u.trim();
    (t.starts_with("/uploads/")
        || t.starts_with("/api/attachments/")
        || t.starts_with("/api/storage/references/"))
        && !t.contains("..")
        && t.len() <= 240
        && t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '?' | '='))
}

pub fn safe_webhook_url(u: &str) -> bool {
    let t = u.trim();
    (t.starts_with("https://") || t.starts_with("http://"))
        && t.len() <= 400
        && !t.contains("169.254.169.254")
        && !t.contains("metadata.google")
}

pub fn csv_escape(cell: &str) -> String {
    let safe = if cell.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{}", cell)
    } else {
        cell.to_string()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

pub async fn emit_enterprise(state: &AppState, server_id: Uuid, kind: &str, payload: Value) {
    if let Ok(ids) = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM server_members WHERE server_id = $1",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await
    {
        state
            .ws_hub
            .send_to_users(
                &ids,
                WsEvent::EnterpriseEvent(serde_json::json!({
                    "server_id": server_id,
                    "kind": kind,
                    "payload": payload,
                    "at": Utc::now(),
                })),
            )
            .await;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EnterpriseConfigRow {
    pub server_id: Uuid,
    pub enabled_modules: Value,
    pub module_order: Value,
    pub general_settings: Value,
    pub display_settings: Value,
    pub security_policy: Value,
    pub hr_policy: Value,
    pub attendance_policy: Value,
    pub leave_policy: Value,
    pub approval_policy: Value,
    pub task_policy: Value,
    pub project_policy: Value,
    pub document_policy: Value,
    pub notification_policy: Value,
    pub ai_config: Value,
    pub privacy_policy: Value,
    pub updated_at: DateTime<Utc>,
}
