# Security Audit (phạm vi giai đoạn này)

Không tuyên bố hệ thống "an toàn tuyệt đối". Đây là danh sách những gì đã kiểm tra trong phiên, những rủi ro đã
thấy và chưa xử lý.

## Đã kiểm tra / giữ nguyên
| Mục | Kết quả |
|---|---|
| E2EE DM | Không thay đổi giao thức (ECDH P-256 + HKDF-SHA-256 + AES-256-GCM, ký ECDSA P-256). UI mới chỉ hiển thị bản rõ đã giải mã trên client. Test UI: Alice ↔ Bob gửi/nhận, 0 lỗi giải mã trong hội thoại mới. |
| Không fallback plaintext | Tin không giải mã được hiển thị trạng thái lỗi, không gửi lại dạng thường. |
| Dev-login | Backend chỉ cho phép `DEV_LOGIN=true` khi `PUBLIC_URL` là loopback (`config.rs`), UI chỉ hiện khi `/api/auth/providers` trả `dev=true`. |
| Cookie phiên | `HttpOnly; SameSite=Lax`, `Secure` theo cấu hình. |
| CSP | `script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval'` — không inline script. Script áp theme sớm được đặt trong `boot.js` (không nới CSP). |
| XSS trong tin nhắn | Dioxus escape text; link chỉ nhận `http://`/`https://`, mở với `rel="noopener noreferrer nofollow"`. Không dùng `dangerous_inner_html`. |
| Sticker | Chỉ hiển thị dạng ảnh khi nội dung là đường dẫn `/uploads/…` với đuôi ảnh, render bằng `<img>` (không thực thi). |
| Presence | Bỏ nhãn "Ngoại tuyến"/"Đang hoạt động" vì backend không theo dõi presence realtime — tránh hiển thị thông tin sai. |

## Rủi ro đã thấy, CHƯA xử lý (ưu tiên giai đoạn sau)
1. **Khóa riêng tư lưu plaintext trong `localStorage`** (`chatw_e2ee_keys_{uid}`): XSS bất kỳ có thể đọc khóa. Đề xuất: WebCrypto non-extractable `CryptoKey` trong IndexedDB, hoặc bọc khóa bằng passphrase (PBKDF2/Argon2 + AES-GCM).
2. **Bản sao lưu khóa xuất JSON không mã hóa** và được copy vào clipboard. Đã thêm cảnh báo trong UI; cần mã hóa bằng passphrase.
3. **Tin nhắn nhóm & kênh cộng đồng không E2EE** — máy chủ lưu bản rõ. UI hiện ghi rõ điều này ở đầu hội thoại. Đề xuất nghiên cứu MLS (RFC 9420, ví dụ OpenMLS) thay vì tự thiết kế.
4. **Không có forward secrecy / multi-device**: mỗi trình duyệt sinh khóa mới → tin cũ không giải mã được trên thiết bị mới ("Không thể giải mã tin nhắn" mà người dùng thấy là hệ quả thiết kế này, không phải hỏng dữ liệu). Đề xuất Double Ratchet / Sesame hoặc MLS.
5. **Xác minh danh tính**: đã hiển thị vân tay để so sánh thủ công; chưa có trạng thái "đã xác minh" lưu cục bộ và cảnh báo khi khóa đối phương đổi trên UI mới (logic `get_or_verify_peer_keys` có sẵn trong crypto, chưa kiểm thử lại).
6. **Rate limiting**: chỉ có giới hạn số kết nối WebSocket (`MAX_WS_CONNECTIONS`, trả `TooManyRequests`); không thấy rate limit theo người dùng/IP cho auth, friends, messages — cần bổ sung.
7. **`'unsafe-eval'` trong CSP** vẫn còn (Dioxus web dùng eval cho `document::eval`).

## Không kiểm tra trong phiên này
CSRF chi tiết, IDOR từng route, SQL injection (SQLx macro đã tham số hóa), upload MIME thực tế, phân quyền cộng đồng
end-to-end, WebSocket auth, WebRTC authorization.

## Cập nhật Spec 005 (đã sửa)
- `PUT/POST /api/users/me/profile` bỏ qua `badges` từ client (trước đây người dùng tự gắn được huy hiệu, ví dụ "staff").
- `custom_banner_url` chỉ chấp nhận `/uploads/banners/…`; màu hồ sơ kiểm tra `#RRGGBB`; khung/hiệu ứng theo danh sách trắng; có CHECK ở DB (migration 0009).
- `POST /api/stickers/upload`: ≤ 512 KB, xác định định dạng bằng magic bytes (PNG/GIF/WebP/AVIF), tên tệp UUID; SVG và HTML bị từ chối (test: payload `<svg onload>` trả 400).
- `add_sticker` chỉ nhận URL `/uploads/stickers/…`; xóa sticker/bộ chỉ cho tác giả; danh sách chỉ gồm bộ có sẵn + bộ của chính mình.
- Sticker trong DM đi bên trong phong bì E2EE như văn bản; máy chủ không biết sticker nào được gửi.
- Reaction DM là siêu dữ liệu máy chủ nhìn thấy (emoji + người thả) — không mã hóa đầu cuối, ghi chú để người dùng biết.
