# QUY TẮC BẮT BUỘC DÀNH CHO AI AGENT (AGENTS.MD)

Dự án này là ứng dụng nhắn tin cá nhân bảo mật cao, mã hóa đầu cuối (E2EE), hiệu năng cao được xây dựng bằng **Rust (Axum)** và **SolidJS (TypeScript)**.
Mọi AI Coding Assistant hoạt động trên kho lưu trữ này bắt buộc phải tuân thủ nghiêm ngặt 100% các nguyên tắc dưới đây.

---

## 1. QUY TẮC ĐẨY CODE LÊN GITHUB (MANDATORY AUTO-PUSH RULE)

> [!IMPORTANT]
> **Mỗi lần code xong hoặc hoàn thành xong yêu cầu của 1 prompt từ người dùng, AI Agent BẮT BUỘC PHẢI TỰ ĐỘNG COMMIT VÀ ĐẨY (GIT PUSH) TOÀN BỘ THAY ĐỔI LÊN GITHUB NGAY LẬP TỨC MÀ KHÔNG ĐỢI NGƯỜI DÙNG PHẢI NHẮC LẠI.**

### Quy trình đẩy code chuẩn:
1. Kiểm tra trạng thái: `git status`
2. Đưa tệp vào hàng đợi: `git add .` (đảm bảo `.gitignore` loại trừ `target/`, `node_modules/`, `dist/`, `uploads/`, `.env`)
3. Tạo commit với thông điệp rõ ràng theo chuẩn Conventional Commits (ví dụ: `feat: ...`, `fix: ...`, `style: ...`, `chore: ...`)
4. Đẩy thẳng lên nhánh chính: `git push origin main`
5. Báo cáo trạng thái commit và link GitHub cho người dùng trong phản hồi.

- **Kho lưu trữ chính**: `https://github.com/kiyogo/chart`
- **Nhánh chính**: `main`

---

## 2. KIẾN TRÚC VÀ CÔNG NGHỆ BẮT BUỘC

### 2.1. Backend (Rust)
- **Framework**: Axum 0.8+, Tokio runtime, Tower-HTTP.
- **Cơ sở dữ liệu**: PostgreSQL 16 qua SQLx (kèm migrations tự động).
- **Mã hóa E2EE**: Web Crypto API ở Client, Backend chỉ đóng vai trò Blind Relay (không lưu khóa riêng tư, không thể đọc nội dung tin nhắn).
- **Lưu trữ Avatar**: Tất cả avatar tải lên bắt buộc phải lưu dưới định dạng `.avif` trong thư mục `uploads/avatars/{user_id}.avif` và phục vụ qua tuyến `/uploads/avatars/`.

### 2.2. Frontend (SolidJS + TypeScript)
- **Framework**: SolidJS thuần (sử dụng fine-grained reactivity: `createSignal`, `createEffect`, `For`, `Show`). Tuyệt đối không dùng React, Vue, Next.js, Angular.
- **Phong cách giao diện**: Pure CSS thuần, Flat Minimalist Design, không viền dày, không đổ bóng đậm.
- **Logo & Đồ họa**: Logo chuẩn định dạng Vector `.svg` cho tất cả các vị trí (favicon, logo trang đăng nhập, logo ứng dụng).

---

## 3. TIÊU CHUẨN TỐI ƯU HÓA SEO & MARKETING (MARKETING SKILLS)
Tuân thủ toàn diện các quy chuẩn SEO & Marketing từ `marketingskills`:
- **Thẻ Meta chuẩn**: Tiêu đề trang, thẻ mô tả `meta description`, `keywords`, `canonical link`, `theme-color`.
- **Mạng xã hội (Social Share)**: Đầy đủ OpenGraph (`og:title`, `og:description`, `og:image`, `og:url`, `og:type`) và Twitter Card (`summary_large_image`).
- **Dữ liệu có cấu trúc (Schema.org)**: Nhúng JSON-LD chuẩn `WebApplication` / `SoftwareApplication` mô tả tính năng E2EE, nền tảng hoạt động và giấy phép.
- **Thu thập dữ liệu tìm kiếm**: Cung cấp đầy đủ `/robots.txt`, `/sitemap.xml` và `/llms.txt` (tối ưu hóa cho các AI Search Engine như ChatGPT, Perplexity, Claude, Gemini).

---

## 4. QUY TẮC BẢO TRÌ & PHÁT TRIỂN
- Không thêm tính năng AI thừa thãi hoặc chatbot vào trong ứng dụng.
- Không viết code giả (mock API) trong môi trường thực tế.
- Tất cả các chuỗi giao diện người dùng không được chứa ký tự ba chấm `...` mang tính tạm bợ.
- Giữ vững thứ tự ưu tiên: **1. Bảo mật thực chất -> 2. Hoạt động đúng -> 3. Độ ổn định -> 4. Hiệu năng -> 5. Đơn giản -> 6. Giao diện đẹp**.
