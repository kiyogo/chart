# Danh sách Tác vụ Triển khai: Đồng bộ Giao diện 100% Khớp Web

## Nhóm 1: Hệ thống Bảng màu & Visual Styling (Theme)
- [x] Task 1.1: Cập nhật egui `Visuals` trong `crates/chatw-client/src/app.rs` sang Light Minimalist (`#FFFFFF`, `#F7F8FA`, `#171717`, `#737373`, `#2563EB`, `#E5E7EB`).
- [x] Task 1.2: Cập nhật component `titlebar.rs` đảm bảo giữ thanh tiêu đề Dark `#18181B` với các nút điều khiển thu nhỏ, phóng to, đóng chính xác.

## Nhóm 2: Màn hình Đăng nhập & Đặt tên
- [x] Task 2.1: Tái thiết kế `crates/chatw-client/src/views/login.rs` khớp 100% web: Container nền trắng, box 320px, logo xanh, nút Google xanh kèm icon G 4 màu, divider "chế độ phát triển cục bộ", form dev login và banner lỗi đỏ.
- [x] Task 2.2: Tái thiết kế `crates/chatw-client/src/views/set_name.rs` khớp 100% web: Box 320px, input bo góc viền mờ, nút "Tiếp tục" xanh `#2563EB`.

## Nhóm 3: Màn hình Chat & Thanh bên Sidebar
- [x] Task 3.1: Xây dựng cơ chế Tabs trong Sidebar của `ChatView`: Tab 1 "Đoạn chat" (`chats`) và Tab 2 "Bạn bè" (`friends`).
- [x] Task 3.2: Triển khai danh sách hội thoại trong Tab "Đoạn chat" hiển thị Avatar tròn, tên bạn bè, tin nhắn xem trước gần nhất, và trạng thái hội thoại đang chọn.
- [x] Task 3.3: Triển khai Tab "Bạn bè" với ô tìm kiếm kết bạn, danh sách lời mời kết bạn đang chờ (nút Chấp nhận ✓ và Từ chối ✕), cùng danh sách bạn bè trực tuyến.
- [x] Task 3.4: Bổ sung nút Settings và nút Đăng xuất trên thanh Profile Header của Sidebar.

## Nhóm 4: Khung Chat Chính, Tin nhắn & Bộ chọn Emoji
- [x] Task 4.1: Thiết kế Header cuộc trò chuyện với Avatar bạn bè, tên hiển thị, tag `🔒 Mã hóa đầu cuối` và nút Info toggle.
- [x] Task 4.2: Tinh chỉnh bong bóng tin nhắn: Tin gửi đi xanh `#2563EB` chữ trắng bo góc 14px (góc dưới phải 2px); Tin nhận xám `#F1F3F5` chữ đen bo góc 14px (góc dưới trái 2px), kèm thời gian bên dưới.
- [x] Task 4.3: Thiết kế thanh nhập liệu với input xám bo góc `#F7F8FA`, nút icon Emoji và nút Gửi xanh `#2563EB` icon `➤`.
- [x] Task 4.4: Xây dựng Popup Emoji Picker dạng lưới 6 cột hiển thị các biểu tượng cảm xúc phổ biến khi bấm nút Emoji.

## Nhóm 5: Bảng Thông tin Đối tác (Partner Info) & Bảng Cài đặt (Settings)
- [x] Task 5.1: Xây dựng Partner Info Sidebar (cột thứ 3 bên phải): Hiển thị Avatar lớn 64px, tên, mã định danh UUID, thông tin giao thức E2EE (ECDH P-256 / AES-256-GCM, ECDSA P-256 / SHA-256), trạng thái khóa xanh `Đã kích hoạt trên thiết bị`, và vân tay khóa (Fingerprint).
- [x] Task 5.2: Xây dựng màn hình Cài đặt (Settings View): Xem thông tin hồ sơ tài khoản, thông tin khóa bảo mật E2EE của bản thân, tùy chỉnh URL máy chủ và nút Đăng xuất an toàn.

## Nhóm 6: Kiểm thử, Biên dịch Release & Đẩy mã nguồn lên GitHub
- [x] Task 6.1: Chạy kiểm thử tự động toàn diện (`cargo test -p chatw-client`), đảm bảo tất cả các test case đều pass.
- [x] Task 6.2: Biên dịch bản Release `cargo build --release -p chatw-client` và cập nhật `dist-desktop/chatw.exe`.
- [x] Task 6.3: Tự động commit và đẩy toàn bộ thay đổi lên GitHub `origin main` theo quy định Rule 1 trong `AGENTS.md`.

## Nhóm 7: Sửa lỗi hình học Layout, Glyph font và Vector Icons (Pixel-Perfect Matching)
- [x] Task 7.1: Sửa triệt để lỗi ô vuông font `□` bằng cách ưu tiên Arial và vẽ icon vector SVG thuần (`Painter`) cho Send, Close, Menu, Back, Chats, Friends, Settings, Logout.
- [x] Task 7.2: Khắc phục lỗi thanh input chat bị trôi lơ lửng giữa màn hình bằng bố cục 3 vùng hình học cố định (`header_rect`, `msgs_rect`, `input_bar_rect`) và `auto_shrink([false, false])`.
- [x] Task 7.3: Loại bỏ thanh tab con thừa thãi trong sidebar, chuẩn hóa 4 icon điều hướng trên header sidebar giống 100% web.
- [x] Task 7.4: Hiển thị avatar đối tác 28px cạnh bong bóng tin nhắn nhận.

