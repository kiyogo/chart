# Spec: Professional Overhaul of ChatW

## 1. Mục tiêu
Nâng cấp giao diện ChatW từ phong cách "AI demo" sang "Modern Flat Desktop UI" chuyên nghiệp, đồng nhất, và bổ sung các tính năng cộng đồng (kênh, phân quyền, hồ sơ đầy đủ, sticker, poll...).

## 2. Các thay đổi chính (UX/UI)
- Loại bỏ hoàn toàn: `box-shadow` phát sáng, gradient không cần thiết, border blur, hiệu ứng AI-style.
- Áp dụng hệ thống token CSS semantic cho Light/Dark mode.
- Cấu trúc Nhóm -> Category -> Channel.
- Hệ thống hồ sơ cá nhân hoàn chỉnh.
- Giao diện cuộc gọi video/thoại tối giản, chuyên nghiệp.
- Hệ thống thông báo và cài đặt chi tiết.

## 3. Các yêu cầu kỹ thuật
- Framework: Rust + Dioxus 0.7.x (giữ nguyên).
- Backend: Rust + Axum + PostgreSQL (giữ nguyên).
- Hỗ trợ đa nền tảng: Web, Windows, Linux, macOS, Android, iOS.
- Không phá vỡ E2EE hiện có.
