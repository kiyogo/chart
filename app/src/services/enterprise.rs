//! Enterprise Workspace client API & shared constants for Spec 015.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

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
    pub const ALL: i64 = (1 << 23) - 1;
}

pub const ENT_PERMS_META: [(i64, &str, &str); 23] = [
    (ent_perm::VIEW_HR, "Xem danh bạ nhân sự", "Xem danh sách thành viên, phòng ban và hồ sơ cơ bản"),
    (ent_perm::MANAGE_HR, "Quản lý nhân sự", "Thêm, sửa hồ sơ, cập nhật hàng loạt và bàn giao nghỉ việc"),
    (ent_perm::VIEW_SENSITIVE_HR, "Xem dữ liệu nhân sự nhạy cảm", "Xem email công việc, số điện thoại, loại hợp đồng và ghi chú nội bộ"),
    (ent_perm::MANAGE_DEPARTMENTS, "Quản lý phòng ban", "Tạo, xóa phòng ban và sắp xếp nhân sự"),
    (ent_perm::MANAGE_PROJECTS, "Quản lý dự án", "Tạo, chỉnh sửa dự án và mốc tiến độ"),
    (ent_perm::CREATE_TASKS, "Tạo công việc", "Tạo công việc mới và chuyển tin nhắn kênh thành công việc"),
    (ent_perm::ASSIGN_TASKS, "Giao việc và điều phối", "Xem toàn bộ công việc và phân công cho thành viên khác"),
    (ent_perm::APPROVE_TASKS, "Duyệt hoàn thành công việc", "Xem xét và phê duyệt công việc ở bước kiểm tra"),
    (ent_perm::VIEW_ATTENDANCE, "Chấm công và xem chấm công", "Thực hiện vào ca, tan ca và xem chấm công cá nhân"),
    (ent_perm::MANAGE_ATTENDANCE, "Quản lý chấm công toàn công ty", "Xem chấm công mọi nhân viên và duyệt điều chỉnh giờ công"),
    (ent_perm::APPROVE_LEAVE, "Duyệt đơn nghỉ phép", "Phê duyệt hoặc từ chối đơn xin nghỉ phép"),
    (ent_perm::MANAGE_DOCUMENTS, "Quản lý kho tài liệu", "Tải lên, ghim, đưa vào thùng rác và chia sẻ tài liệu chung"),
    (ent_perm::APPROVE_REQUESTS, "Phê duyệt đề xuất", "Xem xét và duyệt quy trình đề xuất nội bộ nhiều bước"),
    (ent_perm::VIEW_REPORTS, "Xem báo cáo thống kê", "Xem biểu đồ tiến độ, công việc và khối lượng phòng ban"),
    (ent_perm::EXPORT_DATA, "Xuất dữ liệu CSV / JSON", "Xuất bảng chấm công, báo cáo, nhật ký và toàn bộ dữ liệu"),
    (ent_perm::MANAGE_SECURITY, "Quản lý bảo mật và lưu trữ", "Thiết lập khóa bảo toàn dữ liệu và chính sách bảo mật doanh nghiệp"),
    (ent_perm::MANAGE_ENTERPRISE, "Cấu hình không gian doanh nghiệp", "Bật tắt phân hệ làm việc, giao diện và quy định chung"),
    (ent_perm::VIEW_AUDIT, "Xem nhật ký kiểm tra", "Theo dõi toàn bộ thao tác thay đổi trên máy chủ doanh nghiệp"),
    (ent_perm::MANAGE_INTEGRATIONS, "Quản lý tích hợp ngoài", "Cấu hình webhook và kết nối hệ thống bên thứ ba"),
    (ent_perm::MANAGE_AI, "Quản lý Vico AI doanh nghiệp", "Bật tắt và cấu hình tính năng tóm tắt bằng Vico AI"),
    (ent_perm::MANAGE_AUTOMATION, "Quản lý quy trình tự động hóa", "Tạo và điều khiển luật tự động chạy theo sự kiện"),
    (ent_perm::POST_ANNOUNCEMENTS, "Đăng thông báo nội bộ", "Gửi thông báo toàn công ty hoặc theo phòng ban"),
    (ent_perm::MANAGE_MEETINGS, "Quản lý tất cả cuộc họp", "Xem và điều phối toàn bộ lịch họp nội bộ"),
];

pub const MODULE_DEFS: [(&str, &str); 15] = [
    ("overview", "Tổng quan"),
    ("hr", "Nhân sự"),
    ("departments", "Phòng ban"),
    ("tasks", "Công việc"),
    ("projects", "Dự án"),
    ("calendar", "Lịch làm việc"),
    ("meetings", "Cuộc họp"),
    ("attendance", "Chấm công"),
    ("leave", "Nghỉ phép"),
    ("approvals", "Phê duyệt"),
    ("documents", "Tài liệu"),
    ("announcements", "Thông báo"),
    ("reports", "Báo cáo"),
    ("ai", "Vico AI"),
    ("automation", "Tự động hóa"),
];

pub fn module_label(key: &str) -> &'static str {
    MODULE_DEFS.iter().find(|(k, _)| *k == key).map(|(_, l)| *l).unwrap_or("Không gian làm việc")
}

pub fn vn_emp_status(s: &str) -> &'static str {
    match s {
        "active" => "Đang làm việc",
        "onboarding" => "Thử việc",
        "suspended" => "Tạm khóa",
        "offboarded" => "Đã nghỉ việc",
        _ => "Đang làm việc",
    }
}

pub fn vn_work_mode(s: &str) -> &'static str {
    match s {
        "onsite" => "Tại văn phòng",
        "remote" => "Từ xa",
        "hybrid" => "Linh hoạt",
        _ => "Tại văn phòng",
    }
}

pub fn vn_task_status(s: &str) -> &'static str {
    match s {
        "todo" => "Cần làm",
        "in_progress" => "Đang làm",
        "review" => "Chờ duyệt",
        "done" => "Hoàn thành",
        "archived" => "Lưu trữ",
        _ => "Cần làm",
    }
}

pub fn vn_project_status(s: &str) -> &'static str {
    match s {
        "planning" => "Lập kế hoạch",
        "active" => "Đang triển khai",
        "on_hold" => "Tạm dừng",
        "completed" => "Hoàn thành",
        "archived" => "Lưu trữ",
        _ => "Đang triển khai",
    }
}

pub fn vn_priority(s: &str) -> &'static str {
    match s {
        "low" => "Thấp",
        "medium" => "Trung bình",
        "high" => "Cao",
        "urgent" => "Khẩn cấp",
        _ => "Trung bình",
    }
}

pub fn vn_att_status(s: &str) -> &'static str {
    match s {
        "present" => "Đúng giờ",
        "late" => "Đi muộn",
        "early_leave" => "Về sớm",
        "remote" => "Làm từ xa",
        "overtime" => "Tăng ca",
        _ => "Đúng giờ",
    }
}

pub fn vn_approval_status(s: &str) -> &'static str {
    match s {
        "pending" => "Chờ duyệt",
        "approved" => "Đã duyệt",
        "rejected" => "Từ chối",
        "needs_revision" => "Cần bổ sung",
        "withdrawn" => "Đã thu hồi",
        _ => "Chờ duyệt",
    }
}

pub fn vn_leave_type(s: &str) -> &'static str {
    match s {
        "annual" => "Phép năm",
        "sick" => "Nghỉ ốm",
        "unpaid" => "Không lương",
        _ => "Nghỉ phép",
    }
}

pub fn vn_doc_scope(s: &str) -> &'static str {
    match s {
        "company" => "Toàn công ty",
        "department" => "Phòng ban",
        "project" => "Dự án",
        "personal" => "Cá nhân",
        _ => "Nội bộ",
    }
}

pub fn vn_category(s: &str) -> &'static str {
    match s {
        "purchase" => "Mua sắm",
        "payment" => "Thanh toán",
        "equipment" => "Thiết bị",
        "it_support" => "Hỗ trợ CNTT",
        "travel" => "Công tác",
        "access_request" => "Cấp quyền",
        "leave" => "Nghỉ phép",
        "attendance_fix" => "Điều chỉnh công",
        "project_proposal" => "Đề xuất dự án",
        _ => "Đề xuất",
    }
}

pub fn vn_meeting_type(s: &str) -> &'static str {
    match s {
        "video" => "Họp video",
        "audio" => "Họp thoại",
        "standup" => "Họp nhanh",
        "review" => "Đánh giá",
        "all_hands" => "Toàn công ty",
        _ => "Họp nội bộ",
    }
}

pub fn vn_meeting_status(s: &str) -> &'static str {
    match s {
        "scheduled" => "Đã lên lịch",
        "live" => "Đang diễn ra",
        "ended" => "Đã kết thúc",
        "cancelled" => "Đã hủy",
        _ => "Đã lên lịch",
    }
}

pub fn vn_ann_priority(s: &str) -> &'static str {
    match s {
        "normal" => "Bình thường",
        "important" => "Quan trọng",
        "urgent" => "Khẩn cấp",
        _ => "Bình thường",
    }
}

async fn call<T: DeserializeOwned>(method: &str, path: &str, body: Option<Value>) -> Result<T, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

fn enc(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

pub fn export_reports_csv_url(sid: &str) -> String {
    format!("/api/servers/{}/enterprise/reports/csv", enc(sid))
}

pub fn export_attendance_csv_url(sid: &str) -> String {
    format!("/api/servers/{}/enterprise/attendance/csv", enc(sid))
}

pub fn export_audit_csv_url(sid: &str) -> String {
    format!("/api/servers/{}/enterprise/audit/csv", enc(sid))
}

pub fn export_all_json_url(sid: &str) -> String {
    format!("/api/servers/{}/enterprise/export-all", enc(sid))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EnterpriseConfig {
    #[serde(default)]
    pub server_id: String,
    #[serde(default)]
    pub enabled_modules: Vec<String>,
    #[serde(default)]
    pub module_order: Vec<String>,
    #[serde(default)]
    pub general_settings: Value,
    #[serde(default)]
    pub display_settings: Value,
    #[serde(default)]
    pub security_policy: Value,
    #[serde(default)]
    pub hr_policy: Value,
    #[serde(default)]
    pub attendance_policy: Value,
    #[serde(default)]
    pub leave_policy: Value,
    #[serde(default)]
    pub approval_policy: Value,
    #[serde(default)]
    pub task_policy: Value,
    #[serde(default)]
    pub project_policy: Value,
    #[serde(default)]
    pub document_policy: Value,
    #[serde(default)]
    pub notification_policy: Value,
    #[serde(default)]
    pub ai_config: Value,
    #[serde(default)]
    pub privacy_policy: Value,
    #[serde(default)]
    pub retention_locked: bool,
    #[serde(default)]
    pub enterprise_disabled_at: Option<String>,
    #[serde(default)]
    pub my_permissions: i64,
    #[serde(default)]
    pub my_enterprise_permissions: i64,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(default)]
    pub department_ids: Vec<String>,
    #[serde(default)]
    pub managed_department_ids: Vec<String>,
}

impl EnterpriseConfig {
    pub fn has(&self, bit: i64) -> bool {
        self.is_owner
            || (self.my_permissions & 64 != 0)
            || (self.my_enterprise_permissions & bit != 0)
    }

    pub fn can_see_module(&self, key: &str) -> bool {
        if !self.enabled_modules.iter().any(|m| m == key) {
            return false;
        }
        match key {
            "ai" => self.ai_config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) || self.has(ent_perm::MANAGE_AI),
            "automation" => self.has(ent_perm::MANAGE_AUTOMATION) || self.has(ent_perm::MANAGE_INTEGRATIONS) || self.has(ent_perm::MANAGE_ENTERPRISE),
            "reports" => self.has(ent_perm::VIEW_REPORTS),
            _ => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EnterpriseOverview {
    #[serde(default)]
    pub my_open_tasks: i64,
    #[serde(default)]
    pub my_overdue_tasks: i64,
    #[serde(default)]
    pub total_tasks: i64,
    #[serde(default)]
    pub done_tasks: i64,
    #[serde(default)]
    pub today_meetings: i64,
    #[serde(default)]
    pub unread_announcements: i64,
    #[serde(default)]
    pub pending_approvals: i64,
    #[serde(default)]
    pub pending_leave: i64,
    #[serde(default)]
    pub my_attendance: Option<AttendanceItem>,
    #[serde(default)]
    pub active_employees: i64,
    #[serde(default)]
    pub departments_count: i64,
    #[serde(default)]
    pub active_projects: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub kind: String,
    pub module: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmployeeItem {
    pub id: String,
    pub server_id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub presence: String,
    #[serde(default)]
    pub employee_code: Option<String>,
    pub member_kind: String,
    pub full_name: String,
    #[serde(default)]
    pub work_email: Option<String>,
    #[serde(default)]
    pub work_phone: Option<String>,
    #[serde(default)]
    pub job_title: Option<String>,
    #[serde(default)]
    pub position_level: Option<String>,
    #[serde(default)]
    pub department_id: Option<String>,
    #[serde(default)]
    pub department_name: Option<String>,
    #[serde(default)]
    pub manager_user_id: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub work_location: Option<String>,
    pub work_mode: String,
    #[serde(default)]
    pub contract_type: Option<String>,
    pub employment_status: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub work_schedule: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub internal_notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DepartmentItem {
    pub id: String,
    pub server_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub manager_user_id: Option<String>,
    #[serde(default)]
    pub manager_name: Option<String>,
    #[serde(default)]
    pub linked_channel_id: Option<String>,
    #[serde(default)]
    pub linked_channel_name: Option<String>,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub member_count: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub server_id: String,
    #[serde(default)]
    pub department_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    #[serde(default)]
    pub manager_user_id: Option<String>,
    #[serde(default)]
    pub manager_name: Option<String>,
    #[serde(default)]
    pub linked_channel_id: Option<String>,
    #[serde(default)]
    pub linked_channel_name: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub task_total: i64,
    #[serde(default)]
    pub task_done: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub server_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub department_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub assignee_name: Option<String>,
    #[serde(default)]
    pub checklist: Value,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub recurrence: String,
    #[serde(default)]
    pub source_channel_id: Option<String>,
    #[serde(default)]
    pub source_message_id: Option<String>,
    #[serde(default)]
    pub source_channel_name: Option<String>,
    #[serde(default)]
    pub source_message_preview: Option<String>,
    #[serde(default)]
    pub comment_count: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskCommentItem {
    pub id: String,
    pub task_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttendanceItem {
    pub id: String,
    pub server_id: String,
    pub user_id: String,
    pub username: String,
    pub work_date: String,
    #[serde(default)]
    pub check_in_at: Option<String>,
    #[serde(default)]
    pub check_out_at: Option<String>,
    pub work_mode: String,
    pub status: String,
    #[serde(default)]
    pub late_minutes: i32,
    #[serde(default)]
    pub early_minutes: i32,
    #[serde(default)]
    pub overtime_minutes: i32,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustmentItem {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub work_date: String,
    #[serde(default)]
    pub requested_in: Option<String>,
    #[serde(default)]
    pub requested_out: Option<String>,
    pub reason: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaveBalanceItem {
    pub year: i32,
    pub leave_type: String,
    pub total_days: f64,
    pub used_days: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaveRequestItem {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub leave_type: String,
    pub start_date: String,
    pub end_date: String,
    pub half_day: String,
    pub days_count: f64,
    pub reason: String,
    pub status: String,
    #[serde(default)]
    pub decision_note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct LeaveData {
    #[serde(default)]
    pub balances: Vec<LeaveBalanceItem>,
    #[serde(default)]
    pub requests: Vec<LeaveRequestItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeetingItem {
    pub id: String,
    pub server_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub meeting_type: String,
    pub host_id: String,
    pub host_name: String,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub minutes: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormTemplateItem {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub fields: Value,
    #[serde(default)]
    pub workflow_steps: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequestItem {
    pub id: String,
    #[serde(default)]
    pub template_id: Option<String>,
    pub title: String,
    pub category: String,
    pub requester_id: String,
    pub requester_name: String,
    #[serde(default)]
    pub form_data: Value,
    pub status: String,
    pub current_step: i32,
    pub total_steps: i32,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ApprovalsData {
    #[serde(default)]
    pub templates: Vec<FormTemplateItem>,
    #[serde(default)]
    pub requests: Vec<ApprovalRequestItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FolderItem {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub scope: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocumentItem {
    pub id: String,
    #[serde(default)]
    pub folder_id: Option<String>,
    pub scope: String,
    pub owner_id: String,
    pub owner_name: String,
    pub title: String,
    pub file_url: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub pinned: bool,
    pub version: i32,
    #[serde(default)]
    pub trashed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DocumentsData {
    #[serde(default)]
    pub folders: Vec<FolderItem>,
    #[serde(default)]
    pub documents: Vec<DocumentItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnnouncementItem {
    pub id: String,
    pub scope: String,
    pub author_id: String,
    pub author_name: String,
    pub title: String,
    pub content: String,
    pub priority: String,
    pub pinned: bool,
    pub read_count: i64,
    pub read_by_me: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegrationItem {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub endpoint_url: Option<String>,
    pub has_secret: bool,
    #[serde(default)]
    pub last_tested_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkflowItem {
    pub id: String,
    pub name: String,
    pub trigger_event: String,
    pub action_type: String,
    #[serde(default)]
    pub action_config: Value,
    pub enabled: bool,
    pub run_count: i64,
    #[serde(default)]
    pub last_run_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AutomationData {
    #[serde(default)]
    pub integrations: Vec<IntegrationItem>,
    #[serde(default)]
    pub workflows: Vec<WorkflowItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnterpriseAuditItem {
    pub id: String,
    #[serde(default)]
    pub actor_name: Option<String>,
    pub action: String,
    #[serde(default)]
    pub target: Option<String>,
    pub created_at: String,
}

// Config & Dashboard & Search
pub async fn get_config(sid: &str) -> Result<EnterpriseConfig, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/config", enc(sid)), None).await
}
pub async fn update_config(sid: &str, fields: Value) -> Result<Value, ApiError> {
    call("PATCH", &format!("/api/servers/{}/enterprise/config", enc(sid)), Some(fields)).await
}
pub async fn get_overview(sid: &str) -> Result<EnterpriseOverview, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/overview", enc(sid)), None).await
}
pub async fn search(sid: &str, q: &str) -> Result<Vec<SearchResultItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/search?q={}", enc(sid), enc(q)), None).await
}
pub async fn reports(sid: &str, dept_id: Option<&str>) -> Result<Value, ApiError> {
    let qs = dept_id.filter(|s| !s.is_empty()).map(|d| format!("?department_id={}", enc(d))).unwrap_or_default();
    call("GET", &format!("/api/servers/{}/enterprise/reports{qs}", enc(sid)), None).await
}
pub async fn audit_log(sid: &str, q: &str) -> Result<Vec<EnterpriseAuditItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/audit?q={}", enc(sid), enc(q)), None).await
}

// HR & Departments
pub async fn list_employees(sid: &str, q: &str, status: &str, dept_id: &str) -> Result<Vec<EmployeeItem>, ApiError> {
    call(
        "GET",
        &format!("/api/servers/{}/enterprise/employees?q={}&status={}&department_id={}", enc(sid), enc(q), enc(status), enc(dept_id)),
        None,
    )
    .await
}
pub async fn upsert_employee(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/employees", enc(sid)), Some(body)).await
}
pub async fn bulk_employees(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/employees/bulk", enc(sid)), Some(body)).await
}
pub async fn handover_employee(sid: &str, user_id: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/employees/{}/handover", enc(sid), enc(user_id)), Some(body)).await
}
pub async fn list_departments(sid: &str) -> Result<Vec<DepartmentItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/departments", enc(sid)), None).await
}
pub async fn create_department(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/departments", enc(sid)), Some(body)).await
}
pub async fn delete_department(sid: &str, dept_id: &str) -> Result<Value, ApiError> {
    call("DELETE", &format!("/api/servers/{}/enterprise/departments/{}", enc(sid), enc(dept_id)), None).await
}
pub async fn add_department_member(sid: &str, dept_id: &str, user_id: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/departments/{}/members", enc(sid), enc(dept_id)), Some(json!({ "user_id": user_id }))).await
}

// Projects & Tasks
pub async fn list_projects(sid: &str) -> Result<Vec<ProjectItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/projects", enc(sid)), None).await
}
pub async fn create_project(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/projects", enc(sid)), Some(body)).await
}
pub async fn update_project(sid: &str, project_id: &str, body: Value) -> Result<Value, ApiError> {
    call("PATCH", &format!("/api/servers/{}/enterprise/projects/{}", enc(sid), enc(project_id)), Some(body)).await
}
pub async fn list_tasks(sid: &str, view: &str, q: &str) -> Result<Vec<TaskItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/tasks?view={}&q={}", enc(sid), enc(view), enc(q)), None).await
}
pub async fn create_task(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/tasks", enc(sid)), Some(body)).await
}
pub async fn update_task(sid: &str, task_id: &str, body: Value) -> Result<Value, ApiError> {
    call("PATCH", &format!("/api/servers/{}/enterprise/tasks/{}", enc(sid), enc(task_id)), Some(body)).await
}
pub async fn list_task_comments(sid: &str, task_id: &str) -> Result<Vec<TaskCommentItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/tasks/{}/comments", enc(sid), enc(task_id)), None).await
}
pub async fn add_task_comment(sid: &str, task_id: &str, content: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/tasks/{}/comments", enc(sid), enc(task_id)), Some(json!({ "content": content }))).await
}

// Attendance, Leave & Meetings
pub async fn list_attendance(sid: &str) -> Result<Vec<AttendanceItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/attendance", enc(sid)), None).await
}
pub async fn check_in(sid: &str, work_mode: &str, note: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/attendance/check-in", enc(sid)), Some(json!({ "work_mode": work_mode, "note": note }))).await
}
pub async fn check_out(sid: &str, note: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/attendance/check-out", enc(sid)), Some(json!({ "note": note }))).await
}
pub async fn list_adjustments(sid: &str) -> Result<Vec<AdjustmentItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/attendance/adjustments", enc(sid)), None).await
}
pub async fn request_adjustment(sid: &str, work_date: &str, requested_in: Option<&str>, requested_out: Option<&str>, reason: &str) -> Result<Value, ApiError> {
    call(
        "POST",
        &format!("/api/servers/{}/enterprise/attendance/adjustments", enc(sid)),
        Some(json!({ "work_date": work_date, "requested_in": requested_in, "requested_out": requested_out, "reason": reason })),
    )
    .await
}
pub async fn decide_adjustment(sid: &str, adj_id: &str, status: &str, note: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/attendance/adjustments/{}/decide", enc(sid), enc(adj_id)), Some(json!({ "status": status, "note": note }))).await
}
pub async fn list_leave(sid: &str) -> Result<LeaveData, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/leave", enc(sid)), None).await
}
pub async fn create_leave(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/leave", enc(sid)), Some(body)).await
}
pub async fn decide_leave(sid: &str, req_id: &str, status: &str, note: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/leave/{}/decide", enc(sid), enc(req_id)), Some(json!({ "status": status, "note": note }))).await
}
pub async fn list_meetings(sid: &str) -> Result<Vec<MeetingItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/meetings", enc(sid)), None).await
}
pub async fn create_meeting(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/meetings", enc(sid)), Some(body)).await
}
pub async fn update_meeting(sid: &str, mid: &str, body: Value) -> Result<Value, ApiError> {
    call("PATCH", &format!("/api/servers/{}/enterprise/meetings/{}", enc(sid), enc(mid)), Some(body)).await
}

// Approvals, Documents, Announcements, Integrations, Workflows & AI
pub async fn list_approvals(sid: &str) -> Result<ApprovalsData, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/approvals", enc(sid)), None).await
}
pub async fn create_form_template(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/approvals/templates", enc(sid)), Some(body)).await
}
pub async fn submit_approval(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/approvals", enc(sid)), Some(body)).await
}
pub async fn decide_approval(sid: &str, req_id: &str, decision: &str, comment: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/approvals/{}/decide", enc(sid), enc(req_id)), Some(json!({ "decision": decision, "comment": comment }))).await
}
pub async fn list_documents(sid: &str, scope: &str, trashed: bool, q: &str) -> Result<DocumentsData, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/documents?scope={}&trashed={trashed}&q={}", enc(sid), enc(scope), enc(q)), None).await
}
pub async fn create_document(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/documents", enc(sid)), Some(body)).await
}
pub async fn create_folder(sid: &str, name: &str, scope: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/documents/folders", enc(sid)), Some(json!({ "name": name, "scope": scope }))).await
}
pub async fn update_document(sid: &str, doc_id: &str, body: Value) -> Result<Value, ApiError> {
    call("PATCH", &format!("/api/servers/{}/enterprise/documents/{}", enc(sid), enc(doc_id)), Some(body)).await
}
pub async fn list_announcements(sid: &str) -> Result<Vec<AnnouncementItem>, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/announcements", enc(sid)), None).await
}
pub async fn create_announcement(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/announcements", enc(sid)), Some(body)).await
}
pub async fn mark_announcement_read(sid: &str, ann_id: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/announcements/{}/read", enc(sid), enc(ann_id)), None).await
}
pub async fn list_automation(sid: &str) -> Result<AutomationData, ApiError> {
    call("GET", &format!("/api/servers/{}/enterprise/integrations", enc(sid)), None).await
}
pub async fn create_integration(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/integrations", enc(sid)), Some(body)).await
}
pub async fn test_integration(sid: &str, integ_id: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/integrations/{}/test", enc(sid), enc(integ_id)), None).await
}
pub async fn create_workflow(sid: &str, body: Value) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/workflows", enc(sid)), Some(body)).await
}
pub async fn toggle_workflow(sid: &str, wf_id: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/workflows/{}/toggle", enc(sid), enc(wf_id)), None).await
}
pub async fn ai_summarize(sid: &str, scope: &str, target_id: Option<&str>) -> Result<Value, ApiError> {
    call("POST", &format!("/api/servers/{}/enterprise/ai/summarize", enc(sid)), Some(json!({ "scope": scope, "target_id": target_id }))).await
}
