# chatw

Ứng dụng nhắn tin bảo mật cao, mã hóa đầu cuối (E2EE) thời gian thực trên nền Web (Rust Axum + SolidJS).

## Cấu trúc thư mục

```text
chatw/
├── backend/              # Rust (Axum 0.8 + Tokio + SQLx + PostgreSQL)
│   ├── migrations/       # SQL migrations tự động
│   ├── src/
│   │   ├── attachments/  # Tệp đính kèm mã hóa
│   │   ├── audit/        # Nhật ký bảo mật
│   │   ├── auth/         # Google OAuth 2.0 & session cookie
│   │   ├── crypto/       # Xác thực chữ ký & envelope E2EE
│   │   ├── db/           # PostgreSQL pool & migrations
│   │   ├── friends/      # Bạn bè & lời mời kết bạn
│   │   ├── messages/     # Relay tin nhắn E2EE 1-1
│   │   ├── servers/      # Máy chủ, kênh, phân quyền RBAC
│   │   ├── typing/       # Trạng thái đang nhập
│   │   ├── users/        # Hồ sơ, khóa công khai, avatar .avif
│   │   ├── webrtc/       # Tín hiệu thoại/video WebRTC
│   │   ├── websocket/    # Hub WebSocket thời gian thực
│   │   ├── config.rs     # Đọc cấu hình môi trường
│   │   ├── error.rs      # Xử lý lỗi HTTP chuẩn
│   │   ├── models.rs     # DTO & hằng số phân quyền
│   │   └── main.rs       # Khởi tạo server & static assets
│   └── tests/            # Integration tests
├── frontend/             # SolidJS + TypeScript + Pure CSS
│   ├── e2e/              # Playwright E2E tests
│   ├── public/           # logo.svg, og-image.svg, robots.txt, sitemap.xml, llms.txt
│   └── src/
│       ├── api/          # HTTP API client
│       ├── components/   # Avatar, EmojiPicker, Icons
│       ├── crypto/       # Web Crypto API (ECDH/ECDSA P-256, AES-256-GCM)
│       ├── pages/        # Login, SetupUsername, Chat
│       ├── styles/       # Pure CSS Flat Minimalist
│       ├── utils/        # Xử lý ảnh AVIF
│       └── websocket/    # WebSocket client tự động kết nối lại
├── docker-compose.yml    # PostgreSQL 17 cho môi trường dev
└── .env.example          # Mẫu biến môi trường
```

## Khởi chạy môi trường phát triển

1. Khởi động PostgreSQL:
```bash
docker compose up -d
```

2. Sao chép `.env.example` thành `.env` (và `backend/.env`), điền `DATABASE_URL`, `SESSION_SECRET`, thông tin Google OAuth.

3. Chạy Backend (port `8085`):
```bash
cd backend && cargo run
```

4. Chạy Frontend (port `5173`, proxy `/api` tới `8085`):
```bash
cd frontend && npm install && npm run dev
```

## Build Production

```bash
cd frontend && npm run build
cargo build --release
./target/release/chatw
```

## Kiểm thử

```bash
cargo test
npm --prefix frontend run check
npm --prefix frontend run build
```
