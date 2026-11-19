# Spec 023: Nâng cấp Toàn diện Hệ thống Máy chủ với Chế độ Doanh nghiệp (Enterprise Server Upgrade v2)

## 1. Mục tiêu & Phạm vi
Hoàn thiện và nâng cấp toàn diện **Chế độ doanh nghiệp** ngay bên trong hệ thống **Máy chủ (Servers)** hiện có của `chatw`:
- Tuyệt đối không tạo ứng dụng riêng hay hệ thống máy chủ thứ hai; giữ nguyên 100% tính năng nhắn tin cá nhân E2EE, nhóm chat, máy chủ cộng đồng thông thường và phong cách giao diện Flat Minimalist Pure CSS.
- Đảm bảo toàn bộ 42 mục yêu cầu từ tạo máy chủ, chuyển đổi máy chủ cũ, hệ thống 15 module động, nhân sự (kèm nhập danh sách & thao tác nhanh), phòng ban & sơ đồ tổ chức liên kết kênh, vai trò & 23 quyền doanh nghiệp theo phạm vi, công việc (Kanban/Danh sách/Theo hạn, checklist, mẫu công việc, tạo việc/tài liệu/thông báo từ tin nhắn), dự án (mốc tiến độ, mẫu dự án, kênh liên kết), lịch làm việc (Ngày/Tuần/Tháng), họp trực tuyến gắn kênh thoại, chấm công giờ máy chủ, nghỉ phép, biểu mẫu & phê duyệt nhiều cấp, kho tài liệu (tích hợp CAS/S3 Spec 022, phiên bản, ghim, xem trước, chia sẻ có thời hạn), thông báo nội bộ, tìm kiếm, báo cáo CSV/JSON, Vico AI, tự động hóa & cài đặt doanh nghiệp chi tiết hoạt động đồng bộ, bảo mật và được kiểm thử tự động đầu-cuối.

## 2. Tiêu chí Nghiệm thu (Acceptance Criteria)
- **AC1 (Tạo & Chuyển đổi máy chủ an toàn, chống tạo trùng)**:
  - Dialog tạo máy chủ có công tắc **Chế độ doanh nghiệp** (mặc định tắt) cùng mô tả `"Bổ sung công cụ quản lý và làm việc dành cho doanh nghiệp."`.
  - Backend `POST /api/servers` chạy trong transaction, chống tạo trùng khi nhấn liên tiếp (idempotency guard 5 giây theo `(owner_id, name, is_enterprise)`), tự khởi tạo cấu hình, vai trò, phòng ban, kênh, dự án và quy trình mẫu khi bật.
  - Bật/tắt chế độ doanh nghiệp trên máy chủ đã tồn tại giữ nguyên 100% dữ liệu cũ, kiểm tra khóa bảo toàn dữ liệu (`retention_locked`), hỗ trợ xuất toàn bộ JSON và ghi nhật ký kiểm toán.
- **AC2 (Nhân sự, Nhập danh sách, Phòng ban & Sơ đồ tổ chức)**:
  - Quản lý hồ sơ nhân sự gắn với `server_members`, tách rõ `employee`, `partner`, `guest`, `bot`; ẩn thông tin nhạy cảm nếu thiếu quyền `VIEW_SENSITIVE_HR`.
  - Hỗ trợ tìm kiếm, lọc phòng ban/trạng thái, sắp xếp, thao tác hàng loạt, bàn giao công việc & tài liệu khi nghỉ việc, nhập danh sách nhân sự nhanh (`POST /api/servers/{id}/enterprise/employees/import`), mở trò chuyện và giao việc trực tiếp từ thẻ nhân sự.
  - Phòng ban hỗ trợ cấu trúc nhiều cấp (`parent_id`), liên kết với kênh máy chủ sẵn có (`linked_channel_id`) mà không tự động mở khóa kênh riêng tư trái phép, kèm chế độ xem **Sơ đồ tổ chức** trực quan.
- **AC3 (Công việc, Dự án & Hành động doanh nghiệp từ Tin nhắn)**:
  - Hỗ trợ Kanban, Danh sách, Theo hạn; lọc theo cá nhân/được giao/quá hạn/phòng ban/dự án; hỗ trợ checklist tương tác trực tiếp, mẫu công việc nhanh, bình luận và lịch sử.
  - Từ menu hành động tin nhắn kênh, nút **Tạo công việc** mở hộp thoại gọn cho phép tạo công việc gắn tin nhắn gốc (ẩn trích dẫn nếu kênh riêng tư), hoặc lưu nhanh thành tài liệu / thông báo nội bộ.
  - Dự án hỗ trợ liên kết kênh, phòng ban, mẫu dự án và cập nhật mốc tiến độ (`milestones`).
- **AC4 (Lịch Ngày/Tuần/Tháng, Họp trực tuyến, Chấm công giờ máy chủ & Nghỉ phép)**:
  - Lịch làm việc hỗ trợ lọc theo khung thời gian **Ngày / Tuần / Tháng** và loại sự kiện.
  - Cuộc họp liên kết trực tiếp với kênh thoại, hỗ trợ phòng chờ, biên bản họp và tạo công việc sau họp.
  - Chấm công dùng `Utc::now()` của backend, đồng bộ chính xác `grace_minutes` / `late_grace_min` từ cấu hình, hỗ trợ xin điều chỉnh & phê duyệt, xuất CSV.
- **AC5 (Biểu mẫu đa cấp, Kho tài liệu CAS/S3, Thông báo, Báo cáo, AI & Cài đặt 19 nhóm)**:
  - Tạo mẫu biểu mẫu tùy chỉnh nhiều bước (tuần tự/song song), gửi đề xuất, duyệt / yêu cầu sửa / từ chối / thu hồi có kiểm tra quyền từng bước.
  - Kho tài liệu hỗ trợ thư mục, phạm vi, ghim, nâng phiên bản (`v1 -> v2`), xem trước, thùng rác/khôi phục, liên kết chia sẻ có thời hạn và tương thích URL lưu trữ CAS (`/uploads/` & `/api/attachments/` & `/api/storage/references/`).
  - Cài đặt doanh nghiệp bao phủ đầy đủ các nhóm thiết lập với thanh tìm kiếm tức thì, lưu trực tiếp các công tắc an toàn và bảo vệ thao tác nhạy cảm.
