# Implementation Plan — Spec 015: Hệ thống Máy chủ Doanh nghiệp

## 1. Kiến trúc & Nguyên tắc triển khai
- **Backend**: Mở rộng `web/backend` (Axum 0.8 + SQLx 0.9 + PostgreSQL 18 + WsHub).
- **Frontend**: Mở rộng `app/` (Dioxus 0.7 WASM/Desktop + `main.css` design tokens).
- **Tái sử dụng hạ tầng hiện có**:
  - Dùng chung `servers`, `server_members`, `server_roles`, `channels`, `server_events`, `server_audit_log`, `/api/media` (`uploads`), WebSocket `WsHub`, và giao diện `ServerSettingsDialog`, `CommunityModal`, `NavSection::Server`.

## 2. Database Migration (`web/backend/migrations/0022_enterprise_servers.sql`)
- Mở rộng bảng `servers` (`is_enterprise`, `enterprise_disabled_at`, `retention_locked`) và `server_roles` (`enterprise_permissions BIGINT`).
- Bổ sung nhóm bảng liên kết chặt với `servers(id) ON DELETE CASCADE` và chỉ mục tối ưu `(server_id, ...)`:
  1. `server_enterprise_configs`: toàn bộ cấu hình module, hiển thị, bảo mật, HR, chấm công, nghỉ phép, công việc, dự án, phê duyệt, tài liệu, thông báo, AI, quyền riêng tư.
  2. `enterprise_departments`, `enterprise_department_members`: phòng ban đa cấp, liên kết kênh, người quản lý.
  3. `enterprise_employees`: hồ sơ nhân viên gắn `(server_id, user_id)`, phân tách `employee`/`guest`/`partner`/`bot`.
  4. `enterprise_projects`, `enterprise_tasks`, `enterprise_task_comments`, `enterprise_task_activity`: quản lý công việc/dự án và liên kết nguồn tin nhắn.
  5. `enterprise_meetings`: lịch họp trực tuyến gắn kênh thoại, người tham dự, biên bản họp.
  6. `enterprise_attendance`, `enterprise_attendance_adjustments`: chấm công theo giờ máy chủ & điều chỉnh công.
  7. `enterprise_leave_balances`, `enterprise_leave_requests`: số dư & đơn nghỉ phép.
  8. `enterprise_form_templates`, `enterprise_approval_requests`, `enterprise_approval_decisions`: biểu mẫu & duyệt nhiều cấp.
  9. `enterprise_folders`, `enterprise_documents`, `enterprise_document_versions`, `enterprise_document_shares`: kho tài liệu phân tầng, phiên bản, thùng rác, chia sẻ có hạn.
  10. `enterprise_announcements`, `enterprise_announcement_reads`: thông báo nội bộ & thống kê đã đọc.
  11. `enterprise_integrations`, `enterprise_workflows`, `enterprise_workflow_runs`: tích hợp webhook/dịch vụ & tự động hóa sự kiện.

## 3. Cấu trúc Backend (`web/backend/src/enterprise/`)
- `models.rs`: Struct request/response, bitmask quyền doanh nghiệp `enterprise_perms`, helper kiểm tra quyền & phạm vi dữ liệu.
- `mod.rs`: Router `/api/servers/{id}/enterprise/...`, logic khởi tạo & chuyển đổi chế độ doanh nghiệp (transaction an toàn), cấu hình chi tiết, tìm kiếm doanh nghiệp, báo cáo & xuất CSV, nhật ký kiểm toán, kích hoạt workflow an toàn, phát sự kiện `WsEvent::EnterpriseEvent`.
- `hr.rs`: Nhân viên (CRUD, mời/gán thành viên, xử lý hàng loạt, lọc nhạy cảm theo `VIEW_SENSITIVE_HR`), Phòng ban & Sơ đồ tổ chức.
- `work.rs`: Công việc (List/Kanban/Calendar, tạo từ tin nhắn với kiểm tra quyền kênh, checklist, comment, activity, automation trigger), Dự án (milestones, thành viên, kênh liên kết).
- `time.rs`: Chấm công (Check-in/out server time, tính muộn/sớm/OT, điều chỉnh công, bảng công CSV), Nghỉ phép (quy tắc số dư, tạo/rút/duyệt/từ chối), Lịch & Cuộc họp.
- `ops.rs`: Biểu mẫu & Phê duyệt nhiều cấp, Tài liệu (thư mục, upload, phiên bản, thùng rác/khôi phục, link chia sẻ hết hạn), Thông báo nội bộ + đọc xác nhận, Tích hợp (Webhook test thực tế), AI tóm tắt (từ chối rõ ràng khi chưa cấu hình/E2EE hoặc dùng cấu hình hợp lệ).
- Cập nhật `servers/mod.rs` & `servers/roles.rs`: hỗ trợ `is_enterprise` trong `CreateServerRequest` & `ServerItem` & `ServerSettings`, truyền `enterprise_permissions` trong role & `my-permissions`.

## 4. Cấu trúc Frontend (`app/src/`)
- `models.rs` & `services/enterprise.rs`: Kiểu dữ liệu + client HTTP API đầy đủ cho mọi endpoint enterprise.
- `components/icons.rs`: Bổ sung icon Duo thống nhất (`BuildingIcon`, `BriefcaseIcon`, `ListChecksIcon`, `CalendarIcon`, `FileTextIcon`, `ClipboardCheckIcon`, `BarChartIcon`, `NetworkIcon`, `EditIcon`, `PlugIcon`).
- `components/navigation/modals.rs`: Thêm switch **Chế độ doanh nghiệp** (mặc định OFF) vào `CommunityModal`.
- `components/server_settings.rs`: Bật/tắt chế độ doanh nghiệp trong Tổng quan, cấp quyền doanh nghiệp trong Vai trò, tab **Cài đặt doanh nghiệp**.
- `components/navigation/sidebar.rs` & `pages/chat.rs`: Hiển thị thanh điều hướng module doanh nghiệp thông minh theo module đang bật + quyền người dùng; hỗ trợ mở `EnterpriseWorkspace` trong `main-stage`; xử lý `WsEvent::EnterpriseEvent`; bổ sung hành động **Tạo công việc** trong `MessageList`.
- `components/enterprise/`:
  - `mod.rs`: Router module doanh nghiệp, tìm kiếm nhanh & modal tạo công việc từ tin nhắn.
  - `overview.rs`: Tổng quan điều hành theo quyền & check-in nhanh.
  - `hr_dept.rs`: Nhân sự, hồ sơ, thao tác hàng loạt & Phòng ban/Sơ đồ tổ chức.
  - `tasks_projects.rs`: Công việc (Danh sách / Kanban / Lịch) & Dự án.
  - `time_leave.rs`: Chấm công, Nghỉ phép & Lịch họp.
  - `ops.rs`: Phê duyệt biểu mẫu, Tài liệu, Thông báo nội bộ, Báo cáo CSV, Kiểm toán & AI.
  - `settings.rs`: Cài đặt doanh nghiệp đa nhóm có thanh tìm kiếm & cấu hình từng module.
- `assets/main.css`: Bổ sung style gọn nhẹ, dùng đúng CSS variables của ChatW, hỗ trợ responsive mobile/tablet/desktop.

## 5. Kiểm thử
- Backend integration test (`web/backend/tests/`): Tạo máy chủ thường vs doanh nghiệp, chuyển đổi máy chủ cũ không mất dữ liệu, kiểm tra RBAC & chống leo thang đặc quyền & ẩn trường nhạy cảm, tạo task từ tin nhắn có kiểm tra kênh riêng tư, check-in/out & leave balance, approval workflow nhiều cấp, document share expiry, xuất CSV, audit log.
- Frontend check (`cargo check --target wasm32-unknown-unknown`, `cargo test -p chatw-app --lib`, `bash app/build-web.sh`) + kiểm thử API thực tế trên máy chủ đang chạy.
