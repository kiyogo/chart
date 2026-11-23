# Spec 108: Xóa các chi tiết rườm rà ở Tùy chọn nhà phát triển (Remove Developer Options Decorations)

## Bối cảnh & Yêu cầu của người dùng
Người dùng yêu cầu xóa các thành phần trang trí không cần thiết trong phần cài đặt Chế độ nhà phát triển (Settings -> Giới thiệu / About):
> "xóa cho tôi "Tùy chọn nhà phát triển"
> ""Hiển thị mục Nhà phát triển gồm xuất dữ liệu nâng cao, khóa mã hóa, đồng bộ khôi phục, tiện ích và chẩn đoán." và xóa icon và xóa thanh xám và xóa Đang bật
> ,va xóa mở nhà phát triểu đi nhé"

Hình ảnh người dùng đính kèm (`media_1795452254566_2955b3ff.jpg`) hiển thị:
1. Tiêu đề nhóm: `"Tùy chọn nhà phát triển"` (`settings-group-title`)
2. Thẻ cài đặt:
   - Icon tia chớp: `BoltIcon` (`about-dev-icon`)
   - Dòng mô tả dài: `"Hiển thị mục Nhà phát triển gồm xuất dữ liệu nâng cao, khóa mã hóa, đồng bộ khôi phục, tiện ích và chẩn đoán."` (`about-dev-hint`)
   - Đường viền/thanh phân cách màu xám: `border-top` trên `about-dev-footer`
   - Huy hiệu trạng thái: `"Đang bật"` (`tag-active` / `cw-live-dot`)
   - Nút hành động: `"Mở Nhà phát triển"` (`btn-secondary` gọi `on_open_developer`)

## Mục tiêu
1. Trong `app/src/components/settings_about_dev.rs`:
   - Xóa `div { class: "settings-group-title", "Developer options" }`.
   - Xóa icon `span { class: "about-dev-icon", BoltIcon { size: 18 } }`.
   - Xóa dòng mô tả `span { class: "about-dev-hint", ... }`.
   - Xóa toàn bộ khối `div { class: "about-dev-footer", ... }` gồm thanh kẻ ngang phân cách xám (`border-top`), huy hiệu `"Enabled"` / `"Đang bật"` và nút `"Open Developer"` / `"Mở Nhà phát triển"`.
   - Xóa `BoltIcon` khỏi import.
   - Giữ lại duy nhất hàng cài đặt tối giản phẳng: `"Chế độ nhà phát triển"` (hoặc `"Developer mode"`) kèm nút gạt chuyển đổi `switch` (checkbox).
2. Trong `app/assets/main.css`:
   - Dọn dẹp/tối ưu các class CSS liên quan để giao diện phẳng, tối giản, không còn viền xám thừa.
3. Kiểm thử tự động bằng Playwright:
   - Truy cập Cài đặt -> Giới thiệu.
   - Bật Developer Mode.
   - Kiểm tra xác nhận không còn:
     - Văn bản `"Tùy chọn nhà phát triển"`.
     - Icon `.about-dev-icon`.
     - Đoạn văn `.about-dev-hint`.
     - Chân thẻ `.about-dev-footer` (thanh xám, badge `"Đang bật"`, nút `"Mở Nhà phát triển"`).
     - Hàng chuyển đổi `"Chế độ nhà phát triển"` và công tắc switch hoạt động mượt mà.
4. Tự động commit và push lên `origin main`.
