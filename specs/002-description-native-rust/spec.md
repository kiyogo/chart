# Feature Specification: Native Rust Desktop App (100% Pure Rust)

**Feature Branch**: `002-native-rust-desktop-app`
**Status**: In Implementation
**Input**: Xóa bản app cũ (SolidJS + Tauri), tự code lại 100% bằng Rust nguyên bản với egui / eframe.

## User Stories & Acceptance Criteria

### User Story 1 - Thư viện lõi E2EE & Giao thức mạng thuần Rust (`chatw-core`) (Priority: P1)
- Xây dựng crate `chatw-core` chứa toàn bộ thuật toán mật mã đầu cuối (E2EE) tương thích với chuẩn Web Crypto API cũ:
  - Sinh cặp khóa ECDH P-256 và ECDSA P-256.
  - Bắt tay tạo Shared Secret và mã hóa/giải mã đối xứng AES-256-GCM.
  - Định nghĩa dữ liệu truyền tải (User, Message, Friend, WebSocket events).
- **AC 1.1**: Hai thực thể có thể trao đổi khóa công khai và mã hóa/giải mã tin nhắn thành công qua unit test.

### User Story 2 - Desktop Client Native GUI thuần Rust (`chatw-client`) (Priority: P1)
- Xây dựng ứng dụng Desktop bằng `egui` / `eframe`:
  - **Custom Titlebar:** Thanh tiêu đề Windows tùy biến tối màu cao 32px, có logo, tiêu đề, kéo thả (`StartDrag`), 3 nút Minimize, Maximize/Restore, Close (hover đỏ `#E81123`).
  - **Màn hình Đăng nhập:** Đăng nhập an toàn (Google OAuth mở browser hoặc Đăng nhập nhanh Dev).
  - **Màn hình Đặt tên:** Nhập username lần đầu.
  - **Màn hình Chat:**
    - Cột trái: Thông tin tài khoản, danh sách bạn bè, tìm kiếm bạn mới, danh sách lời mời kết bạn.
    - Cột phải: Khung chat, lịch sử tin nhắn mã hóa đầu cuối E2EE (bong bóng gửi xanh / nhận xám), thanh nhập tin nhắn, emoji.
  - Kết nối WebSocket chạy ngầm bằng Tokio để nhận tin nhắn và trạng thái realtime.
- **AC 2.1**: Ứng dụng chạy mượt mà, không dùng webview hay node_modules.
- **AC 2.2**: Khởi động tức thì, bộ nhớ tiêu thụ thấp (< 35MB RAM).

### User Story 3 - Chuyển đổi Cargo Workspace & Dọn dẹp Frontend cũ (Priority: P1)
- Dọn dẹp thư mục `frontend/` cũ.
- Thiết lập Cargo Workspace chuẩn gồm:
  - `crates/chatw-core`
  - `crates/chatw-client`
  - `backend` (hoặc `crates/chatw-server`)
- Cập nhật script `run-desktop.bat` và `start-all.bat` để chạy trực tiếp client Native Rust.
- **AC 3.1**: Toàn bộ dự án biên dịch thành công qua `cargo build --workspace`.
