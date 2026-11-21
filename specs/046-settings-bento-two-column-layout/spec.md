# Spec 046: Settings Bento 2-Column Layout, Equal 1/3 Edge Spacing & IQ-60 Simplicity

## Problem Statement
1. **Huge Wasted Empty Space on the Right Half**: `.settings-content` was capped at `max-width: 46rem` (`~690px`) in a single vertical stack, leaving over 50% of `.settings-body` completely blank on standard desktop screens (`1440px+`), as shown in the user's screenshot of the `Trò chuyện` tab.
2. **Oversized & Uneven Edge Margins**: `.settings-body` had `padding: var(--space-8) clamp(var(--space-4), 4vw, var(--space-12))` (`30–32px` top, `45–48px` left/right), pushing content far away from the left/right edges while top/left/right spacing was uneven.
3. **Remaining Technical Jargon & Parentheses in Secondary Settings Tabs**: Tabs like `Lưu trữ`, `Đồng bộ`, `Sao lưu`, `Đám mây`, and `Thiết bị` still contained technical buzzwords (`OPFS`, `IndexedDB`, `SigV4`, `Multipart`, `AES-256-GCM`, `E2EE`, `delta`, `TOTP`, `PKCE`) and bracketed notes `(...)`.

## Goals & Acceptance Criteria
1. **Exact 1/3 Edge Spacing & 100% Equal Top = Left = Right Margins**:
   - Reduce `.settings-body` padding from `3rem` (`45–48px`) to exactly `1/3`: `1rem` (`15–16px`, `var(--space-4)`) uniformly on top, left, right, and bottom (`padding: var(--space-4)`).
   - Remove `.settings-content` `max-width: 46rem` cap (`max-width: none; width: 100%;`) and remove top offset on `.settings-page-title` so measured distances from `.settings-body` inner edges satisfy `top === left === right` (within `±1px` pixel accuracy!).
2. **Balanced 2-Column Side-by-Side Bento Grid (Left & Right Settings Columns)**:
   - On desktop viewports (`>= 960px`), organize every settings page into clean Left and Right balanced columns (`.settings-bento-cols` / 2-column CSS Grid `repeat(2, minmax(0, 1fr))` with uniform `var(--space-4)` gap) so BOTH left and right sides hold meaningful, logically grouped settings with ZERO blank empty right half and minimal scrolling:
     - **`Trò chuyện`**: Left = *Khi trò chuyện* (6 quick toggles); Right = *Trạng thái* (2 toggles) + *Dán bảng tính* (segmented picker).
     - **`Giao diện`**: Left = *Chủ đề*, *Màu nhấn*, *Chữ và mật độ*, *Màu sắc nâng cao*, *Hình dạng và bố cục*; Right = *Xem trước* (live right next to colors/theme!), *Hình nền trò chuyện*, *Hình nền toàn ứng dụng* + *Khôi phục mặc định*.
     - **`Thông báo`**: Left = *Thông báo*; Right = *Âm thanh*.
     - **`Quyền riêng tư`**: Left = *Ai có thể liên hệ với bạn*; Right = *Danh sách chặn* + *Bảo vệ tin nhắn*.
     - **`Tài khoản`**: Top full-width compact Profile Card; Left = *Thông tin tài khoản*; Right = *Mật khẩu và bảo mật* + *Thông tin định danh*.
     - **`Khóa ứng dụng`**: Left = *Khóa ứng dụng* (status/toggle/timeout/lock now); Right = *Đặt / Đổi mật khẩu khóa*.
     - **`Trả lời tự động`**: Left = *Bật & thời gian trả lời*; Right = *Nội dung tin nhắn tự động*.
     - **`Tùy chỉnh`**: Left = *Thanh lối tắt* + *Thông báo và hiệu ứng*; Right = *Đồng bộ* + *Ảnh đại diện*.
     - **`Bảo mật`**: Left = *Xác minh 2 bước* + *Mã hóa bảo vệ*; Right = *Sao lưu chìa khóa* + *Xuất dữ liệu* + *Chuyển tài khoản*.
     - **`Thiết bị`**: Left = *Thiết bị tin cậy*; Right = *Đăng xuất*.
     - **Other tabs (`Lưu trữ`, `Đồng bộ`, `Sao lưu`, `Đám mây`, `Ngôn ngữ`, `Thư viện`, `Vẽ hình`, `Kho đồ`, `Xem thử`, `Kinh doanh`, `Tiện ích`, `Hướng dẫn`)**: Full-width responsive grid filling left-to-right evenly with equal `1rem` padding on top/left/right.
   - On narrow/mobile screens (`< 960px`), gracefully collapse to 1 clean column.
3. **Ultra-Simple IQ-60 Friendly Copy Everywhere**:
   - Strip ALL remaining technical jargon (`OPFS`, `IndexedDB`, `SigV4`, `Local CAS`, `AES-256-GCM`, `E2EE`, `delta`, `TOTP`, `PKCE`, `KDATA`) and ALL `(...)` parentheses and `...` ellipses across EVERY settings tab (`settings_view.rs`, `settings_more.rs`, `settings_extra.rs`, `storage_ui.rs`, `backup_cloud_ui.rs`).
   - Keep labels ultra-short, visual, scannable, and flat minimalist (borderless & shadowless).
4. **Full Verification**:
   - Playwright E2E test (`app/e2e/spec046-settings-bento-layout.mjs`) verifying pixel-exact `top === left === right === 15–16px` symmetry, 2-column side-by-side placement (`rightCol.left > leftCol.right`), zero empty right-half waste (`contentRight` within `18px` of `bodyClientRight`), zero jargon, zero `...`, zero `(...)`, plus visual screenshot inspection across multiple tabs.
   - Pass all existing regression suites (`Spec 028`, `040`–`045`).
