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

## Spec 005
| Tính năng | Trạng thái | Bằng chứng |
|---|---|---|
| Cột trái chỉ biểu tượng, không server, tooltip | DONE | `rail is icon-only`, `rail shows navigation, no servers by default` |
| Ghim cộng đồng / nhóm vào cột trái (Cài đặt) | DONE | `pinned communities appear in rail`, `pinned groups appear in rail` |
| Bộ icon SVG riêng "ChatW Duo" (47 icon) + logo mới | DONE | `all rail icons use the ChatW Duo set` |
| Hồ sơ: màu chủ đề, khung avatar, hiệu ứng, ngày tham gia, lưu server | DONE | `profile customization saved on server` |
| Popover hồ sơ từ avatar, đóng bằng Esc | DONE | `profile popover opens from avatar`, `closes with Escape` |
| Bộ sticker gốc + tạo bộ + tải ảnh + xóa | DONE | `uploaded sticker appears in my pack` |
| Sticker trong DM (E2EE) / nhóm / kênh | DONE | `sticker decrypts and renders for receiver` |
| Reaction DM realtime | DONE | `DM reaction syncs to the other side` |
| Ctrl+K chuyển nhanh | DONE | `quick switcher opens the picked chat` |
| Hình nền chat, âm báo, bật/tắt hiệu ứng | DONE (client, lưu theo tài khoản) | ảnh `settings_customize_*` |
| Cắt ảnh avatar/banner thủ công (crop) | NOT DONE | hiện tự cắt giữa |
| Huy hiệu được cấp bởi hệ thống | NOT DONE | trường chỉ đọc, chưa có cơ chế cấp |

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

## Spec 007 (quyền riêng tư, nhóm, bot, Kinh doanh, tùy biến)
| Tính năng | Trạng thái | Bằng chứng |
|---|---|---|
| "Chính mình": luôn có trong Tin nhắn, mỗi tin ghi thiết bị gửi (Linux 1, Windows 2…), đổi tên thiết bị (bấm tên hoặc Cài đặt), xóa và tạo lại, chỉ một cái mỗi tài khoản | DONE | UI `self note is encrypted, sent and shown`; migration 0014 bỏ ràng buộc cũ chặn tin gửi cho chính mình |
| Ghim cuộc trò chuyện, ghim tin nhắn (thanh tin ghim) | DONE | `pinned conversation moves to the pinned section`, `pinned message bar appears` |
| Thẻ phân loại (tạo, đổi màu, gắn cho DM / nhóm / kênh / bot, lọc) | DONE | API smoke (nhãn người khác bị từ chối) |
| "Đã xem" và "đang soạn tin" (tắt là ẩn hai chiều) | DONE | `"Đã xem" shows after the friend reads` |
| Chặn / bỏ chặn, danh sách chặn, ai được kết bạn / gọi / thêm vào nhóm, cho phép tìm thấy | DONE | `blocking shows a banner and disables sending`; API smoke |
| Trả lời tin nhắn trong DM (E2EE), bấm đúp vùng cạnh tin để trả lời | DONE | |
| Gợi ý @ nhắc tên, emoji `:tên`, trả lời nhanh `/phím`, sticker theo nội dung | DONE | `@ mention suggestions appear`, `Tab inserts the mention`, `sticker suggestions match the message` |
| Dán ô Excel: Luôn hỏi / Dạng ảnh / Dạng bảng / Dạng chữ | DONE | `pasting spreadsheet cells asks image / table / text`, `table message renders for the receiver (E2EE)` |
| Bình chọn, ghi chú, nhắc hẹn (cả DM và nhóm), ảnh E2EE trong DM | DONE | `poll arrives for the friend`, `vote is counted on both sides` |
| Thông báo hệ thống + 4 âm báo, tắt thông báo từng cuộc trò chuyện | DONE (client) | quyền thông báo do trình duyệt quyết định |
| Mã PIN khóa ứng dụng (PBKDF2, khóa khi không hoạt động / khi mở, giới hạn số lần sai) | DONE | `lock screen appears`, `wrong PIN is rejected`, `correct PIN unlocks` |
| Trả lời tự động (vắng mặt / theo giờ / luôn luôn), lời chào Kinh doanh | DONE (khi app đang mở trên một thiết bị, vì E2EE) | |
| Tùy biến: màu nhấn tự chọn, màu bong bóng, hình dạng bong bóng / avatar, độ rộng, phông, giảm chuyển động, hình nền toàn app (ảnh / màu / mẫu, độ tối, làm mờ) | DONE | |
| Hình nền riêng từng đoạn chat, hai bên cùng thấy (realtime) | DONE | API smoke; giá trị được kiểm tra chặt ở cả máy chủ và client |
| 33 ngôn ngữ cho điều hướng, danh sách và ô soạn tin; RTL cho Ả Rập, Ba Tư, Do Thái, Urdu | PARTIAL | `language switch translates the rail`, `right-to-left languages flip the layout`; các trang khác vẫn tiếng Việt |
| Kinh doanh: trang công khai `/k/handle`, sản phẩm, trả lời nhanh, lời chào, gửi xác minh | DONE | API smoke (escape HTML, sitemap) |
| Nhóm đơn giản trong Tin nhắn, chuyển sang Cộng đồng; kênh tách ra Tin nhắn | DONE | |
| Quản lý nhóm kiểu Zalo: Trưởng / Phó nhóm, 7 quyền thành viên, duyệt thành viên, đánh dấu tin trưởng / phó, liên kết tham gia (sao chép / chia sẻ / đổi), chặn nhóm | DONE | `group management dialog opens`, `group invite is a link` |
| Lời mời là liên kết `/invite/g/…` và thẻ mời có ảnh, ảnh bìa, số thành viên | DONE | |
| Vai trò cộng đồng kiểu Discord: tạo / sửa / xóa, 10 quyền, gán / gỡ, mời ra, không cấp quyền mình không có | DONE | API smoke 2 (11 kiểm tra) |
| Bot như Telegram: tạo, token (hiện một lần, đổi), getMe / getUpdates / sendMessage, bot trong nhóm, quy tắc tên | DONE | API smoke |
| Tìm kiếm toàn trang: của bạn, mọi người, cộng đồng / nhóm / bot / Kinh doanh công khai đã xác minh | DONE | `full-page search finds friends` |
| Trang quản trị riêng `/admin` (xác minh, tài khoản, nhật ký) | DONE | API smoke |
| SEO: trang cộng đồng công khai liệt kê tin ghim trước, sitemap động | DONE | |
| Emoji: 1.906 emoji RGI (+ màu da), 259 cờ; 146 sticker (126 mới, 7 bộ); 22 ảnh động phản ứng | DONE | `tools/gen_stickers.py`, ảnh xem thử |
| Phòng họp kiểu Google Meet ở cột trái (khách chỉ nhập tên, phòng chờ duyệt) | NOT DONE | cần dịch vụ phòng + phiên khách |
| Phát trực tiếp (Go Live) trong kênh thoại | NOT DONE | |
| Bộ sticker / emoji tải từ Internet | NOT DONE | cần bạn đồng ý tải (tên tệp, nguồn, dung lượng) |
