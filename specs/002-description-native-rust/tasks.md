# Tasks: Native Rust Desktop App (100% Pure Rust)

- [x] Task 1: Thiết lập Cargo Workspace & Xây dựng `crates/chatw-core` <!-- id: 1 -->
  - Tạo root `Cargo.toml`.
  - Tạo `crates/chatw-core/Cargo.toml` và các module crypto E2EE (`p256`, `aes-gcm`), giao thức protocol.
  - Viết unit test xác nhận tính tương thích và bảo mật E2EE.

- [x] Task 2: Xây dựng Crate Native GUI `crates/chatw-client` <!-- id: 2 -->
  - Tạo `crates/chatw-client/Cargo.toml` với `eframe`, `egui`, `tokio`, `reqwest`, `tokio-tungstenite`.
  - Triển khai `titlebar.rs` với Custom Windows Titlebar (kéo thả, Minimize, Maximize, Close đỏ).
  - Triển khai mạng bất đồng bộ `net/http.rs` và `net/ws.rs`.
  - Triển khai lưu trữ khóa và cấu hình phiên `storage.rs`.

- [x] Task 3: Triển khai Giao diện Người dùng trong `chatw-client` <!-- id: 3 -->
  - View Đăng nhập `views/login.rs` (Google OAuth & Dev Login 1 chạm).
  - View Đặt tên `views/set_name.rs`.
  - View Chat chính `views/chat.rs` (Sidebar bạn bè, tìm kiếm, phòng chat E2EE, bong bóng tin nhắn, input).

- [x] Task 4: Dọn dẹp `frontend/` và Tích hợp Bản build <!-- id: 4 -->
  - Xóa bỏ toàn bộ thư mục `frontend/` cũ.
  - Cập nhật `run-desktop.bat` và `start-all.bat`.
  - Build bản release `chatw.exe` đặt vào `dist-desktop/`.

- [x] Task 5: Kiểm thử Toàn diện & Xác thực <!-- id: 5 -->
  - Chạy test unit E2EE `cargo test -p chatw-core`.
  - Chạy test biên dịch client `cargo test -p chatw-client`.
  - Xác nhận chạy thử ứng dụng Native Client `chatw.exe` độc lập thành công.
