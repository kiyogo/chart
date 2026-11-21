# ChatW Plugin & Mod SDK

Nền tảng phát triển Plugin & Mod chính thức cho ChatW (Web & Desktop).
Tác giả Plugin có thể tự lập trình hoàn toàn giao diện HTML/CSS/JS/TS hoặc biên dịch Rust sang WebAssembly (`.wasm`) mà **không cần chỉnh sửa mã nguồn cốt lõi của ChatW**.

## Cấu trúc SDK

- `sdk/ts/` — `@chatw/plugin-sdk`: TypeScript definitions (`index.d.ts`) và helper (`index.js`) cho toàn bộ namespace `chatw.*` (`app`, `account`, `users`, `messages`, `conversations`, `groups`, `servers`, `channels`, `members`, `voice`, `video`, `events`, `ui`, `modals`, `menus`, `sidebar`, `toolbar`, `commands`, `notifications`, `storage`, `network`, `webhooks`, `bots`, `settings`, `permissions`, `theme`, `lifecycle`).
- `sdk/rust/` — `chatw-plugin-sdk`: Crate Rust biên dịch tới `wasm32-unknown-unknown` xuất ABI chuẩn `chatw_alloc` / `chatw_invoke` và gọi hàm host `chatw_host_log` / `chatw_host_emit`.
- `sdk/cli/chatw-plugin.mjs` — CLI phát triển gói plugin:
  - `node sdk/cli/chatw-plugin.mjs init <dir> <publisher.slug>`
  - `node sdk/cli/chatw-plugin.mjs validate <dir>`
  - `node sdk/cli/chatw-plugin.mjs hash <dir>`
  - `node sdk/cli/chatw-plugin.mjs pack <dir> [output.chatw-plugin.json]`
- `sdk/examples/task-tracker-js/` — Plugin thực tế viết bằng JS + HTML + CSS tùy biến (bảng Kanban, cài đặt, lưu trữ đồng bộ, lệnh `/track` & `/tasks`, nút thao tác `"Tạo thẻ"` trên tin nhắn, Event Hook `onMessageReceived`, kiểm thử cách ly sandbox và kiểm thử từ chối quyền E2EE).
- `sdk/examples/calc-wasm-rust/` — Plugin thực tế viết bằng Rust biên dịch ra nhị phân `plugin.wasm` (`wasm32-unknown-unknown`), cung cấp lệnh `/wcalc <biểu_thức>` và giao diện máy tính tùy biến.

## Bảo mật & Quyền (Zero-Trust Sandbox)

- Mọi logic JS/TS và `.wasm` chạy trong **Blob Web Worker** cách ly, giới hạn bộ nhớ WASM và cơ chế ngắt `worker.terminate()` khi vượt quá thời gian thực thi.
- Mọi giao diện tùy biến chạy trong `<iframe sandbox="allow-scripts">` (opaque origin, **không** bật `allow-same-origin`, CSP nghiêm ngặt) và chỉ giao tiếp với ChatW thông qua kênh JSON-RPC `postMessage` có nonce xác thực.
- Khóa riêng tư E2EE **không bao giờ** rời khỏi lõi ứng dụng ChatW. Các quyền nhạy cảm và quyền E2EE (`e2ee.read_decrypted`, `e2ee.process_local`, `e2ee.egress_external`, `e2ee.store_external`) yêu cầu duyệt hai lớp và người dùng cấp tường minh theo từng cuộc trò chuyện; thu hồi trên Bảng quyền riêng tư có hiệu lực ngay lập tức.
- Webhook hỗ trợ đầy đủ Incoming Webhook tương thích Embed kiểu Discord (`POST /api/webhooks/{id}/{token}` + `Idempotency-Key`) và Outgoing Webhook ký `HMAC-SHA256` chống SSRF.
