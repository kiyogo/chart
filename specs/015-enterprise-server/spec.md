# Spec 015: Nâng cấp Hệ thống Máy chủ với Chế độ Doanh nghiệp (Enterprise Mode)

## 1. Tổng quan & Mục tiêu
Tích hợp đầy đủ hệ thống quản lý doanh nghiệp ngay bên trong **Máy chủ (Servers/Communities)** hiện tại của `chatw`:
- Giữ nguyên 100% tính năng nhắn tin cá nhân, nhóm chat, máy chủ cộng đồng thông thường, kênh thoại/văn bản/diễn đàn/sự kiện, bot, tệp và bảo mật E2EE hiện có.
- Không tạo ứng dụng riêng, không nhân đôi hệ thống thành viên/kênh/tin nhắn/cuộc gọi, không thay đổi thiết kế tổng thể của app.
- Khi tạo máy chủ hoặc chuyển đổi máy chủ cũ sang **Chế độ doanh nghiệp**, hệ thống mở rộng máy chủ đó với các phân hệ làm việc & quản trị đồng bộ từ PostgreSQL -> Rust Axum API -> RBAC/Audit/WebSocket -> Dioxus UI.

## 2. Acceptance Criteria

### AC1 — Tạo máy chủ & Chuyển đổi máy chủ cũ (An toàn, Không mất dữ liệu)
- Dialog tạo cộng đồng/máy chủ (`CommunityModal`) thêm duy nhất công tắc **Chế độ doanh nghiệp** (mặc định TẮT) với mô tả `"Bổ sung công cụ quản lý và làm việc dành cho doanh nghiệp."`.
- Backend (`POST /api/servers`) nhận `is_enterprise: Option<bool>`, chạy transaction khởi tạo máy chủ, `server_enterprise_configs`, mẫu biểu mẫu chuẩn và vai trò doanh nghiệp mặc định mà không bắt xác minh rườm rà; chống tạo trùng khi nhấn liên tiếp.
- Trong **Cài đặt máy chủ (`ServerSettingsDialog`)**: chủ sở hữu có thể bật/tắt **Chế độ doanh nghiệp**:
  - Bật: giữ nguyên `server_id`, tên, avatar, thành viên, vai trò, kênh, tin nhắn, tệp, bot, lịch sử; tự bổ sung cấu hình & dữ liệu doanh nghiệp khởi tạo.
  - Tắt: kiểm tra quyền chủ sở hữu, chặn nếu đang bật khóa lưu trữ (`retention_locked`), cảnh báo rõ ràng, không xóa dữ liệu nhân sự/tài liệu đã lưu, cho phép xuất dữ liệu và ghi audit log.

### AC2 — Cấu trúc & Hệ thống Module động
- Các module quản trị doanh nghiệp: `overview` (Tổng quan), `hr` (Nhân sự), `departments` (Phòng ban & Sơ đồ tổ chức), `tasks` (Công việc), `projects` (Dự án), `calendar` (Lịch & Họp), `attendance` (Chấm công), `leave` (Nghỉ phép), `approvals` (Biểu mẫu & Phê duyệt), `documents` (Tài liệu), `announcements` (Thông báo nội bộ), `reports` (Báo cáo & Xuất CSV), `ai` (Trợ lý AI tùy chọn), `automation` (Tự động hóa & Tích hợp).
- Quản trị viên có thể bật/tắt và sắp xếp thứ tự module trong **Cài đặt doanh nghiệp**.
- Tắt module ẩn khỏi điều hướng chính nhưng **không xóa dữ liệu** ở backend; thành viên chỉ thấy các module mà mình có quyền.

### AC3 — Phân quyền Enterprise RBAC theo Role & Phạm vi
- Mở rộng `server_roles` bằng cột `enterprise_permissions BIGINT` (tương thích ngược hoàn toàn với bit `permissions` cộng đồng `0..10`).
- Các nhóm quyền chi tiết (23 bit quyền doanh nghiệp: Xem/Quản lý Nhân sự, Xem hồ sơ nhạy cảm, Phòng ban, Dự án, Tạo/Giao/Duyệt công việc, Xem/Quản lý Chấm công, Duyệt nghỉ phép, Tài liệu, Duyệt biểu mẫu, Báo cáo, Xuất dữ liệu, Bảo mật, Cài đặt doanh nghiệp, Kiểm toán, Tích hợp, AI, Tự động hóa, Thông báo, Cuộc họp) kiểm tra hoàn toàn ở backend trên từng endpoint, tránh leo thang đặc quyền (`cannot_grant_missing_permissions`) và hỗ trợ phạm vi (toàn máy chủ, phòng ban, dự án, tài nguyên cá nhân/được gán).
- Tách bạch rõ `member_kind`: `employee` (nhân viên), `guest` (khách mời), `partner` (đối tác), `bot`. Hồ sơ nhạy cảm (SĐT, email công việc, hợp đồng, ghi chú nội bộ) chỉ trả về cho bản thân hoặc người có quyền `VIEW_SENSITIVE_HR`.

### AC4 — Nhân sự & Phòng ban liên kết Kênh/Thành viên
- Hồ sơ nhân viên gắn 1-1 với `(server_id, user_id)` trong `server_members`, hỗ trợ mã NV, chức vụ, phòng ban, chi nhánh, người quản lý, lịch làm việc, hình thức làm việc, trạng thái (`active`, `onboarding`, `suspended`, `offboarded`), tìm kiếm/lọc/sắp xếp/chọn nhiều xử lý hàng loạt, mở nhanh DM, xem sơ đồ tổ chức nhiều cấp.
- Phòng ban có thể liên kết với kênh chat sẵn có (`linked_channel_id`), người quản lý, danh sách thành viên; gán vào phòng ban không tự ý mở khóa kênh riêng tư chưa cấp quyền.

### AC5 — Công việc, Dự án & Giao việc từ Tin nhắn
- Tạo công việc từ menu hành động tin nhắn (**Tạo công việc** — chỉ xuất hiện trong máy chủ doanh nghiệp khi đủ quyền), mang nội dung tin nhắn và liên kết `source_channel_id` / `source_message_id` (tôn trọng quyền kênh riêng tư khi xem chi tiết).
- Hỗ trợ Danh sách, Kanban, Lịch dùng chung một nguồn dữ liệu thật; có checklist, bình luận, tệp đính kèm, phụ thuộc, nhãn, người theo dõi, lịch sử hoạt động, công việc con và mẫu/lặp lại.
- Dự án quản lý tiến độ, mốc (milestones), thành viên, kênh liên kết, công việc & tài liệu trực thuộc.

### AC6 — Lịch, Họp trực tuyến, Chấm công & Nghỉ phép
- Chấm công Check-in / Check-out dùng **đồng hồ máy chủ (`Utc::now()`)**, tự tính đi muộn / về sớm / tăng ca theo chính sách ca làm việc cấu hình; hỗ trợ xin điều chỉnh chấm công & phê duyệt; xuất bảng công CSV.
- Nghỉ phép cấu hình chính sách phép năm, nghỉ nửa ngày / nhiều ngày, tự động kiểm tra & cập nhật số dư phép khi duyệt/hủy/rút đơn.
- Lịch doanh nghiệp tích hợp `server_events` + đặt lịch họp trực tuyến gắn kênh thoại/cuộc gọi, chủ trì/đồng chủ trì, phòng chờ, ghi chú/biên bản họp và tạo công việc trực tiếp từ cuộc họp.

### AC7 — Biểu mẫu & Phê duyệt nhiều cấp, Tài liệu, Thông báo & Báo cáo
- Biểu mẫu linh hoạt (Nghỉ phép, Mua sắm, Thanh toán, Thiết bị, IT, Công tác, Điều chỉnh công, Đề xuất dự án, Cấp quyền, Tùy chỉnh) với quy trình duyệt tuần tự/song song nhiều cấp; backend xác thực người có quyền duyệt tại đúng bước.
- Tài liệu chia theo phạm vi Công ty / Phòng ban / Dự án / Cá nhân, có thư mục, phiên bản tài liệu, ghim/yêu thích, thùng rác & khôi phục, kiểm soát chia sẻ liên kết có thời hạn và tái sử dụng hệ thống upload tệp hiện có.
- Thông báo nội bộ theo phạm vi & mức độ (`normal`, `important`, `urgent`), ghim, theo dõi xác nhận đã đọc.
- Báo cáo tổng hợp số liệu thật (Nhân sự, Công việc, Dự án, Chấm công, Nghỉ phép, Phê duyệt, Tài liệu) & xuất file CSV thực sự; nhật ký kiểm toán (`server_audit_log`) & tìm kiếm doanh nghiệp tôn trọng phân quyền.
- Tự động hóa (Workflow trigger/action nội bộ có chống lặp & nhật ký chạy) + Tích hợp ngoài (Webhook có test thật, secret bảo mật, không giữ secret ở frontend, trạng thái cấu hình trung thực cho Calendar/GitHub/AI).

### AC8 — Cài đặt Doanh nghiệp đa lớp, Tối giản & Realtime
- Tổ chức **Cài đặt doanh nghiệp** gọn nhẹ với tìm kiếm từ khóa và phân nhóm rõ ràng (Chung, Giao diện & Module, Thành viên & Bảo mật, Nhân sự, Chấm công & Nghỉ phép, Công việc & Dự án, Phê duyệt & Tài liệu, Thông báo & AI & Tích hợp & Tự động hóa, Kiểm toán & Xuất dữ liệu & Vô hiệu hóa).
- Đồng bộ realtime qua kênh WebSocket hiện có (`WsEvent::EnterpriseEvent`).
- Giao diện đồng bộ 100% Design System ChatW (pure CSS, Duo icons từ `icons.rs`, không HTML entity rác, không dấu `...` giữ chỗ, responsive desktop/tablet/mobile).
