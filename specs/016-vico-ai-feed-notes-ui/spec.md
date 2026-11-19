# Spec 016: Vico AI Rebrand, Social Feed ("Mới"), Messenger Notes Bar & Command Icon Fixes

## 1. Tổng quan & Mục tiêu
Hoàn thiện nâng cấp giao diện và tính năng theo yêu cầu trực tiếp của người dùng:
1. **Đổi tên & logo Vico AI**: Tải và sử dụng logo chính thức từ `https://vico.kiyo.vn/img/favicon.svg` (`app/assets/vico-ai.svg` -> `VicoAiIcon`) thay thế toàn bộ nhãn `"Trợ lý AI"` thành **"Vico AI"** trong phân hệ doanh nghiệp, cài đặt và điều hướng chính.
2. **Bổ sung 2 mục trên cột điều hướng trái (`AppRail`)**:
   - **Mới**: Bảng tin mạng xã hội dạng Threads (`feed_posts`, `feed_likes`, hỗ trợ đăng bài, ảnh, trả lời luồng, thích, đăng lại, lọc theo bạn bè/công khai/cá nhân, kèm thanh Tin/Ghi chú ở đầu trang). Giao diện tối giản không lặp lại thanh tiêu đề thừa.
   - **Vico AI**: Giao diện **Chat trực tiếp với Vico AI** độc lập hoàn toàn với AI trong Máy chủ Doanh nghiệp (`vico_conversations`, `vico_messages`, `vico_user_configs`), hỗ trợ lưu lịch sử chat PostgreSQL, kết nối endpoint OpenAI-compatible tùy chọn và công cụ hỗ trợ nội bộ thực tế. Logo Vico AI trên thanh trái hiển thị tràn khung đầy đủ kích thước chuẩn không viền trắng hay viền xanh.
3. **Thanh Ghi chú / Tạo tin cuộn ngang ở cột Tin nhắn (`StoriesNotesBar`)**:
   - Đặt ngay dưới ô tìm kiếm cuộc trò chuyện trong danh sách **Tin nhắn**.
   - Hiển thị bong bóng suy nghĩ / trạng thái của bản thân (`Tạo tin` / `+` / `"Chia sẻ suy nghĩ"`) và của bạn bè kèm chấm xanh trực tuyến.
   - Hỗ trợ kéo ngang qua lại (mouse drag + wheel + touch swipe) và lưu/đồng bộ trạng thái thực tế xuống PostgreSQL (`/api/users/me/profile`, `/api/profile-ext/me`) phát realtime qua WebSocket (`UserPresenceUpdate`).
4. **Sửa triệt để danh sách gợi ý lệnh `/` (`Composer`)**:
   - **Đúng icon cho từng lệnh**: Mỗi lệnh (`/roll`, `/flip`, `/time`, `/calc`, `/upper`, `/sound`, `/craft`, `/coords`, `/seed`, `/neon`, `/status`, `/todo`, `/note`, `/help`, `/workspace`, `/task`, `/checkin`, `/checkout`, `/leave`, `/meet`, `/approve`, `/announce`...) hiển thị đúng vector SVG icon tương ứng với chức năng lệnh thay vì lặp lại icon đồng hồ của bot hay dùng emoji hệ thống thô.
   - **Xóa khung viền thừa & phóng to logo**: Khi gợi ý lệnh đã có logo hoặc SVG icon, loại bỏ hộp nền/viền tròn `.suggest-cmd-badge` (`has-logo`), hiển thị icon/logo to rõ ràng (`22–24px`).
   - **Đưa gợi ý lệnh sát xuống ô nhập chat hơn**: Giảm khoảng cách thừa giữa `.suggest-list` và `.composer-box`.
5. **Làm sạch nhãn `"Plugin"` thừa**:
   - Xóa tiêu đề `"Plugin"` dư ở phần đầu trang Plugin Studio (`PluginCustomPageView`) và xóa huy hiệu xanh `"Plugin"` dưới avatar trong danh sách trò chuyện & header bot.

## 2. Acceptance Criteria
- **AC1**: `VicoAiIcon` hiển thị sắc nét (SVG unique gradient ID), toàn bộ `"Trợ lý AI"` được đổi sang `"Vico AI"`.
- **AC2**: Cột điều hướng trái (`AppRail`) có đầy đủ **Mới** (`NavSection::Feed`) và **Vico AI** (`NavSection::VicoAi`) kích thước đồng đều chuẩn design system.
- **AC3**: Bảng tin **Mới** lưu dữ liệu thật trong PostgreSQL (migration `0023_social_feed.sql`), API `/api/feed` kiểm tra quyền/chặn người dùng, hỗ trợ đăng bài, thích, bình luận luồng (threads) và xóa bài viết của chính mình.
- **AC4**: Trong **Tin nhắn**, dải **Tạo tin / Ghi chú** cho phép đặt/xóa suy nghĩ (lưu database + tự hết hạn), xem & trả lời ghi chú bạn bè, kéo cuộn ngang mượt mà.
- **AC5**: Gõ `/` trong khung chat hiển thị đúng vector icon riêng biệt cho từng lệnh Plugin và Doanh nghiệp, không còn khung viền xanh/xám bao quanh khi có logo/icon, icon lớn rõ ràng và khung gợi ý nằm sát ngay trên thanh nhập.
- **AC6**: Không còn tiêu đề `"Plugin"` trống ở đầu Plugin Studio và không còn badge `"Plugin"` đè dưới avatar.
