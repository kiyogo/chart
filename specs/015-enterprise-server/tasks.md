# Tasks — Spec 015: Hệ thống Máy chủ Doanh nghiệp

- [x] T001 Tạo migration `web/backend/migrations/0022_enterprise_servers.sql` và áp dụng lên PostgreSQL (`servers.is_enterprise`, `server_roles.enterprise_permissions`, toàn bộ bảng doanh nghiệp & index).
- [x] T002 Mở rộng `web/backend/src/models.rs`, `servers/mod.rs`, `servers/roles.rs`, `websocket/mod.rs` để hỗ trợ `is_enterprise`, chuyển đổi an toàn máy chủ cũ sang/tắt doanh nghiệp, `enterprise_permissions`, và `WsEvent::EnterpriseEvent`.
- [x] T003 Triển khai backend module `web/backend/src/enterprise/` (`mod.rs`, `models.rs`, `hr.rs`, `work.rs`, `time.rs`, `ops.rs`) và gắn vào `web/backend/src/main.rs` & `lib.rs` với kiểm tra RBAC, phạm vi quyền, validation, audit log, workflow trigger, xuất CSV và broadcast realtime.
- [x] T004 Mở rộng `app/src/models.rs`, `app/src/services/api.rs`, `app/src/services/enterprise.rs`, `app/src/components/icons.rs`.
- [x] T005 Tích hợp checkbox/switch **Chế độ doanh nghiệp** vào `CommunityModal` (`modals.rs`), chuyển đổi & cài đặt quyền doanh nghiệp + `EnterpriseSettings` vào `ServerSettingsDialog` (`server_settings.rs`).
- [x] T006 Tích hợp điều hướng module doanh nghiệp vào `sidebar.rs`, `pages/chat.rs`, hành động **Tạo công việc** vào `messages.rs`, và xử lý realtime `WsEvent::EnterpriseEvent`.
- [x] T007 Xây dựng toàn bộ UI module doanh nghiệp tại `app/src/components/enterprise/` (`mod.rs`, `overview.rs`, `hr_dept.rs`, `tasks_projects.rs`, `time_leave.rs`, `ops.rs`, `settings.rs`) và CSS đồng bộ tại `app/assets/main.css`.
- [x] T008 Viết và chạy integration test backend, unit test frontend, biên dịch WASM + release/dev backend, kiểm tra live server `127.0.0.1:8085`, cập nhật `tasks.md` và push lên `main`.
