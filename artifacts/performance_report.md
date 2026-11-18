# Performance Report

Chỉ ghi số đã đo thật trong phiên này.

| Hạng mục | Giá trị | Ghi chú |
|---|---|---|
| WASM release (`chatw_app_bg.wasm`) | 4.09 MB (930 KB gzip) | chưa chạy `wasm-opt` |
| JS glue (`chatw_app.js`) | 82 KB | |
| CSS (`main.css` + `theme.css`) | 75.5 KB chưa nén | nhúng vào bundle qua `include_str!` |
| Thời gian build wasm release (tăng dần) | ~16 s | |

## Thay đổi ảnh hưởng hiệu năng
- Bỏ listener resize đẩy mọi thay đổi kích thước vào state; giờ chỉ gửi sự kiện khi vượt ngưỡng 1280px.
- Hiệu ứng chỉ dùng opacity/transform (không animate width/height của vùng chat); hỗ trợ `prefers-reduced-motion`.
- Ảnh sticker dùng `loading="lazy"`.

## Chưa làm / chưa đo
- Virtual scrolling cho danh sách tin và danh sách hội thoại (hiện render toàn bộ 50 tin tải về).
- Mỗi tin nhắn mới vẫn làm render lại toàn bộ `MessageList` của hội thoại đang mở (chuyển đổi sang view model mỗi lần render).
- Chưa đo startup time, bộ nhớ, CPU, FPS khi cuộn.
- Đề xuất: `wasm-opt -Oz`, tách CSS ra file tĩnh có cache, phân trang tin cũ khi cuộn lên.
