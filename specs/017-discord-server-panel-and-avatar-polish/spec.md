# Spec 017: Discord Server Member Sidebar & Header Search, Round Vico AI Avatar, Feed & Avatar Hover Polish

## 1. Mục tiêu
1. **Avatar tròn & đúng logo cho Vico AI (`vico_ai_view.rs`, `main.css`)**:
   - Avatar của Vico AI trong tiêu đề, màn hình chào/trống (sau khi xóa cuộc trò chuyện hoặc tạo mới) và bên cạnh bong bóng tin nhắn trợ lý sử dụng khung tròn (`border-radius: 50%; overflow: hidden`) chứa đầy đủ logo SVG `VicoAiIcon { full: true }` không viền vuông thừa.
   - Luôn cho phép xóa/làm mới cuộc trò chuyện trong khung chat Vico AI (kể cả khi chỉ có 1 cuộc trò chuyện, xóa sẽ tạo phiên mới sạch với màn hình logo tròn chuẩn).
2. **Làm sạch thanh lọc trên bảng tin Mới (`feed_view.rs`)**:
   - Xóa thanh `.feed-top-bar` (`Tất cả`, `Bạn bè`, `Của tôi` và nút làm mới) ở đầu bảng tin **Mới**, chỉ giữ dải Tin/Ghi chú, ô đăng bài và danh sách bài viết.
3. **Loại bỏ hoàn toàn hiệu ứng hover & viền focus xanh trên tất cả Avatar (`main.css`, `profile_card.rs`)**:
   - Khi di trỏ chuột vào bất kỳ avatar nào (dải `StoriesNotesBar`, bảng tin `Mới`, danh sách tin nhắn, khung tin nhắn, thanh rail, danh sách thành viên), avatar đứng yên hoàn toàn: không dịch chuyển `translateY`, không scale, không animation hover `.avm-t-hover:hover`, không đổ bóng/viền hover.
   - Bỏ viền focus xanh đậm (`outline: 2px solid var(--accent)`) khỏi các nút bấm icon/avatar (`.note-item`, `.feed-avatar-btn`, `.message-avatar-slot button`, `.rail-account`, `.icon-btn`, `.member-row`).
4. **Thanh tìm kiếm & Danh sách thành viên phong cách Discord cho Máy chủ & Doanh nghiệp**:
   - Trong header của **CẢ Máy chủ cộng đồng lẫn Không gian Doanh nghiệp** (cả các module Doanh nghiệp lẫn kênh văn bản của máy chủ Doanh nghiệp):
     - Giữ nút bật/tắt bảng thành viên bên phải (`PanelRightIcon`, không viền xanh).
     - Hiển thị ô tìm kiếm gọn kiểu Discord (`Tìm kiếm {tên máy chủ}` kèm `SearchIcon`) tìm đồng thời thành viên, tin nhắn kênh (`/api/servers/{id}/search`) và dữ liệu doanh nghiệp (khi ở chế độ Doanh nghiệp).
   - Bảng thành viên bên phải (`ServerMembersPanel`) kiểu Discord:
     - Phân nhóm thành viên theo Vai trò (`server_roles`, hoặc `Chủ sở hữu` / `Thành viên`) và nhóm `Ngoại tuyến`, hiển thị `{Tên nhóm} — {Số lượng}`.
     - Từng dòng thành viên có avatar tròn + chấm trạng thái (`online`/`idle`/`dnd`/`offline`), tên mang màu vai trò + nameplate (nếu có) và dòng trạng thái phụ (`status_emoji` + `status_text`) phía dưới.

5. **Làm gọn danh sách Bạn bè & Ảnh bìa bảng Thông tin DM**:
   - Trong thanh bên **Bạn bè** (`sidebar.rs`), xóa các dòng chữ `"Lời mời kết bạn"`, `"Không có lời mời nào"`, `"Tất cả bạn bè"` và cân đều khoảng cách giữa ô tìm kiếm/kết bạn, lời mời (nếu có) và danh sách bạn bè.
   - Trong bảng chi tiết cuộc trò chuyện DM (`details_panel.rs`), xóa thanh tiêu đề `"Thông tin"` ở trên cùng để phần ảnh bìa (`.details-banner`) hiển thị tràn lên sát đỉnh thẻ bên phải (full ảnh bìa).

## 2. Acceptance Criteria
- `AC1`: Trong `Chat với Vico AI`, mọi avatar của Vico AI (header, empty state sau xóa/tạo mới, message stream) đều tròn hoàn toàn (`50%`) và hiển thị full logo Vico AI không bị khung vuông.
- `AC2`: Trang `Mới` không còn dải chip `Tất cả / Bạn bè / Của tôi` và nút refresh ở trên đầu.
- `AC3`: Di chuột qua bất kỳ avatar nào hay click nút icon/avatar không xuất hiện hiệu ứng nhấp nhô/scale/animation hay khung viền xanh vuông nào.
- `AC4`: Mở máy chủ thường hoặc máy chủ Doanh nghiệp (kể cả khi mở các module `Tổng quan`, `Nhân sự`, `Công việc`, hoặc kênh `#general`) đều thấy nút `PanelRightIcon` + ô `Tìm kiếm {ServerName}` ở góc phải header và bảng thành viên bên phải phân nhóm theo vai trò giống Discord.
- `AC5`: Tab **Bạn bè** không còn chữ `"Lời mời kết bạn"`, `"Không có lời mời nào"`, `"Tất cả bạn bè"` và mọi hàng đều cách đều nhau; bảng chi tiết DM bỏ thanh `"Thông tin"` phía trên để hiện full ảnh bìa từ sát mép trên.
