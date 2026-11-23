# Spec 104: Terms of Service, Privacy Policy & Community Standards (Điều khoản dịch vụ, Chính sách bảo mật, Tiêu chuẩn cộng đồng)

## 1. Problem Statement
The user requested adding:
1. **Điều khoản dịch vụ (Terms of Service)**
2. **Chính sách bảo mật (Privacy Policy)**
3. **Tiêu chuẩn cộng đồng (Community Standards)**

Specifically, for the **Điều khoản dịch vụ**, the tone and style must be:
- "ngắn nhất có thể một là một hai là hai đúng" (as concise and definitive as possible, steel manifesto / crisp legal principles).
- Must include the user's specific numbered points:
  - **Thứ nhất**: Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!
  - **Thứ hai**: Từ chối cung cấp bất kì thông tin cho bên thứ ba nào.
  - **Thứ ba**: Ẩn danh tuyệt đối (không thu thập số điện thoại, danh tính thật hay dấu vết cá nhân).
  - **Thứ tư**: Mã hóa đầu cuối (E2EE) thực chất (chỉ người gửi và người nhận giải mã được; máy chủ hoàn toàn mù - Blind Relay).
  - **Thứ năm**: Toàn bộ dữ liệu máy chủ luôn được mã hóa, chỉ người có mật khẩu mới mở được, khi bị tấn công hệ thống sẽ tự động ngăn chặn và bảo vệ dữ liệu.
  - **Thứ sáu**: Zero-logs tuyệt đối (không lưu vết tin nhắn, cuộc gọi, hành vi; xóa là biến mất vĩnh viễn).
  - **Thứ bảy**: Quyền sở hữu tối thượng (khóa mật mã nằm trọn trên thiết bị của bạn).
  - **Thứ tám**: Mã nguồn mở minh bạch (không cửa sau, sẵn sàng cho kiểm toán độc lập).
  - **Thứ chín**: Tự do và phi thương mại hóa (không quảng cáo, không bán dữ liệu).
  - **Thứ mười**: Không khoan nhượng với phá hoại (mọi hành vi tấn công, phát tán mã độc bị loại bỏ vĩnh viễn).
- Accompanying **Chính sách bảo mật** and **Tiêu chuẩn cộng đồng** written with the same ultra-concise, definitive clarity.
- Seamlessly accessible in:
  1. **Settings -> About (Cài đặt -> Giới thiệu)**: Subtabs for Overview, Terms, Privacy Policy, Community Standards, and Guide.
  2. **Login screen (Trang đăng nhập)**: 1-click modal popups for Terms, Privacy, and Community Standards.
  3. **Settings -> Privacy & Security**: Direct access card to review Terms & Privacy Policy.

## 2. Acceptance Criteria
- **AC1**: Dedicated component `LegalPanel` providing the 3 documents in Vietnamese and localized cleanly.
- **AC2**: **Điều khoản dịch vụ** matches the exact 10 steel principles ("một là một hai là hai đúng").
- **AC3**: **Chính sách bảo mật** contains 5 crisp core commitments (Zero data collection, E2EE, Zero sharing, Permanent deletion, Active system self-defense).
- **AC4**: **Tiêu chuẩn cộng đồng** contains 5 definitive rules (Privacy respect, No malware/fraud, No spam/DDoS, Zero tolerance for child harm, Civilized conduct).
- **AC5**: Settings -> About integrates subtabs: `Tổng quan`, `Điều khoản`, `Chính sách bảo mật`, `Tiêu chuẩn cộng đồng`, `Hướng dẫn`.
- **AC6**: Login page (`login.rs`) has interactive links to open each document in a modal.
- **AC7**: Playwright E2E automated test verifies accessibility, display, and navigation to all three documents.
