# Tasks: Fix Desktop Login Blank Screen & Custom Windows Titlebar

- [x] Task 1: Cấu hình Tauri v2 Window Permissions & Decorations <!-- id: 1 -->
  - `decorations: false`, quyền `core:window:*`, cấp capability cho origin backend (`remote.urls`).

- [x] Task 2: Backend Dev Login Endpoint (có kiểm soát bảo mật) <!-- id: 2 -->
  - `POST /api/auth/dev-login` chỉ bật khi `DEV_LOGIN=true`; server từ chối khởi động nếu `PUBLIC_URL` không phải loopback.
  - `GET /api/auth/providers` trả về `{ google, dev }` để UI không điều hướng vào ngõ cụt.

- [x] Task 3: Sửa nguyên nhân màn hình trắng & cập nhật Login <!-- id: 3 -->
  - Nguyên nhân: trang bundled ở `http://tauri.localhost` điều hướng tới `/api/auth/google` (không tồn tại) → trắng.
  - Desktop giờ chạy dạng launcher: kiểm tra backend rồi nạp app từ chính backend origin (cookie first-party, OAuth callback hoạt động). Nếu backend tắt → màn hình "Thử lại".
  - Nút Google bị vô hiệu khi chưa cấu hình `GOOGLE_CLIENT_ID` thật.

- [x] Task 4: Component Custom Titlebar & Styling <!-- id: 4 -->
  - `Titlebar.tsx`: logo, tiêu đề, vùng kéo thả, double-click phóng to, 3 nút Minimize / Maximize-Restore / Close (hover đỏ `#E81123`).

- [x] Task 5: Kiểm thử <!-- id: 5 -->
  - `tsc` + `vite build` pass; `cargo build` + `cargo test --no-run` pass.
  - `/api/auth/providers` → `{"google":false,"dev":true}`; dev-login → 200, `/api/auth/me` trả user.
  - Build lại `dist-desktop/chatw.exe` (Tauri release) và khởi chạy.
