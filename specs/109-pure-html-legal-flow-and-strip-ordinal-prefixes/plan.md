# Plan 109: Kế hoạch thực hiện giao diện thuần HTML và loại bỏ tiền tố thứ tự

## 1. Thiết kế kỹ thuật
- **File**: `app/src/components/legal.rs`
  - Cập nhật hàm `get_legal_items(LegalDocKind::Terms)`:
    - `"Thứ nhất. Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!"` -> `"Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!"`
    - `"Thứ hai. Từ chối cung cấp bất kì thông tin cho bên thứ ba nào."` -> `"Từ chối cung cấp bất kì thông tin cho bên thứ ba nào"`
    - `"Thứ ba. Ẩn danh tuyệt đối."` -> `"Ẩn danh tuyệt đối"`
    - `"Thứ tư. Mã hóa đầu cuối thực chất."` -> `"Mã hóa đầu cuối thực chất"`
    - `"Thứ năm. Toàn bộ dữ liệu máy chủ luôn được mã hóa, chỉ người có mật khẩu mới mở được, khi bị tấn công hệ thống sẽ tự động ngăn chặn và bảo vệ dữ liệu."` -> `"Toàn bộ dữ liệu máy chủ luôn được mã hóa, chỉ người có mật khẩu mới mở được, khi bị tấn công hệ thống sẽ tự động ngăn chặn và bảo vệ dữ liệu"`
    - `"Thứ sáu. Zero-logs tuyệt đối."` -> `"Zero-logs tuyệt đối"`
    - `"Thứ bảy. Quyền sở hữu thuộc trọn về người dùng."` -> `"Quyền sở hữu thuộc trọn về người dùng"`
    - `"Thứ tám. Mã nguồn mở và minh bạch."` -> `"Mã nguồn mở và minh bạch"`
    - `"Thứ chín. Phi thương mại hóa và không thao túng."` -> `"Phi thương mại hóa và không thao túng"`
    - `"Thứ mười. Không khoan nhượng với phá hoại."` -> `"Không khoan nhượng với phá hoại"`
- **File**: `app/assets/main.css`
  - Loại bỏ hoàn toàn styling hộp thẻ ở `.legal-simple-item` và `.legal-simple-item:hover`:
    ```css
    .legal-simple-item {
      display: block;
      margin-bottom: 1.5rem;
      padding: 0;
      background: transparent;
      border: none;
      border-radius: 0;
      box-shadow: none;
    }
    .legal-simple-item:hover {
      background: transparent;
      border: none;
      box-shadow: none;
    }
    ```
  - Định dạng kiểu văn bản HTML thuần khiết: tiêu đề rõ ràng, đoạn văn cách dòng vừa vặn, chuẩn tài liệu đọc.

## 2. Kiểm tra & Đóng gói
- Kiểm tra biên dịch WebAssembly: `cargo check -p chatw-app --target wasm32-unknown-unknown`.
- Đóng gói bản phát hành: `./app/build-web.sh release`.
- Chạy Playwright E2E test `spec109-pure-html-legal-flow.mjs` kiểm tra và chụp ảnh xác nhận.
- Git commit và auto-push lên `origin main`.
