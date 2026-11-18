# Feature Matrix

Trạng thái: **DONE** (có frontend + backend thật, kiểm chứng trong phiên này) · **EXISTING** (đã có từ trước, không
kiểm thử lại trong phiên này) · **PARTIAL** · **NOT DONE** · **BLOCKED**.

## Giai đoạn này (Spec 004)
| Tính năng | Trạng thái | Bằng chứng |
|---|---|---|
| Shell khối nổi (sidebar có bộ chuyển Tin nhắn · Bạn bè · Cộng đồng / main / details) | DONE | ui-audit 14 viewport |
| Design tokens light / dark / system, 5 màu nhấn, 3 mật độ, cỡ chữ | DONE | Settings › Giao diện, lưu localStorage, áp dụng trước khi WASM khởi động (`boot.js`) |
| Responsive desktop / tablet / mobile | DONE | 0 lỗi bounding box |
| Danh sách hội thoại: tìm kiếm + bộ lọc (Tất cả / Chưa đọc / Cá nhân / Nhóm) | DONE | lọc cục bộ |
| Bubble mới: gom nhóm 5 phút, phân cách ngày, giờ trong bubble, link an toàn (`rel=noopener noreferrer nofollow`) | DONE | |
| Composer textarea, Enter / Shift+Enter, IME-safe | DONE | tin 3 dòng |
| Emoji picker đầy đủ (tìm kiếm, danh mục, gần đây, màu da, phím) | DONE | |
| Sticker hiển thị dạng ảnh, picker trong composer nhóm | DONE (hiển thị) | gửi sticker phụ thuộc bộ sticker người dùng tự tạo |
| Reaction nhóm: pill có đếm + trạng thái của tôi, thả nhanh khi hover | DONE | dùng API có sẵn |
| Trả lời tin nhắn trong nhóm (reply_to_id) | DONE | API có sẵn |
| Details panel DM: banner, bio, hành động, tìm trong hội thoại, mục bảo mật thu gọn | DONE | |
| Cộng đồng: danh sách trong sidebar (không còn cột server bên trái), danh mục dọc, kênh text/voice, tạo cộng đồng/danh mục/kênh, mã mời | DONE (UI nối API có sẵn) | `community opens from sidebar list` PASS |
| Phòng thoại: tham gia, danh sách người, tắt mic, rời | PARTIAL | UI + API trạng thái; truyền âm thanh kênh thoại **chưa kiểm chứng** |
| Trang đăng nhập mới | DONE | |
| Settings: Tài khoản, Giao diện, Bảo mật & mã hóa, Phiên; mobile list→detail | DONE | |
| Sửa trùng tin nhắn tự gửi | DONE | |

## Có từ trước (không kiểm thử lại)
DM E2EE (ECDH P-256 / AES-256-GCM / ECDSA), lời mời kết bạn, nhóm + thành viên + vai trò nhóm, mời nhóm, hồ sơ +
banner + avatar AVIF, gọi 1-1 và nhóm WebRTC P2P, sao lưu/khôi phục khóa, audit log backend.

## Chưa làm trong giai đoạn này (NOT DONE)
- Tin nhắn: sửa/xóa/chuyển tiếp/ghim/lưu từ UI mới, hẹn giờ, tin tự hủy, poll, đa chọn, xem trước link, highlight code,
  spoiler, mention, read receipts, trạng thái đã nhận.
- Reply cho DM (wire format E2EE chưa có trường reply khi gửi), reaction cho DM và kênh trong UI mới.
- Thread, forum, announcement riêng, sự kiện, onboarding, discovery UI, moderation queue, audit log UI, slow mode,
  phân quyền kênh, kéo-thả sắp xếp, server settings đầy đủ, roles UI cho cộng đồng.
- Voice message UI mới, video message, upload nhiều tệp + tiến độ, media gallery, GIF.
- Multi-account, command palette, quick switcher, phím tắt toàn cục, thông báo native, tray.
- Privacy settings (ai được nhắn/gọi), block list UI, thiết bị/phiên.
- Virtual scrolling cho danh sách dài.
- E2EE cho nhóm và kênh cộng đồng (hiện lưu dạng thường trên máy chủ — đã ghi rõ trong UI).

## BLOCKED / NOT TESTED
- Truyền media WebRTC thật (âm thanh/video) — không có thiết bị micro/camera trong môi trường headless: **NOT TESTED**.
- Windows/macOS rendering — không có môi trường: **NOT TESTED**.
- Đăng nhập Google OAuth thật — cần tài khoản người dùng: **NOT TESTED**.
