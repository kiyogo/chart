# chatw

Ứng dụng nhắn tin cá nhân một-một tối giản, hiệu năng cao, mã hóa đầu cuối (E2EE) thời gian thực.

## 1. Công nghệ

- **Frontend**: SolidJS, TypeScript, Vite, CSS thuần, Web Crypto API (P-256 ECDH, P-256 ECDSA, HKDF-SHA-256, AES-256-GCM), IndexedDB, WebSocket API.
- **Backend**: Rust, Axum, Tokio, SQLx, Rustls, Ring crypto provider.
- **Database**: PostgreSQL 17.

## 2. Yêu cầu hệ thống

- Rust 1.80+ (`cargo`, `rustc`).
- Node.js 20+ và `npm`.
- Docker hoặc PostgreSQL 16+ đang hoạt động.

## 3. Cấu hình môi trường

Tạo file `.env` tại thư mục `backend/` (dựa trên mẫu `.env.example`):

```bash
# Database
DATABASE_URL=postgres://chatw:chatw_dev_password@127.0.0.1:55432/chatw
DB_MAX_CONNECTIONS=10

# Server
BIND_ADDR=127.0.0.1:8080
PUBLIC_URL=http://localhost:8080

# Google OAuth 2.0 / OpenID Connect
GOOGLE_CLIENT_ID=your-google-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-google-client-secret
# GOOGLE_REDIRECT_URL=http://localhost:8080/api/auth/google/callback

# Session
SESSION_SECRET=your-32-chars-long-random-high-entropy-secret
SESSION_TTL_HOURS=168

# Static frontend assets (cho production)
STATIC_DIR=../frontend/dist
MAX_WS_CONNECTIONS=5000
TRUST_PROXY=false
```

### Thiết lập Google OAuth credentials:
1. Truy cập [Google Cloud Console](https://console.cloud.google.com/) -> **APIs & Services** -> **Credentials**.
2. Tạo **OAuth 2.0 Client ID** (Web application).
3. Thêm Authorized redirect URI: `http://localhost:8080/api/auth/google/callback` (hoặc domain production của bạn).
4. Điền `Client ID` và `Client Secret` vào `.env`.

## 4. Khởi chạy môi trường phát triển

### Bước 1: Khởi động PostgreSQL
```bash
docker compose up -d
```

### Bước 2: Chạy Backend (Rust + Axum)
Migrations được áp dụng tự động khi backend khởi động:
```bash
cd backend
cargo run
```

### Bước 3: Chạy Frontend (SolidJS)
```bash
cd frontend
npm install
npm run dev
```
Truy cập `http://localhost:3000` (Vite dev proxy tới backend port 8080).

## 5. Build và Chạy Production

### Build Frontend
```bash
cd frontend
npm run build
```
File được xuất vào `frontend/dist/`.

### Build Backend Release
```bash
cd backend
cargo build --release
```

### Chạy Production
Chạy file binary `target/release/chatw`. Backend Axum tự động phục vụ static assets từ `frontend/dist` và xử lý toàn bộ API + WebSocket trên cùng một port:
```bash
./target/release/chatw
```
Truy cập `http://localhost:8080`.

## 6. Chạy Kiểm thử (Tests)

### Backend Unit & Integration Tests (bao gồm E2EE flow thực tế với PostgreSQL):
```bash
cd backend
cargo test
```

### Frontend TypeScript Check & Build Verification:
```bash
cd frontend
npm run build
```

### Rust Linter & Format:
```bash
cd backend
cargo clippy -- -D warnings
cargo fmt --check
```

## 7. Kiến trúc Bảo mật & Giới hạn E2EE

- **Giao thức E2EE**:
  - Thỏa thuận khóa: ECDH đường cong NIST P-256 (secp256r1).
  - Chữ ký xác thực: ECDSA P-256 với SHA-256 (định dạng chuẩn IEEE P1363).
  - Dẫn xuất khóa: HKDF-SHA-256 (salt ngẫu nhiên 16 bytes, info gắn `message_id`).
  - Mã hóa xác thực: AES-256-GCM (nonce 12 bytes CSPRNG mỗi tin, tag 16 bytes).
  - AAD (Additional Authenticated Data): Liên kết `version (1)`, `message_id (16)`, `sender_id (16)`, `recipient_id (16)`, `recipient_fingerprint (32)`.
- **Khóa thiết bị**: Private keys được sinh trực tiếp và lưu tại IndexedDB của trình duyệt (`non-extractable`), không bao giờ gửi qua mạng hay lưu trên server.
- **Server Zero-Knowledge**: Backend Rust và PostgreSQL chỉ lưu ciphertext envelopes đã ký. Server không sở hữu khóa giải mã, không lưu plaintext, không giải mã tin nhắn.
- **Phân quyền nghiêm ngặt**:
  - Không thể gửi tin nhắn cho người chưa kết bạn.
  - Người thứ ba không thể truy cập API lịch sử tin nhắn của người khác.
  - Tráo đổi hay chỉnh sửa bất kỳ byte nào của envelope đều bị phát hiện bởi chữ ký ECDSA và xác thực AAD của AES-GCM.
- **Giới hạn kỹ thuật**:
  - E2EE bảo vệ nội dung trước server và kẻ tấn công mạng. Trình duyệt của chính người nhận bắt buộc phải giải mã để hiển thị tin nhắn, do đó người dùng có thể xem plaintext trong bộ nhớ hoặc DevTools của chính họ.
  - Mỗi tài khoản hỗ trợ 1 thiết bị hoạt động; mất private key trên thiết bị đồng nghĩa không thể giải mã lịch sử cũ được mã hóa bằng khóa đó.
