# ChatW — Nền tảng nhắn tin bảo mật E2EE, Cộng đồng, Doanh nghiệp & Hệ sinh thái Plugin đa nền tảng

Repository chính thức: <https://github.com/kiyogo/chart>

ChatW là ứng dụng nhắn tin thời gian thực mã hóa đầu cuối (E2EE), hội nghị thoại/video WebRTC SFU, máy chủ cộng đồng, không gian doanh nghiệp, sao lưu đám mây mã hóa `.kdata` và hệ sinh thái Plugin/Mod chạy trong sandbox cô lập (`origin = "null"`), được xây dựng thống nhất bằng **Rust (Axum 0.8 + Dioxus 0.7 + SQLx 0.9 + PostgreSQL 17)**.

---

## 1. Kiến trúc tổng thể (Modular Monolith)

```text
chatw/
├── app/                          # Ứng dụng Client đa nền tảng (Rust + Dioxus 0.7: Web WASM & Desktop WebKitGTK/WebView2)
│   ├── assets/                   # Hệ thống giao diện CSS thuần (main.css, theme.css)
│   ├── e2e/                      # Bộ kiểm thử tự động Playwright Chromium E2E
│   ├── public/                   # Tài sản tĩnh, SEO (robots.txt, sitemap.xml, llms.txt, logo.svg, stickers)
│   ├── src/
│   │   ├── components/           # Các module giao diện theo miền (messages, navigation, calls, groups, enterprise,
│   │   │                         #   plugins_view, plugin_pages, plugin_hub, plugin_settings, backup_cloud_ui,
│   │   │                         #   storage_ui, brand_logos, accounts, settings_view, feed_view, meet_view)
│   │   ├── crypto/               # E2EE ECDH P-256 + ECDSA P-256 + HKDF-SHA256 + AES-256-GCM & Key Wrapping at rest
│   │   ├── media/                # WebRTC Audio/Video & Screen Share client
│   │   ├── pages/                # Các trang chính (login, setup_username, chat)
│   │   ├── platform/             # Lớp trừu tượng nền tảng Web (WASM) và Native Desktop (Encrypted Vault 0600)
│   │   ├── services/             # Các client giao tiếp API, WebSocket, Plugin Host Bridge, Storage Sync
│   │   └── storage/              # OPFS / IndexedDB / Native FS cache & đồng bộ đa thiết bị
│   └── web/                      # Engine JS bổ trợ (plugin_runtime.js, storage_engine.js, custom_pickers.js,
│                                 #   canvas_scrollbars.js, panel_resizer.js, living_motion.js, media_optimizer.js)
├── crates/
│   └── kiyo-kdata/               # Định dạng sao lưu mã hóa .kdata (Argon2id + XChaCha20-Poly1305 + HMAC-SHA256 + ZIP AES)
├── docs/
│   └── architecture/             # Tài liệu kiến trúc hệ thống, mô hình bảo mật E2EE & Plugin Sandbox
├── sdk/                          # Bộ công cụ phát triển Plugin chính thức
│   ├── cli/chatw-plugin.mjs      # Công cụ dòng lệnh khởi tạo, kiểm tra, băm SHA-256 và đóng gói Plugin
│   ├── rust/                     # Rust WebAssembly SDK (chatw-plugin-sdk)
│   ├── ts/                       # TypeScript / JavaScript SDK (@chatw/plugin-sdk)
│   └── examples/                 # Các Plugin mẫu thực tế (task-tracker-js, calc-wasm-rust)
├── web/
│   ├── backend/                  # Máy chủ Rust Axum 0.8 + Tokio + SQLx 0.9 + PostgreSQL
│   │   ├── migrations/           # 31 tệp SQL migrations tự động (0001..0031)
│   │   ├── src/                  # Các module miền Backend (auth, security, crypto, messages, groups, servers,
│   │   │                         #   calls/sfu, enterprise, plugins, storage, backup, cloud, feed, vico, websocket)
│   │   └── tests/                # Bộ kiểm thử tích hợp thực tế trên PostgreSQL (integration_test, plugin_platform,
│   │                             #   storage_sync_test, kdata_cloud_test)
│   └── frontend/                 # Trình chạy Playwright E2E & tài sản bổ trợ
└── .github/workflows/ci.yml      # Đường ống CI đa nền tảng (PostgreSQL Backend Tests, Web WASM, Linux, Windows, macOS)
```

---

## 2. Điểm nổi bật về Bảo mật & Kiến trúc

1. **Mã hóa đầu cuối (E2EE) & Lưu trữ khóa an toàn (At-Rest Encryption)**:
   - Tin nhắn cá nhân (DM) và tệp đính kèm E2EE được mã hóa trên thiết bị bằng **ECDH P-256 + HKDF-SHA256 + AES-256-GCM** và ký số **ECDSA P-256**. Máy chủ chỉ đóng vai trò Blind Relay.
   - Khóa riêng tư E2EE và token phiên được bọc mã hóa **AES-256-GCM** (`enc:v1:...` / `vault:v1:...` với khóa thiết bị lưu trong tệp `.vault.key` quyền `0600` trên Desktop), đồng thời **tự động nâng cấp không phá hủy dữ liệu (non-destructive migration)** đối với các khóa cũ để người dùng không bao giờ mất tin nhắn cũ.
2. **Hệ thống Plugin Sandbox Cô lập Hoàn toàn (`origin = "null"`)**:
   - Cả giao diện HTML/CSS/JS lẫn Background Worker/WebAssembly (`.wasm`) của Plugin đều được khởi chạy bên trong `<iframe sandbox="allow-scripts">` (không có `allow-same-origin`, CSP `connect-src 'none'`).
   - Plugin chạy dưới opaque origin `"null"`, không thể đọc cookie, `localStorage`, `IndexedDB`, DOM của ứng dụng hay gọi trực tiếp `/api/*`, mà bắt buộc đi qua **Host Permission Broker** qua `postMessage` có nonce xác thực.
3. **Phân tách Quyền 4 Lớp & Bảo vệ E2EE khỏi Rò rỉ Mạng (Server-Side E2EE Taint Guard)**:
   - Tách biệt rõ: **Phạm vi cài đặt (Installation Scope)**, **Chủ thể cấp quyền (Grant Principal `user_id`)**, **Phạm vi tài nguyên (Resource Scope)** và **Phê duyệt hai lớp (Verified Publisher + Admin Sensitive Permission Approval)**.
   - Quyền cá nhân (`profile.read`, `contacts.*`, `e2ee.*`) không bao giờ bị kế thừa chéo giữa các thành viên trong máy chủ hoặc nhóm.
   - Máy chủ tự động theo dõi trạng thái E2EE (`is_server_side_e2ee_tainted`) và chặn mọi kết nối mạng ra ngoài (`/net/fetch`) nếu Plugin chưa được cấp quyền `e2ee.egress_external`, kể cả khi client giả mạo cờ `e2ee_tainted: false`.
4. **Xác minh Chữ ký Số Gói Plugin & Chống SSRF Luồng Có Giới hạn (Bounded Streaming)**:
   - Hỗ trợ xác minh chữ ký số **ECDSA P-256** (`chatw-plugin-sig-v1:<plugin_key>:<version>:<sha256>`) cùng mã băm `SHA-256` trước khi phát hành và cài đặt.
   - Proxy mạng của Plugin và các kết nối Cloud (WebDAV / Nextcloud / NAS / S3) vô hiệu hóa hoàn toàn HTTP redirect (`redirect::Policy::none()`), ghim IP sau khi phân giải DNS chống DNS rebinding và đọc luồng phản hồi theo từng chunk có giới hạn dung lượng cứng (`256 KB`).

---

## 3. Khởi chạy & Phát triển

### 3.1. Yêu cầu hệ thống
- Rust stable (`1.85+`, edition `2024`) và target `wasm32-unknown-unknown` + `wasm-bindgen-cli`
- PostgreSQL `16` hoặc `17`
- Node.js `20+` (dùng cho Playwright E2E và CLI `sdk/cli/chatw-plugin.mjs`)

### 3.2. Khởi chạy Backend & Web Bundle
```bash
# 1. Khởi động PostgreSQL (qua Docker hoặc dịch vụ cục bộ)
docker compose up -d

# 2. Biên dịch gói giao diện Web WASM vào app/dist/
./app/build-web.sh release

# 3. Chạy máy chủ Backend (tự động chạy toàn bộ migrations 0001..0031 và phục vụ trên cổng 8085)
cargo run -p chatw
```

### 3.3. Biên dịch Ứng dụng Desktop (Linux / Windows / macOS)
```bash
# Biên dịch bản thực thi Desktop
cargo build -p chatw-app --features desktop --release

# Đóng gói gói cài đặt Debian/Ubuntu (.deb)
./app/package-deb.sh
```

---

## 4. Kiểm thử (Unit, Integration, Security Regression & E2E)

```bash
# Kiểm thử đơn vị toàn bộ workspace (shared crate, backend, client)
cargo test -p kiyo-kdata
cargo test -p chatw --lib
cargo test -p chatw-app --lib

# Kiểm thử tích hợp Backend & Bảo mật Plugin trên PostgreSQL thực
cargo test -p chatw --test plugin_platform
cargo test -p chatw --test integration_test
cargo test -p chatw --test storage_sync_test
cargo test -p chatw --test kdata_cloud_test

# Kiểm thử giao diện & luồng thực tế trên trình duyệt Chromium (Playwright)
node app/e2e/plugin-platform-e2e.mjs
node app/e2e/spec048-ux-refinements.mjs
node app/e2e/spec049-brand-logos-living-motion.mjs
node app/e2e/spec050-zero-morph-and-pure-white.mjs
```
