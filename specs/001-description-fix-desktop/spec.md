# Feature Specification: Fix Desktop Login Blank Screen & Custom Windows Titlebar

**Feature Branch**: `001-fix-desktop-login-and-titlebar`
**Created**: 2026-11-05
**Status**: Specified
**Input**: Sửa lỗi bấm đăng nhập Google bị full màn hình trắng và làm giao diện thanh trên (Titlebar) của Windows theo ảnh đính kèm.

---

## 1. User Scenarios & Acceptance Criteria

### User Story 1 - Sửa lỗi đăng nhập và cung cấp chế độ đăng nhập linh hoạt (Priority: P1)
- **Vấn đề**: Trong ứng dụng Tauri desktop, khi bấm "Tiếp tục với Google", ứng dụng điều hướng đến `/api/auth/google` tương đối dẫn đến URL nội bộ `tauri://localhost/api/auth/google` (không tồn tại), làm WebView bị crash thành trang trắng (`about:blank`). Ngoài ra, trong môi trường dev chưa có Google Client ID thật thì Google sẽ từ chối.
- **Giải pháp**:
  1. Trong Tauri desktop, các request API phải trỏ đúng về backend base URL `http://127.0.0.1:8082`.
  2. Bổ sung tính năng "Đăng nhập thử nghiệm (Dev / Demo)" tại backend `/api/auth/dev-login` khi ở môi trường local hoặc client id dev. Tạo ngay session chuẩn `chatw_session` với đầy đủ E2EE device keys để người dùng vào thẳng ứng dụng và chat được ngay.
  3. Xử lý nút đăng nhập Google: chuyển hướng đúng URL backend, có xử lý lỗi mạng / cấu hình rõ ràng, không bao giờ để webview bị trắng xóa.

**Acceptance Criteria**:
- **AC 1.1**: Bấm đăng nhập không bao giờ gây ra hiện tượng trắng xóa toàn bộ cửa sổ ứng dụng.
- **AC 1.2**: Người dùng có thể đăng nhập ngay lập tức bằng 1 chạm qua tùy chọn "Đăng nhập thử nghiệm" hoặc nhập tài khoản mẫu trong môi trường local, tự động sinh khóa E2EE và chuyển vào màn hình cài đặt tên / danh bạ chat.
- **AC 1.3**: Đăng nhập Google được điều hướng an toàn qua backend `http://127.0.0.1:8082/api/auth/google` thay vì đường dẫn chết trên `tauri://localhost`.

---

### User Story 2 - Giao diện thanh trên (Custom Titlebar) của Windows (Priority: P1)
- **Vấn đề**: Cửa sổ Windows đang dùng viền và thanh tiêu đề mặc định của hệ điều hành, vừa không liền mạch với giao diện tối giản của app, vừa thiếu thẩm mỹ như trong ảnh 2 của người dùng.
- **Giải pháp**:
  1. Tắt `decorations` trong `tauri.conf.json` (`"decorations": false`).
  2. Cấp quyền window actions trong `src-tauri/capabilities/default.json` (`allow-minimize`, `allow-toggle-maximize`, `allow-close`, `allow-start-dragging`).
  3. Tạo component `Titlebar.tsx` chuẩn phong cách Windows hiện đại:
     - Biểu tượng logo SVG + tiêu đề `chatw – Secure Messaging`.
     - Vùng kéo thả cửa sổ (`data-tauri-drag-region`), hỗ trợ double-click để phóng to / thu nhỏ.
     - Cụm 3 nút điều khiển Windows ở góc phải: Thu nhỏ (`—`), Phóng to / Khôi phục (`□`), Đóng (`✕`).
     - Hiệu ứng hover: nền sáng nhẹ cho thu nhỏ/phóng to, nền đỏ `#E81123` cho nút đóng.

**Acceptance Criteria**:
- **AC 2.1**: Thanh trên của cửa sổ hiển thị thanh tiêu đề tùy biến sắc nét, đồng bộ màu sắc và phong cách phẳng tối giản (Flat Minimalist).
- **AC 2.2**: Người dùng có thể kéo rê thanh tiêu đề để di chuyển cửa sổ, bấm nút thu nhỏ, phóng to và đóng cửa sổ hoạt động chính xác.
- **AC 2.3**: Khi chạy trên nền Web, thanh tiêu đề tự động ẩn hoặc chỉ hiển thị ở chế độ phù hợp.
