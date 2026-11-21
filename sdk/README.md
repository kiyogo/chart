# ChatW Plugin & Mod SDK

Nền tảng phát triển Plugin & Mod chính thức cho ChatW (Web & Desktop).
Tác giả Plugin có thể tự lập trình hoàn toàn giao diện HTML/CSS/JS/TS hoặc biên dịch Rust sang WebAssembly (`.wasm`) mà **không cần chỉnh sửa mã nguồn cốt lõi của ChatW**.

## 1. Cấu trúc SDK

- `sdk/ts/` — `@chatw/plugin-sdk`: TypeScript definitions (`index.d.ts`) và helper (`index.js`) cho toàn bộ namespace `chatw.*` (`app`, `account`, `users`, `messages`, `conversations`, `groups`, `servers`, `channels`, `members`, `voice`, `video`, `events`, `ui`, `modals`, `menus`, `sidebar`, `toolbar`, `commands`, `notifications`, `storage`, `network`, `webhooks`, `bots`, `settings`, `permissions`, `theme`, `lifecycle`).
- `sdk/rust/` — `chatw-plugin-sdk`: Crate Rust biên dịch tới `wasm32-unknown-unknown` xuất ABI chuẩn `chatw_alloc` / `chatw_invoke` và gọi hàm host `chatw_host_log` / `chatw_host_emit`.
- `sdk/cli/chatw-plugin.mjs` — CLI phát triển gói plugin:
  - `node sdk/cli/chatw-plugin.mjs init <dir> <publisher.slug>`
  - `node sdk/cli/chatw-plugin.mjs validate <dir>`
  - `node sdk/cli/chatw-plugin.mjs hash <dir>`
  - `node sdk/cli/chatw-plugin.mjs pack <dir> [output.chatw-plugin.json]`
- `sdk/examples/task-tracker-js/` — Plugin thực tế viết bằng JS + HTML + CSS tùy biến (bảng Kanban, cài đặt, lưu trữ đồng bộ, lệnh `/track` & `/tasks`, nút thao tác `"Tạo thẻ"` trên tin nhắn, Event Hook `onMessageReceived`, kiểm thử cách ly sandbox và kiểm thử từ chối quyền E2EE).
- `sdk/examples/calc-wasm-rust/` — Plugin thực tế viết bằng Rust biên dịch ra nhị phân `plugin.wasm` (`wasm32-unknown-unknown`), cung cấp lệnh `/wcalc <biểu_thức>` và giao diện máy tính tùy biến.

## 2. Bảo mật, Chữ ký số & Quyền (Zero-Trust Opaque-Origin Sandbox)

- **Cô lập Origin Tuyệt đối (`origin = "null"`)**: Mọi logic JS/TS, WebAssembly (`.wasm`) và giao diện HTML/CSS tùy biến đều chạy bên trong `<iframe sandbox="allow-scripts">` (không bật `allow-same-origin`, CSP `connect-src 'none'`). Worker nền được khởi tạo bên trong chính iframe opaque-origin này nên không thể truy cập cookie, `localStorage`, `IndexedDB` của ứng dụng hay gọi trực tiếp `/api/*`.
- **Kiểm soát Tài nguyên & Vòng đời**: Giới hạn bộ nhớ WASM (`max_memory_mb`), giới hạn hàng đợi RPC đồng thời (`64`), tự động ngắt `worker.terminate()` khi quá thời gian khởi động (`5000ms`) hoặc quá thời gian thực thi (`max_exec_ms`).
- **Xác minh Chữ ký Số & Toàn vẹn Gói**: Hỗ trợ đăng ký khóa công khai nhà phát triển (`public_key`, P-256 ECDSA) và xác minh chữ ký số trên chuỗi chuẩn `chatw-plugin-sig-v1:<plugin_key>:<version>:<sha256>` cùng mã băm `SHA-256` của manifest và tạo tác.
- **Bảo vệ E2EE Đa lớp**: Khóa riêng tư E2EE không bao giờ rời khỏi lõi ứng dụng ChatW. Các quyền nhạy cảm và quyền E2EE (`e2ee.read_decrypted`, `e2ee.process_local`, `e2ee.egress_external`, `e2ee.store_external`) yêu cầu nhà phát triển đã xác minh, phê duyệt hai lớp và sự đồng ý tường minh của chính người dùng theo từng cuộc trò chuyện; máy chủ tự động chặn kết nối mạng ra ngoài nếu phiên Plugin đọc dữ liệu E2EE mà không có quyền `e2ee.egress_external`.
- **Webhook Chuẩn Discord & Chống SSRF**: Hỗ trợ Incoming Webhook tương thích Embed kiểu Discord (`POST /api/webhooks/{id}/{token}` + `Idempotency-Key`) và Outgoing Webhook ký `HMAC-SHA256` với bộ lọc chống SSRF, ghim IP sau phân giải DNS và chặn redirect.
