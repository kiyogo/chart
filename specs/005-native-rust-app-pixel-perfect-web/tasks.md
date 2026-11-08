# Danh sách Tác vụ Kỹ thuật: Viết lại Giao diện Native Rust (.rs) Khớp 100% Web

## Nhóm 1: Chuẩn hóa Titlebar & Icon Vector SVG
- [x] Task 1.1: Bổ sung và tinh chỉnh toàn bộ các hàm vẽ icon Vector SVG trong `crates/chatw-client/src/titlebar.rs` (`draw_svg_back_icon`, `draw_svg_send_icon`, `draw_svg_menu_icon`, `draw_svg_close_icon`).
- [x] Task 1.2: Chuẩn hóa `render_titlebar` trong `titlebar.rs` không lệch toạ độ, xử lý kéo thả cửa sổ và 3 nút cửa sổ.

## Nhóm 2: Tái thiết kế Giao diện Đăng nhập (LoginView)
- [x] Task 2.1: Viết lại `crates/chatw-client/src/views/login.rs` khớp 100% web: Card 320px giữa màn hình, nút Google xanh, divider mảnh, input tài khoản và nút Đăng nhập nhanh.
- [x] Task 2.2: Rà soát và loại bỏ toàn bộ dấu ba chấm `...` trong `login.rs` theo Rule 6 trong `AGENTS.md`.

## Nhóm 3: Tái thiết kế Giao diện Chính (ChatView & SettingsView)
- [x] Task 3.1: Viết lại Sidebar Header trong `crates/chatw-client/src/views/chat.rs`: Bố trí Avatar 28px + Tên bên trái và 4 nút icon kích thước 34x34 cách nhau 4px bên phải, đảm bảo không bao giờ bị cắt xén icon đăng xuất.
- [x] Task 3.2: Viết lại Tab Bạn bè và Tab Trò chuyện trong Sidebar khớp 100% web, cập nhật các chuỗi trạng thái rỗng và placeholder đúng chuẩn.
- [x] Task 3.3: Viết lại Khu vực Chính (Center View): Thêm màn hình trạng thái rỗng ("Chọn một cuộc trò chuyện để bắt đầu") khi chưa chọn bạn bè.
- [x] Task 3.4: Viết lại Màn hình Cài đặt tràn toàn bộ Khu vực Chính với nút Quay lại `←`, thẻ hồ sơ, các mục Tài khoản, Mã hóa đầu cuối, Máy chủ và Đăng xuất.
- [x] Task 3.5: Cố định vị trí Header (54px), Vùng tin nhắn và Thanh nhập tin nhắn (60px ghim đáy) trong cuộc trò chuyện, bổ sung avatar đối tác ở tin nhắn nhận.

## Nhóm 4: Cấu hình Cửa sổ & Kiểm thử Hệ thống
- [x] Task 4.1: Tinh chỉnh cấu hình cửa sổ trong `crates/chatw-client/src/main.rs` và `app.rs` loại bỏ hiện tượng viền đen xung quanh.
- [x] Task 4.2: Chạy kiểm thử tự động `cargo test -p chatw-client` và `cargo test -p chatw-core`.
- [x] Task 4.3: Biên dịch bản phát hành Native Rust `cargo build --release -p chatw-client` và copy vào `dist-desktop/chatw.exe`.
- [x] Task 4.4: Kiểm tra chạy thử thực tế và tự động commit, push lên nhánh chính GitHub theo Rule 1 trong `AGENTS.md`.
