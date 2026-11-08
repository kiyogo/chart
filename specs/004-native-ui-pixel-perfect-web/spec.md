# Đặc tả kỹ thuật: Đồng bộ giao diện Native Rust Desktop Client giống 100% bản Web (Pixel-Perfect)

## 1. Bối cảnh & Vấn đề
- Bản web gốc (SolidJS + Pure CSS) được thiết kế theo phong cách Flat Minimalist:
  - Nền chính sạch sẽ (`#FFFFFF`), thanh bên và ô nhập liệu xám nhẹ (`#F7F8FA`, `#ECEEF2`).
  - Màu chữ sắc nét tương phản cao (`#171717`, `#737373`).
  - Thanh tiêu đề Windows đồng bộ tối (`#18181B`).
  - Bong bóng tin nhắn gửi đi xanh dương nổi bật (`#2563EB`) với chữ trắng; tin nhắn nhận màu xám nhạt (`#F1F3F5`) với chữ đậm `#171717`.
  - Có 2 tab Sidebar rõ ràng: **Đoạn chat** (`chats`) và **Bạn bè** (`friends`).
  - Có bảng thông tin chi tiết bạn bè bên phải (Partner Info Sidebar) hiển thị chi tiết E2EE: Giao thức, Chữ ký số, Trạng thái khóa, Vân tay khóa.
  - Có bảng Cài đặt (Settings View) xem hồ sơ, khóa cá nhân và cấu hình máy chủ.
  - Có popup chọn Emoji (Emoji Picker) chuẩn.
- Bản Desktop Native Rust (`crates/chatw-client`) hiện tại đang bị lệch giao diện nghiêm trọng:
  - Dùng tone nền đen Zinc (`#09090B`) sai lệch hoàn toàn so với bản web.
  - Thiếu tab phân chia "Đoạn chat" và "Bạn bè".
  - Thiếu bảng thông tin đối tác mã hóa (Partner Info Panel bên phải).
  - Thiếu bảng Cài đặt (Settings Panel).
  - Thiếu bộ chọn Emoji dạng popup lưới 6 cột.
  - Các nút icon, avatar và căn lề chưa chuẩn tỷ lệ kích thước so với web.

## 2. Mục tiêu
Tái cấu trúc và nâng cấp toàn bộ hệ thống giao diện trong `crates/chatw-client` để đạt độ chính xác 100% so với mã nguồn web gốc (`frontend/src/` từ commit `4d2bd39`):
1. **Bảng màu & Visual Styling (Theme 100% khớp web)**:
   - `--bg-primary: #FFFFFF`
   - `--bg-secondary: #F7F8FA`
   - `--bg-hover: #ECEEF2`
   - `--text-primary: #171717`
   - `--text-secondary: #737373`
   - `--msg-sent: #2563EB`, text `#FFFFFF`
   - `--msg-recv: #F1F3F5`, text `#171717`
   - `--border: #E5E7EB`
   - `--tag-active: #E6F4EA`, text `#137333`
   - Custom Titlebar: `#18181B`, viền `#27272A`, chữ `#A1A1AA`
2. **Trang Đăng nhập (`LoginView`)**:
   - Khớp 100% layout `.auth-container` & `.auth-box` (320px).
   - Logo chatw xanh `#2563EB`.
   - Nút Google chuẩn với icon Google đa sắc (Blue/Red/Yellow/Green).
   - Đường phân cách `chế độ phát triển cục bộ`.
   - Nút và input dev login tinh gọn.
3. **Trang Đặt tên (`SetNameView`)**:
   - Khớp 100% layout và thông điệp.
4. **Trang Chat (`ChatView`)**:
   - Sidebar 320px nền `#F7F8FA`: Header hồ sơ người dùng + 2 tab navigation (`Đoạn chat` & `Bạn bè`) có badge đếm.
   - Danh sách đoạn chat có tin nhắn xem trước và thời gian.
   - Danh sách bạn bè có thêm bạn và danh sách yêu cầu kết bạn đang chờ (Accept/Reject).
   - Thanh chat trên cùng với avatar, tên, trạng thái `🔒 Mã hóa đầu cuối` và nút mở bảng thông tin bạn bè.
   - Bong bóng tin nhắn chuẩn bo góc (`nw: 14, ne: 14, sw: 14, se: 2` cho tin gửi; `nw: 14, ne: 14, sw: 2, se: 14` cho tin nhận).
   - Thanh nhập tin nhắn với nút Emoji mở popup lưới emoji, input xám bo góc và nút gửi xanh `#2563EB`.
   - Bảng thông tin đối tác bên phải (Partner Info Sidebar) hiển thị đầy đủ thông tin bảo mật E2EE và vân tay khóa.
   - Hỗ trợ màn hình Cài đặt (Settings View) đồng bộ.

## 3. Tiêu chí nghiệm thu (Acceptance Criteria)
- [ ] Giao diện Native Desktop hiển thị đúng 100% màu sắc, khoảng cách, font chữ và phong cách tối giản của Web.
- [ ] Các màn hình Login, SetName, Chat, Settings, Partner Info hoạt động mượt mà, phản hồi ngay lập tức.
- [ ] Đầy đủ các tương tác: Chuyển tab Sidebar, xem thông tin bạn bè, xem cài đặt, gửi tin nhắn E2EE, nhận tin nhắn thời gian thực qua WebSocket.
- [ ] Biên dịch Release sạch sẽ không lỗi, kiểm thử unit test pass 100%.
- [ ] Tự động commit và đẩy mã nguồn lên GitHub theo quy định Rule 1 trong `AGENTS.md`.
