# Technical Implementation Plan: Fix Desktop Login & Custom Titlebar

## 1. Architecture Overview
- **Backend (Rust Axum)**:
  - Thêm endpoint `POST /api/auth/dev-login` trong `backend/src/auth/mod.rs` (cho phép đăng nhập tức thì với user name / email thử nghiệm khi trong môi trường dev).
  - Tự động tạo user trong database PostgreSQL và cấp cookie `chatw_session`.
- **Frontend (SolidJS + TypeScript)**:
  - `Login.tsx`: Cập nhật logic:
    - Khi ở trong Tauri: gọi `API_BASE + '/api/auth/google'`.
    - Thêm nút "Đăng nhập nhanh thử nghiệm" (Dev Login) gọi `api.auth.devLogin(...)` để người dùng vào thẳng ứng dụng không bị phụ thuộc Google Cloud Console.
  - `Titlebar.tsx`: Component thanh tiêu đề tùy biến mới:
    - Bắt sự kiện `@tauri-apps/api/window` (`appWindow.minimize()`, `appWindow.toggleMaximize()`, `appWindow.close()`).
    - Gắn `data-tauri-drag-region` trên header.
  - `App.tsx`: Nhúng `<Titlebar />` ở trên cùng của ứng dụng.
  - `main.css`: Định kiểu CSS cho `.titlebar`, `.titlebar-controls`, `.titlebar-btn`, đảm bảo hover nút đóng màu đỏ `#E81123`, nút thu nhỏ/phóng to mượt mà.
- **Tauri Configuration**:
  - `frontend/src-tauri/tauri.conf.json`: `"decorations": false`.
  - `frontend/src-tauri/capabilities/default.json`: Thêm permissions:
    - `"core:window:default"`
    - `"core:window:allow-start-dragging"`
    - `"core:window:allow-minimize"`
    - `"core:window:allow-toggle-maximize"`
    - `"core:window:allow-close"`

## 2. File Changes
1. `backend/src/auth/mod.rs` - Thêm `POST /dev-login` và handler.
2. `frontend/src/api/client.ts` - Thêm `devLogin(username)` vào `api.auth`.
3. `frontend/src/pages/Login.tsx` - Sửa redirect url và thêm form/nút đăng nhập nhanh dev.
4. `frontend/src/components/Titlebar.tsx` - Tạo mới component Titlebar tùy biến.
5. `frontend/src/App.tsx` - Đưa `Titlebar` vào layout.
6. `frontend/src/styles/main.css` - Bổ sung styles cho Titlebar và dev login.
7. `frontend/src-tauri/tauri.conf.json` - `"decorations": false`.
8. `frontend/src-tauri/capabilities/default.json` - Thêm window permissions.
