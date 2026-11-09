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

## Nhóm 5: Tinh chỉnh Chi tiết Từng Pixel Khớp 1000% Web & Icon Vector
- [x] Task 5.1: Chuẩn hóa khoảng cách sidebar header buttons: `btn_size = vec2(36.0, 36.0)`, `gap = 6.0` khớp chính xác `.sidebar-actions { gap: 6px; }` và `.icon-btn { width: 36px; height: 36px; }` trong CSS.
- [x] Task 5.2: Bổ sung huy hiệu thông báo chưa đọc (`.tab-badge`) cho tab Trò chuyện (`LogoIcon`) và huy hiệu tin nhắn chưa đọc (`.chat-item-badge`) trong danh sách cuộc trò chuyện.
- [x] Task 5.3: Chuẩn hóa icon Google G đa sắc 18x18 vẽ trực tiếp trên nút xanh `#2563EB` (loại bỏ hoàn toàn khung tròn màu trắng).
- [x] Task 5.4: Chuẩn hóa icon Cài đặt (`SettingsIcon`) 8 răng chuẩn Feather/Lucide, bán kính trong 3.0px.
- [x] Task 5.5: Kiểm thử tự động 10/10 test cases, biên dịch bản release `dist-desktop/chatw.exe` và tự động push lên GitHub.

## Nhóm 6: Tối ưu hóa Bộ Phông chữ & Typography Chuẩn Web Egui (Font Rendering Overhaul)
- [x] Task 6.1: Nhúng trực tiếp tĩnh (compile-time `include_bytes!`) bộ font Inter chuẩn quốc tế (`Inter-Regular.ttf`, `Inter-SemiBold.ttf`, `Inter-Bold.ttf`) loại bỏ hoàn toàn phụ thuộc vào font hệ thống Windows và loại bỏ fallback Ubuntu-Light lỗi thời.
- [x] Task 6.2: Viết kiểm thử `test_inter_vietnamese_glyphs` và mở rộng `test_vietnamese_font_glyphs` đối soát 100% tất cả ký tự có dấu tiếng Việt (thường và hoa) cùng các họ font proportional, semibold và bold.
- [x] Task 6.3: Tinh chỉnh `Visuals` trong `ChatApp::new`: gỡ bỏ `override_text_color = Some(...)`, cấu hình `fg_stroke` và bảng `text_styles` (`Heading`, `Body`, `Button`, `Small`, `Monospace`) chuẩn phẳng sắc nét.
- [x] Task 6.4: Biên dịch lại bản phát hành `cargo build --release -p chatw-client`, cập nhật `dist-desktop/chatw.exe` và tự động commit, push toàn bộ lên GitHub `origin main`.

## Nhóm 7: Khắc phục Triệt để Hiện tượng In Đậm Quá Mức (Slim & Crisp Typography)
- [x] Task 7.1: Ưu tiên font gốc Segoe UI chuẩn mảnh của Windows cho họ `Proportional` và `semibold`, Inter-Regular làm fallback, loại bỏ việc cưỡng ép `Inter-SemiBold` (600) dày cộm trên toàn bộ nhãn văn bản.
- [x] Task 7.2: Chuyển tên tài khoản người dùng (`123`), danh sách bạn bè, tiêu đề chat, nút bấm và các dòng cài đặt từ `font_semibold` sang `font_regular(14.0/15.0)` thanh mảnh, tinh tế giống 100% web.
- [x] Task 7.3: Rà soát và chuẩn hóa văn bản rỗng ("Chưa có bạn bè nào", "Chọn một cuộc trò chuyện để bắt đầu") về kích cỡ 14px regular mảnh màu `#737373` chuẩn CSS `.empty-chat`.
- [x] Task 7.4: Biên dịch lại bản phát hành `dist-desktop/chatw.exe`, kiểm thử 8/8 unit tests thành công và tự động commit, push lên GitHub `origin main` theo Rule 1.

## Nhóm 8: Tích hợp Bộ Phông Be Vietnam Pro & Khử Răng Cưa/Làm Mờ Pixel (Be Vietnam Pro Typography & Pixel Snapping)
- [x] Task 8.1: Nhúng trực tiếp tĩnh (`include_bytes!`) bộ font Be Vietnam Pro (`BeVietnamPro-Regular.ttf`, `BeVietnamPro-Medium.ttf`, `BeVietnamPro-SemiBold.ttf`) - chuẩn typography thiết kế riêng cho màn hình kỹ thuật số tiếng Việt, nét thanh mảnh, dấu thanh tinh tế, không dày cộm.
- [x] Task 8.2: Viết kiểm thử `test_bevietnam_pro_vietnamese_glyphs` và `test_bevietnam_pro_weights_glyphs` kiểm chứng toàn bộ 134 ký tự tiếng Việt có dấu ở các độ đậm khác nhau.
- [x] Task 8.3: Bổ sung các hàm helper làm tròn toạ độ `snap_pos` và `snap_xy` trong `titlebar.rs`, áp dụng cho toàn bộ các điểm vẽ chữ và huy hiệu `painter().text` / `painter().galley` trong `chat.rs`, `login.rs`, `set_name.rs`, và `titlebar.rs` để loại bỏ hiện tượng nội suy làm dày/mờ chữ (bilinear filtering blur).
- [x] Task 8.4: Kiểm thử toàn bộ 9/9 unit tests `chatw-client` và 3/3 `chatw-core`, biên dịch bản phát hành `dist-desktop/chatw.exe` và tự động commit, push lên GitHub `origin main` theo Rule 1.
