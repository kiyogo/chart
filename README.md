# chatw

Ứng dụng nhắn tin bảo mật cao, mã hóa đầu cuối (E2EE) thời gian thực (Rust Axum + SolidJS).

## Cấu trúc thư mục

```text
chatw/
├── app/                      # Ứng dụng client đa nền tảng (Rust + Dioxus 0.7)
├── web/
│   ├── backend/              # Rust (Axum 0.8 + Tokio + SQLx + PostgreSQL)
│   │   ├── migrations/       # SQL migrations tự động
│   │   ├── src/
│   │   └── tests/
│   └── frontend/             # SolidJS + TypeScript + Pure CSS
│       ├── e2e/              # Playwright E2E tests
│       ├── public/           # logo.svg, og-image.svg, robots.txt, sitemap.xml, llms.txt
│       └── src/
├── docker-compose.yml        # PostgreSQL 17 cho môi trường dev
└── .env.example              # Mẫu biến môi trường
```

## Khởi chạy môi trường phát triển

1. Khởi động PostgreSQL:
```bash
docker compose up -d
```

2. Sao chép `.env.example` thành `.env` (và `web/backend/.env`).

3. Chạy Backend (port `8085`):
```bash
cd web/backend && cargo run
```

4. Chạy Frontend (port `5173`, proxy `/api` tới `8085`):
```bash
cd web/frontend && npm install && npm run dev
```

## Build Production & Kiểm thử

```bash
npm --prefix web/frontend run build
cargo test
cargo build --release
```
