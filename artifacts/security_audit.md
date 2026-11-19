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

## Spec 007 (đã kiểm tra)
- Chặn: máy chủ từ chối tin nhắn, lời mời kết bạn, cuộc gọi và thêm vào nhóm giữa hai người đã chặn nhau.
- "Chính mình" mã hóa bằng khóa của chính bạn; máy chủ chỉ lưu bản mã. Nhãn thiết bị nằm bên trong phong bì E2EE.
- Ảnh trong DM: AES-256-GCM với khóa ngẫu nhiên mỗi ảnh, chỉ bản mã được tải lên; khóa đi trong tin nhắn E2EE. Tải tệp trả `application/octet-stream` + `nosniff`.
- Mã PIN: chỉ lưu PBKDF2-SHA256 210.000 vòng có muối ngẫu nhiên trên thiết bị; so sánh thời gian hằng; sai 5 lần thì chờ tăng dần. Đây là khóa giao diện cục bộ, không thay mật khẩu.
- Giá trị từ người khác chèn vào CSS (hình nền chung, ảnh bìa trên thẻ mời, banner) chỉ nhận đường dẫn cùng nguồn gồm ký tự an toàn hoặc `#RRGGBB`; ảnh đại diện trên thẻ mời không tải URL bên ngoài (tránh lộ IP).
- Vai trò cộng đồng: mọi bit quyền mới (mời ra, @everyone, vào kênh thoại) được kiểm tra ở máy chủ; không thể cấp hoặc thu hồi quyền mình không có; không ai mời được chủ sở hữu ra; quản trị viên chỉ chủ sở hữu mời ra được.
- Bot: token chỉ lưu SHA-256, hiện một lần; bot riêng tư chỉ nói chuyện với chủ; tên bot không được trùng tên tài khoản người khác.
- Tin nhắn nhóm, kênh cộng đồng và bot không mã hóa đầu cuối (máy chủ đọc được) — ghi rõ trong giao diện.
- Trả lời tự động và lời chào Kinh doanh gửi từ thiết bị của bạn (mã hóa đầu cuối); có dấu `[Tự động]` để hai tài khoản tự động không trả lời nhau vô hạn.

## Spec 009
- Avatar motion is a declarative object (`deny_unknown_fields`, enum ids, bounded numbers, `#RRGGBB` only); the client
  re-validates before writing CSS variables, so a profile cannot inject CSS or run code.
- Profile details are filtered by the server per viewer (everyone / friends / nobody; blocked users get 404).
- Contact notes are private to their owner (never returned to the other person); they are stored on the server without
  end-to-end encryption and the UI says they are private, not encrypted.
- Settings sync never carries the PIN lock (stripped by client and server); writes are versioned (409 on stale base).
- Avatar uploads must be AVIF / WebP / PNG / JPEG by magic bytes; covers additionally accept GIF; anything else 400.
- Saving a message to "Chính mình" re-encrypts it to the user's own key; nothing is stored in plaintext.

## Spec 011
- Parked accounts' session tokens are only in an HttpOnly, SameSite=Lax cookie scoped to /api/auth; scripts can't read them.
- TOTP secrets encrypted at rest (AES-256-GCM, key = SHA-256(label ‖ server secret)); backup codes stored as SHA-256.
- TOTP: ±1 step, constant-time compare, last accepted step stored (no replay); 5 attempts per challenge, 5-minute
  challenges, 10 failures / 15 min → 429. Disable / regenerate / sign out other devices need a valid code.
- Security log keeps only the network part of the IP (a.b.x.x).
