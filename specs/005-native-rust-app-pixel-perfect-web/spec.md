# Đặc tả Kỹ thuật: Tái thiết kế Giao diện Desktop Native Rust (.rs) Khớp 100% Web

## 1. Mục tiêu & Bối cảnh
Người dùng yêu cầu dứt khoát:
> *"app pahri viết = rs nhé không phải là web ở trong app viết lại từ đầu giao diện giống 100% web cho tôi đi"*

- **Yêu cầu cốt lõi**:
  1. Ứng dụng Desktop chạy độc lập (`dist-desktop/chatw.exe`) **BẮT BUỘC** phải được lập trình bằng mã nguồn Rust thuần (`.rs`) qua crate `crates/chatw-client` (sử dụng `eframe` / `egui`), **TUYỆT ĐỐI KHÔNG** dùng webview nhúng hoặc trình duyệt đóng gói như Tauri / Electron.
  2. Viết lại toàn bộ giao diện từ đầu (from scratch) trong Rust sao cho hiển thị, tỷ lệ, bố cục, màu sắc, font chữ và trải nghiệm tương tác khớp **100% so với bản Web SolidJS**.

---

## 2. Tiêu chí Nghiệm thu (Acceptance Criteria)

### 2.1. Bản dựng & Ngôn ngữ
- [x] AC-1: Tệp thực thi `dist-desktop/chatw.exe` được biên dịch trực tiếp từ Rust (`cargo build --release -p chatw-client`).
- [x] AC-2: Không phụ thuộc WebView2 hay trình duyệt nhúng nào ở tầng hiển thị của Client.

### 2.2. Giao diện Đăng nhập (Login View)
- [x] AC-3: Căn giữa chính xác với chiều rộng `max-width: 320px` trên nền trắng tinh khiết `#FFFFFF`.
- [x] AC-4: Logo chatw dạng bong bóng chat SVG 44x44 màu xanh `#2563EB`, tiêu đề `chatw` (20px, font-weight 600) và phụ đề `Ứng dụng nhắn tin bảo mật E2EE` (13px, `#737373`).
- [x] AC-5: Nút "Tiếp tục với Google" chuẩn màu xanh `#2563EB`, icon Google đa sắc nét, bo góc 8px.
- [x] AC-6: Đường ngăn cách "chế độ phát triển cục bộ" mảnh 1px `#E5E7EB`.
- [x] AC-7: Input thử nghiệm và nút "Đăng nhập nhanh" nền xám `#F7F8FA`, bo góc 8px, hover `#ECEEF2`.

### 2.3. Bố cục Ứng dụng Chính (Chat & Settings Layout)
- [x] AC-8: **Custom Titlebar (32px)**: Nền tối `#18181B`, viền đáy `#27272A`. Icon SVG xanh `#3B82F6` + text `chatw – Secure Messaging`. 3 nút điều khiển Minimize, Maximize/Restore, Close (46px) hoạt động chuẩn xác, không bị viền đen bao quanh cửa sổ.
- [x] AC-9: **Sidebar (320px cố định)**:
  - Header (58px): Avatar tròn 28px + tên người dùng (16px semibold) bên trái. 4 nút icon bên phải (Logo, Friends, Settings, Logout) kích thước 34x34, khoảng cách 4px, không bao giờ bị tràn lề hay cắt ngang icon đăng xuất.
  - Tab Friends: Input "Tên tài khoản" + Nút "Kết bạn" (xanh `#2563EB`), danh sách lời mời kết bạn có nút "Chấp nhận", trạng thái rỗng "Không có lời mời nào".
  - Tab Chats: Danh sách cuộc trò chuyện với Avatar 38px, tên bạn bè, preview tin nhắn cuối cùng, trạng thái rỗng "Chưa có bạn bè nào".
- [x] AC-10: **Khu vực Trò chuyện Chính (Center View)**:
  - Khi chưa chọn cuộc trò chuyện & chưa mở cài đặt: Hiển thị thông báo căn giữa "Chọn một cuộc trò chuyện để bắt đầu".
  - Khi mở Cài đặt (`isSettingsOpen = true`): Chiếm toàn bộ khu vực chính (thay vì cột hẹp bên phải), có Header với nút quay lại `←` và thẻ hồ sơ cá nhân, nhóm thông tin E2EE, cấu hình máy chủ và nút Đăng xuất đỏ.
  - Khi đang trò chuyện:
    - Header (54px): Avatar bạn bè 34px + tên + nút mở thông tin đối tác.
    - Vùng tin nhắn: Tin gửi căn phải màu xanh `#2563EB` chữ trắng; Tin nhận căn trái có Avatar tròn 28px và bong bóng xám `#F1F3F5` chữ đen `#171717`.
    - Thanh nhập tin nhắn (60px): Ghim chặt dưới đáy 100%, nút Emoji + Input xám `#F7F8FA` + Nút Gửi máy bay giấy xanh `#2563EB`.

### 2.4. Quy chuẩn Văn bản & Tuân thủ AGENTS.md
- [x] AC-11: Tuyệt đối không chứa ký tự ba chấm `...` tạm bợ ở bất kỳ chuỗi giao diện nào (Rule 6).
- [x] AC-12: Tự động commit và push toàn bộ lên GitHub `origin main` khi hoàn thành (Rule 1).
