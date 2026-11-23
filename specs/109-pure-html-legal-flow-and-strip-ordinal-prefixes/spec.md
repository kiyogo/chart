# Spec 109: Giao diện văn bản thuần HTML không dùng CSS hộp/thẻ và xóa tiền tố "Thứ... ."

## Bối cảnh & Yêu cầu của người dùng
Người dùng yêu cầu:
> "tất cả 100% không được cái css ,... giao diện dùng html thôi nhé và xóa tất cả thứ .... . ở đầu đi nhé"

Hình ảnh đính kèm (`media_1795452912162_4bda5f86.jpg`) cho thấy:
1. Mỗi điều khoản hiện đang nằm trong các hộp thẻ (`.legal-simple-item`) có viền bo tròn, màu nền trắng, khoảng đệm thẻ và hiệu ứng đổi màu viền khi hover chuột vào (`.legal-simple-item:hover`). Người dùng yêu cầu 100% không dùng kiểu CSS hộp/thẻ này mà phải là văn bản HTML thuần túy tự nhiên (pure HTML document flow), chỉ có tiêu đề `h3` và đoạn văn `p`.
2. Ở đầu mỗi mục điều khoản dịch vụ hiện có tiền tố thứ tự: `"Thứ nhất. "`, `"Thứ hai. "`, `"Thứ ba. "`, `"Thứ tư. "`, `"Thứ năm. "`, `"Thứ sáu. "`, `"Thứ bảy. "`, `"Thứ tám. "`, `"Thứ chín. "`, `"Thứ mười. "`. Người dùng yêu cầu xóa sạch toàn bộ các tiền tố `"thứ .... ."` ở đầu này.

## Mục tiêu cụ thể
1. **Trong `app/src/components/legal.rs`**:
   - Xóa bỏ tiền tố `"Thứ nhất. "`, `"Thứ hai. "`, `"Thứ ba. "`, ... ở đầu tất cả các tiêu đề điều khoản.
   - Giữ lại tiêu đề trực diện, chuẩn mực và đồng bộ với các tài liệu khác.
2. **Trong `app/assets/main.css` & component `LegalContent`**:
   - Xóa bỏ 100% phong cách thẻ/hộp (`border: none`, `background: transparent`, `border-radius: 0`, `padding: 0`, `box-shadow: none`).
   - Xóa bỏ hoàn toàn hiệu ứng `:hover` trên các mục điều khoản.
   - Trình bày như một tài liệu văn bản HTML chuẩn mực tự nhiên: các khối tiêu đề `h3` và đoạn văn `p` kế tiếp nhau.
3. **Kiểm thử tự động bằng Playwright E2E**:
   - Kiểm tra trang đăng nhập (modal) và Cài đặt (Settings -> Giới thiệu -> Điều khoản dịch vụ).
   - Xác nhận không còn chuỗi `"Thứ nhất."` hay bất kỳ `"Thứ"` nào ở đầu mục.
   - Xác nhận không có khung viền hộp thẻ (`border: none`, `background: transparent`, không có viền hover).
   - Chụp ảnh màn hình lưu vào `verify_109/`.
4. **Tự động commit và push lên GitHub**:
   - Thực hiện commit Conventional Commits và push trực tiếp lên nhánh `main`.
