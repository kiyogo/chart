# Chatw Constitution

## Core Principles

### I. Mandatory Auto-Push Rule
Mỗi lần code xong hoặc hoàn thành xong yêu cầu của 1 prompt từ người dùng, AI Agent BẮT BUỘC PHẢI TỰ ĐỘNG COMMIT VÀ ĐẨY (GIT PUSH) TOÀN BỘ THAY ĐỔI LÊN GITHUB NGAY LẬP TỨC (`git push origin main`) mà không đợi người dùng phải nhắc lại.

### II. Spec-Driven Development (Spec Kit)
Mọi tính năng mới, sửa lỗi hoặc tái cấu trúc BẮT BUỘC phải đi qua chuỗi quy trình SDD: Spec (`spec.md`) -> Plan (`plan.md`) -> Tasks (`tasks.md`) -> Implement & Converge. Tuyệt đối không vibe-coding.

### III. Research Open Source Before Coding (Never Build From Scratch)
Đối với những chức năng, công nghệ hoặc thư viện chưa quen thuộc, AI Agent TUYỆT ĐỐI KHÔNG TỰ VIẾT TỪ CON SỐ 0. Bắt buộc phải chủ động tìm kiếm các dự án mã nguồn mở (Open Source) trên GitHub để so sánh, đối chiếu ý tưởng và lựa chọn kiến trúc tối ưu nhất trước khi bắt tay vào làm.

### IV. End-to-End Encryption (E2EE) Blind Relay
Backend Rust (Axum) chỉ đóng vai trò Blind Relay. Khóa riêng tư không bao giờ được lưu tại server. Client mã hóa và giải mã bằng Web Crypto API.

### V. Framework & Tech Stack Inviolability
- Backend: Rust 1.80+, Axum 0.8+, SQLx (PostgreSQL 16), Tokio.
- Frontend: SolidJS thuần (TypeScript, fine-grained reactivity: `createSignal`, `createEffect`, `For`, `Show`). Không dùng React, Vue, Next.js.
- UI: Pure CSS, Flat Minimalist Design, không viền dày, không đổ bóng đậm. Logo vector SVG. Avatar AVIF.

## Quality & Maintenance Priority
Thứ tự ưu tiên tối thượng:
1. Bảo mật thực chất -> 2. Hoạt động đúng -> 3. Độ ổn định -> 4. Hiệu năng -> 5. Đơn giản -> 6. Giao diện đẹp.

**Version**: 1.0.0 | **Ratified**: 2026-11-05 | **Last Amended**: 2026-11-05
