# Spec 014: Đồng bộ Avatar, Khoảng cách Tin nhắn, Switch Gạt, Giao diện Theme Plugin & Cài/Gỡ (Tải/Gỡ)

## Problem Statement
Người dùng báo cáo và cung cấp 10 ảnh chụp màn hình chỉ ra 5 nhóm lỗi UI/UX nghiêm trọng:
1. **Avatar không đồng bộ & chạm viền** (Ảnh 1, 2, 3, 8): Avatar ở `.chat-header` (Bot/Plugin, Nhóm, DM) có kích thước lệch nhau, sát mép trên/cạnh không đều; logo SVG bo góc nằm trong khung tròn đôi khi không phủ đều.
2. **Bong bóng tin nhắn chạm sát avatar** (Ảnh 4, 8): Khoảng cách giữa avatar người gửi và bong bóng tin nhắn đến quá nhỏ (`0–2px`) do `.message-avatar-slot` (`2rem` = `30px`) nhỏ hơn `Avatar { size: 32 }` (`32px`) và `gap` chỉ `var(--space-2)`.
3. **Header trang Plugin thừa icon và mô tả** (Ảnh 5): Trang `Plugin` đang hiển thị vòng tròn icon `PluginIcon` và dòng mô tả `"Tùy chỉnh giao diện, ứng dụng bên trái và lệnh"`.
4. **Lỗi giao diện nặng khi bật Theme Plugin tối & HUD che nội dung** (Ảnh 6, 7, 8, 9):
   - Khi bật theme tối như Voxel / Cyber Neon, `sync_plugins_to_dom` chưa ghi đè `--bg-input`, `--bg-input-focus`, `--bg-active`, `--bg-primary`, `--bg-warning`, `--bg-error`, `--bg-success`, `--border`, `--on-accent`, `color-scheme` khiến các ô `<input>`, `<textarea>`, thanh tab `.segmented`, thanh soạn thảo `.composer-box`, ô tìm kiếm và hộp `.e2ee-notice` vẫn mang nền trắng tinh với chữ trắng không đọc được.
   - Floating HUD (`pos-top-right`) đè lên ảnh bìa và avatar trong panel `Thông tin` bên phải.
5. **Plugin Store chưa có luồng Tải / Gỡ & lỗi nút gạt `.switch`** (Ảnh 7, 10):
   - Các Plugin chưa có nút **`Tải`** (cài từ Kho Plugin) và **`Gỡ`** (gỡ cài đặt khỏi danh sách đang dùng).
   - Nút gạt `input[type="checkbox"].switch` bị dính `input:focus` (`--bg-input-focus` + `--focus-ring`) và `.switch:focus-visible` (`outline` + `outline-offset`), tạo vòng viền trắng/xanh kép méo mó khi bấm hoặc di chuột.

## Acceptance Criteria
- **AC1 (Avatar & Khoảng cách đồng bộ)**:
  - Tất cả avatar trong `.chat-header` (Tự nhắn, DM, Nhóm, Bot/Plugin, Trang Plugin) dùng chung token `--avatar-header`, căn giữa với padding trên/dưới/trái đồng đều, không chạm viền.
  - `.avatar-plugin-badge` (`Plugin`) gọn trong khung header và sát góc dưới-phải avatar danh sách hội thoại mà không đè chữ.
  - Logo SVG dùng cho `BotItem.avatar_url` phủ tràn viền trong khung avatar tròn.
- **AC2 (Tin nhắn cách đều avatar)**:
  - `.message-avatar-slot` khớp kích thước `Avatar` (`2.25rem`), `.message-row` dùng `gap: var(--space-3)` để bong bóng tin nhắn luôn có khe thoáng rõ ràng (`~12px`) so với avatar.
- **AC3 (Xóa icon Plugin và mô tả ở đầu trang Plugin)**:
  - Header trang quản lý `Plugin` chỉ hiển thị tiêu đề `"Plugin"` sạch sẽ; bỏ vòng tròn `PluginIcon` và dòng mô tả `"Tùy chỉnh giao diện, ứng dụng bên trái và lệnh"`.
- **AC4 (Theme Plugin tối/sáng đồng bộ 100% & HUD không che panel Thông tin)**:
  - `sync_plugins_to_dom` tự tính độ sáng tối của nền và màu nhấn để đồng bộ toàn bộ CSS tokens: `--bg-input`, `--bg-input-focus`, `--bg-active`, `--bg-selected`, `--bg-primary`, `--bg-warning`, `--bg-error`, `--bg-success`, `--bg-overlay`, `--border*`, `--on-accent`, `--msg-sent-text`, `--msg-sent-meta`, `color-scheme`.
  - Floating HUD tự động né sang trái `.details-panel` / `.partner-info-sidebar` khi panel bên phải mở.
- **AC5 (Cơ chế Tải & Gỡ cho Plugin)**:
  - `PluginMod` có trường `installed: bool`.
  - Trang quản lý Plugin tách 2 khu vực rõ ràng: **Đã tải** (hiển thị nút gạt bật/tắt, các nút mở/sửa và nút **`Gỡ`**) và **Kho Plugin** (hiển thị nút **`Tải`**).
  - Chỉ Plugin đã `installed && enabled` mới kích hoạt CSS/HUD/FX, hiện trên thanh điều hướng trái, danh sách Bot và gợi ý `/lệnh`.
- **AC6 (Sửa triệt để nút gạt `.switch`)**:
  - Cách ly `input[type="checkbox"].switch` khỏi `input:focus`; giữ hình pill cân đối, kích hoạt trơn tru không viền trắng/kép khi bấm hay hover.
