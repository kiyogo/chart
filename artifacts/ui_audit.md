# UI Audit — Spec 004 (UI foundation rebuild)

Ngày: 2026-11-18. Phương pháp: chạy bản WASM thật trên backend thật (`127.0.0.1:8085`, PostgreSQL), tài khoản
thật tạo qua dev-login (chỉ bật với PUBLIC_URL loopback), tin nhắn E2EE gửi qua UI. Đo bằng Playwright
(`app/e2e/ui-audit.mjs`): bounding box, overflow ngang, phần tử bị che (`elementFromPoint`), độ rộng bubble, cỡ chữ.

## Nguyên nhân gốc (trước khi sửa)
Commit `b897523` thay toàn bộ `main.css` bằng class của shell 4 cột (`workspace-shell`, `nav-rail`, `nav-sidebar`,
`main-stage`) và tạo `NavigationRail`, `NavigationSidebar`, `SettingsView`, modal cộng đồng — nhưng `pages/chat.rs`
**chỉ import, không render** chúng. Màn hình thực tế dùng class cũ (`sidebar`, `chat-view`, `sidebar-header`,
`server-item`, `channel-item`, `voice-btn`, `group-tab-container`…) **không còn CSS** → các lỗi trong ảnh người dùng gửi.

## Bảng lỗi
| # | Lỗi (ảnh người dùng / đo trước sửa) | Bằng chứng trước | Sửa | Sau |
|---|---|---|---|---|
| 1 | Giao diện quá nhỏ ở 1920/2560 | font 14px cố định, sidebar co theo nội dung | Token rem + cỡ chữ gốc tự động 15→16 (≥1680)→17 (≥2400)→18px (≥3400); sidebar `clamp(16.5rem,20vw,21rem)`; không dùng zoom | PASS 14 viewport, font tin nhắn ≥ 14px |
| 2 | Bubble kéo dài gần hết màn hình | 1540px ở 2560 | `--bubble-max: min(40rem,72%)`, luồng tin tối đa 68rem căn giữa, `overflow-wrap:anywhere` | ≤ 720px mọi viewport, 0 bubble tràn |
| 3 | Panel thông tin che chat ở 900/768 | `details.x=600 < msgs.r=900` | Inline ≥1280px; <1280px là drawer có backdrop + nút đóng; tự đóng khi thu nhỏ cửa sổ | PASS (overlay có chủ đích được nhận diện qua backdrop) |
| 4 | Mobile bị đẩy lệch, header/composer ra ngoài viewport | header.x=191, r=547 ở 375px | Shell mới: danh sách (rail + sidebar) ↔ màn hình chat, nút quay lại | PASS 320/360/375/390/430 |
| 5 | Settings mobile cắt chữ | 12 hàng vượt mép phải | Settings dạng danh sách → trang con, hàng tự xuống dòng, vân tay chia khối 4 ký tự | 0 hàng bị cắt |
| 6 | Trang đăng nhập sơ sài | form nhỏ giữa nền trống | Bố cục 2 cột (giới thiệu + thẻ đăng nhập), mobile 1 cột; dev-login tách riêng, ghi rõ "chỉ máy chủ cục bộ" | Ảnh `login_*` |
| 7 | Server/kênh không chuyên nghiệp | form "Tạo / Vào" trần, kênh ngang | Rail logo cộng đồng + sidebar danh mục dọc thu gọn được, kênh text/voice/announcement, modal tạo cộng đồng/danh mục/kênh/mời | Ảnh `community_*` |
| 8 | Panel người dùng đơn điệu, toàn khóa | 8 hàng thông số mật mã | Banner, avatar, tên hiển thị, bio, trạng thái tự đặt, 4 hành động, tìm trong hội thoại; khóa nằm trong mục "Bảo mật và mã hóa" thu gọn | Ảnh `dm_details_*` |
| 9 | Emoji picker 24 ký tự | 24 emoji cố định | 1.154 emoji, 8 danh mục, tìm kiếm EN/VI không dấu, gần đây, màu da, điều hướng phím mũi tên | `emoji search returns results` PASS |
| 10 | Không đồng nhất | padding/radius/inline style lẫn lộn | Design tokens (màu semantic, spacing 4→48, radius, motion, z-index), viết lại `main.css` | — |
| 11 | Tin nhắn tự gửi hiện 2 lần | ảnh `before/dm_details_light_1366x768.png` | Kiểm tra trùng id ở cả nhánh WebSocket và nhánh phản hồi API | 1 lần |
| 12 | Shift+Enter gửi tin thay vì xuống dòng | input nằm trong `<form>` | `Composer` dùng `textarea`, Enter gửi, Shift+Enter xuống dòng, bỏ qua Enter khi đang gõ IME | Ảnh: tin 3 dòng hiển thị đúng |
| 13 | Sticker nhóm hiển thị như URL | text bubble | Nội dung chỉ là đường dẫn ảnh `/uploads/…` hiển thị dạng media | — |
| 14 | Dark mode tương phản thấp | mục chọn chữ xanh trên nền xanh | Bảng màu dark riêng, `--accent-text` tính bằng `color-mix`, surface phân tầng | Ảnh `*_dark_*` |

## Kết quả đo cuối
Xem `test_results.md` — script ghi toàn bộ kiểm tra vào `screenshots/after/results.json`.

## Đánh giá chất lượng hình ảnh (tự đánh giá, có thể sai)
| Tiêu chí | Đánh giá | Ghi chú / còn yếu |
|---|---|---|
| Phân cấp thị giác | Khá | Tiêu đề, tên, metadata tách cấp rõ. Màn hình trống ở 2K/4K vẫn nhiều khoảng trắng phía trên tin nhắn khi hội thoại ngắn. |
| Typography | Khá | Dùng font hệ thống (Noto Sans trên Linux). Chưa đóng gói Inter/Be Vietnam Pro (cần tải font — chưa được phép tải tệp). |
| Màu & tương phản | Khá | Chưa đo WCAG tự động; đánh giá bằng mắt. |
| Căn chỉnh & spacing | Khá | Các component cũ (groups/*, calls/*) vẫn còn inline style chưa chuẩn hóa. |
| Responsive | Tốt | 0 lỗi trên 14 viewport × 2 theme. |
| Icon | Trung bình | Bộ SVG nội bộ kiểu Lucide; vài icon cũ không nhận `size`. |
| Cuộc gọi | Chưa xét lại | `ActiveCallOverlay` giữ nguyên giao diện cũ (nền tối cố định). |
