# Tasks 023: Danh sách Tác vụ Nâng cấp Máy chủ Doanh nghiệp (v2)

- [x] **T1**: Nâng cấp Backend `servers/mod.rs` (chống tạo trùng máy chủ idempotency + hỗ trợ auth header đồng nhất) và `enterprise/{models.rs,hr.rs,time.rs,ops.rs,mod.rs}` (batch import nhân sự, đồng bộ `grace_minutes`, tích hợp URL CAS Spec 022, nâng phiên bản/đổi tên tài liệu).
- [x] **T2**: Mở rộng `app/src/services/enterprise.rs` và nâng cấp `hr_dept.rs` (nhập danh sách nhân sự, sắp xếp, sơ đồ tổ chức nhiều cấp, liên kết kênh cho phòng ban).
- [x] **T3**: Nâng cấp `tasks_projects.rs` (checklist tương tác, mẫu công việc/dự án, lọc phòng ban/dự án, mốc tiến độ dự án, hộp thoại hành động tin nhắn đa năng Công việc / Tài liệu / Thông báo).
- [x] **T4**: Nâng cấp `time_leave.rs` (lịch làm việc Ngày/Tuần/Tháng, họp trực tuyến gắn kênh thoại & phòng chờ) và `ops.rs` (tạo mẫu biểu mẫu nhiều cấp, yêu cầu sửa đơn, ghim/nâng phiên bản/xem trước tài liệu).
- [x] **T5**: Nâng cấp `settings.rs` bao phủ toàn diện các nhóm cài đặt doanh nghiệp với bộ lọc tìm kiếm tức thì.
- [x] **T6**: Bổ sung kiểm thử tích hợp Backend (`test_enterprise_server_lifecycle`) & E2E Playwright (`app/e2e/enterprise-check.mjs`), biên dịch bản Release, xác minh 100% test xanh và tự động `git commit` + `git push origin main`.
