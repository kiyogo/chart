# Plan 023: Kiến trúc & Kế hoạch Kỹ thuật Nâng cấp Máy chủ Doanh nghiệp (v2)

## 1. Backend (`web/backend/src/`)
1. **`servers/mod.rs`**:
   - Nâng cấp `handle_create_server` để dùng `privacy::auth_user` (hỗ trợ cả Cookie và Bearer session token) và bổ sung cơ chế chống tạo trùng (idempotency window 5 giây cho cùng `owner_id`, `name`, `is_enterprise`).
2. **`enterprise/models.rs`**:
   - Mở rộng `safe_upload_url` chấp nhận cả `/uploads/`, `/api/attachments/` và `/api/storage/references/` (tích hợp hoàn chỉnh với hệ thống lưu trữ CAS Spec 022).
3. **`enterprise/hr.rs` & `enterprise/mod.rs`**:
   - Bổ sung API `POST /api/servers/{id}/enterprise/employees/import` để nhập danh sách nhân sự hàng loạt có kiểm tra dữ liệu (`username` / `user_id`, `full_name`, `job_title`, `department_code`, `member_kind`) trong một transaction.
4. **`enterprise/time.rs`**:
   - Đồng bộ khóa cấu hình `grace_minutes` và `late_grace_min` trong `check_in` để chính sách đi muộn cho phép trong Cài đặt có hiệu lực 100%.
5. **`enterprise/ops.rs`**:
   - Mở rộng `update_document` hỗ trợ thêm hành động `bump_version` (cập nhật phiên bản mới cho tài liệu) và `rename` bên cạnh `pin`, `unpin`, `trash`, `restore`, `share`.

## 2. Frontend (`app/src/`)
1. **`services/enterprise.rs`**:
   - Bổ sung client helper `import_employees`, `create_form_template`, và mở rộng `list_tasks` nhận thêm bộ lọc `department_id` & `project_id`.
2. **`components/enterprise/hr_dept.rs`**:
   - `HrPanel`: Thêm sắp xếp (`Tên`, `Mã NV`, `Mới nhất`), nhập danh sách nhân sự nhanh, nút `"Giao việc"` nhanh trên từng nhân sự và xem hồ sơ.
   - `DepartmentsPanel`: Thêm chọn `"Kênh liên kết"` khi tạo phòng ban và chế độ hiển thị **Sơ đồ tổ chức** đa cấp trực quan.
3. **`components/enterprise/tasks_projects.rs`**:
   - `TasksPanel`: Thêm bộ lọc theo Phòng ban & Dự án, chọn **Mẫu công việc nhanh**, quản lý **Checklist** ngay trên thẻ công việc.
   - `ProjectsPanel`: Thêm chọn Phòng ban, Kênh liên kết, **Mẫu dự án** và quản lý **Mốc tiến độ (Milestones)**.
   - `CreateTaskFromMsgDialog`: Hỗ trợ chuyển đổi linh hoạt giữa **Tạo công việc**, **Lưu vào tài liệu** và **Phát hành thông báo nội bộ** từ tin nhắn kênh.
4. **`components/enterprise/time_leave.rs`**:
   - `CalendarPanel`: Bổ sung bộ lọc khung thời gian **Ngày / Tuần / Tháng** (`day` | `week` | `month`).
   - `MeetingsPanel`: Bổ sung chọn kênh thoại liên kết, tùy chọn phòng chờ và nút mở nhanh phòng họp.
5. **`components/enterprise/ops.rs`**:
   - `ApprovalsPanel`: Bổ sung tạo mẫu biểu mẫu nhiều cấp (`Tuần tự` / `Song song`) và nút `"Yêu cầu sửa"` (`revision`).
   - `DocumentsPanel`: Bổ sung nút `"Ghim"`, `"Phiên bản mới"`, `"Xem trước"` và lọc theo thư mục.
6. **`components/enterprise/settings.rs`**:
   - Mở rộng đầy đủ các tùy chọn cấu hình doanh nghiệp (19 nhóm chính sách) trong giao diện phẳng tối giản, có lọc nhanh các mục cài đặt.
