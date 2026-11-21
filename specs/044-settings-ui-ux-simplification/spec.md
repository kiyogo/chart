# Feature Specification — Spec 044: Ultra-Intuitive Settings UI/UX Simplification (Zero Jargon, Zero Ellipses, Zero Parentheses, Short Clear Text)

## Problem Statement
Users requested a complete UI/UX overhaul of Settings ("cho dễ dùng nhất có thể cho người IQ 60"):
1. **Short, direct text**: Labels and explanations must be concise and instantly understandable without reading long paragraphs.
2. **Zero technical jargon**: Eliminate cryptographical / technical jargon (e.g., "băm có muối PBKDF2", "ECDH P-256 / HKDF-SHA-256 / AES-256-GCM", "ECDSA", "JSON raw string", "E2EE", "BCP-47"). Use clear everyday Vietnamese.
3. **Zero `...` and Zero `(...)`**: Strictly eliminate all temporary ellipsis dots (`...`) and unnecessary parenthetical brackets `(...)` from labels, buttons, and descriptions.
4. **Intuitive layout and grouping**: Categorize settings into logical, friendly sections:
   - **Tài khoản**: Hồ sơ, Quyền riêng tư, Bảo mật, Khóa ứng dụng, Thiết bị, Dung lượng, Đồng bộ, Sao lưu, Đám mây.
   - **Trò chuyện**: Giao diện, Tin nhắn, Thông báo, Tự trả lời, Lối tắt, Ngôn ngữ.
   - **Tiện ích**: Hình dán, Vẽ hình, Túi đồ, Xem mẫu.
   - **Mở rộng**: Bán hàng, Tiện ích, Trợ giúp.

## Acceptance Criteria
1. **Navigation Clarity**:
   - Tab labels must be short, friendly, and plain Vietnamese (e.g. "Giao diện", "Tin nhắn", "Khóa ứng dụng", "Thông báo", "Lối tắt", "Hình dán", "Trợ giúp").
   - Search keywords cover natural colloquial terms so users find any option in 1 second.
2. **Settings Content Simplification**:
   - `AdvancedLookSettings`:
     - Sliders have clear live values with short descriptions:
       - "Độ trong suốt khung trắng: {val}%" — "Kéo sang phải để nhìn rõ hình nền".
       - "Làm mờ kính khung" — "Tạo độ mờ kính cho các khung trong suốt".
       - "Mức độ làm mờ kính: {val}px" — "Kéo sang phải để khung mờ hơn" (no parentheses like `(0px đến 32px)`).
       - "Độ tối phủ lên ảnh: {val}%" — "Làm tối bớt hình nền để chữ dễ đọc".
       - "Làm mờ hình nền: {val}px" — "Làm mờ hình nền phía sau".
     - Button: "Khôi phục mặc định".
   - `ChatPrefsSettings`:
     - Short toggle labels: "Bấm hai lần để trả lời", "Nhấn Enter để gửi", "Gợi ý hình dán", "Gợi ý nhắc tên", "Gợi ý mặt cười", "Hiện ảnh đại diện", "Báo đang gõ chữ", "Báo đã xem".
     - Spreadsheet paste options: "Hỏi trước", "Dạng ảnh", "Dạng bảng", "Dạng chữ".
   - `NotificationSettings`:
     - Simple toggles: "Bật thông báo tin nhắn mới", "Hiện nội dung tin nhắn", "Báo tin nhắn nhóm", "Báo cuộc gọi đến", "Phát chuông khi có tin nhắn".
   - `PrivacySettingsPanel`:
     - Friendly questions: "Ai được kết bạn", "Ai được gọi", "Ai được mời vào nhóm".
     - Clean blocklist & message protection descriptions without technical cryptography terminology.
   - `LockSettings`:
     - Friendly text: "Khóa ứng dụng bằng số bí mật để người khác không xem trộm được tin nhắn của bạn trên máy này." (Zero PBKDF2 / salt buzzwords).
     - Buttons: "Khóa ngay", "Tạo mật khẩu", "Đổi mật khẩu", "Tắt khóa".
   - `SecuritySettings`:
     - Friendly cards: "Bảo vệ hai đầu chuẩn cao nhất", "Trạng thái: Đang bật an toàn", "Vân tay bảo mật".
     - Backup: "Lưu chìa khóa bảo mật", "Nhập chìa khóa bảo mật".
3. **Verification**:
   - Automated test suite `app/e2e/spec044-settings-ux-simplification.mjs`:
     - Verify zero `...` and zero `(...)` in Settings UI text.
     - Verify zero technical jargon in Settings UI descriptions.
     - Verify navigation tabs render cleanly and respond to clicks.
   - Run existing regression tests (Spec 040, 041, 042, 043).
   - Rebuild release web WASM bundle and Linux desktop `.deb` package.
   - Commit with Conventional Commits and auto-push to `origin/main` per AGENTS.md Rule #1.
