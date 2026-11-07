# Technical Plan: Native Rust Desktop App (100% Pure Rust)

## 1. Kiến trúc Workspace
Cấu trúc Cargo Workspace tại root `chatw/Cargo.toml`:
- `members = ["crates/chatw-core", "crates/chatw-client", "backend"]`
- `backend` đóng vai trò `chatw-server`.
- `crates/chatw-core`: logic mã hóa E2EE, model chung.
- `crates/chatw-client`: GUI client egui.

## 2. Kế hoạch từng bước
1. Khởi tạo `crates/chatw-core`:
   - Dependencies: `p256`, `aes-gcm`, `sha2`, `hkdf`, `rand_core`, `base64`, `hex`, `serde`, `serde_json`, `uuid`, `chrono`.
   - Viết các hàm sinh khóa, derive shared secret, mã hóa tin nhắn JSON envelope, giải mã.
   - Viết unit test cho E2EE.
2. Thiết lập Root Cargo Workspace:
   - Tạo root `Cargo.toml`.
3. Xây dựng `crates/chatw-client`:
   - Dependencies: `eframe = "0.31"`, `egui = "0.31"`, `tokio`, `reqwest`, `tokio-tungstenite`, `chatw-core`.
   - Titlebar component (`titlebar.rs`).
   - Async network manager (`net/mod.rs`, `net/http.rs`, `net/ws.rs`).
   - App views (`views/login.rs`, `views/set_name.rs`, `views/chat.rs`).
   - App main controller (`app.rs`, `main.rs`).
4. Dọn dẹp thư mục `frontend/` và cập nhật các kịch bản `.bat`.
5. Kiểm thử toàn diện và đóng gói `dist-desktop/chatw.exe`.
