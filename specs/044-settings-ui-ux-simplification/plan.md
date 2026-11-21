# Technical Plan — Spec 044: Ultra-Intuitive Settings UI/UX Simplification

## Architectural Approach
1. **Auditing and Replacing UI Strings**:
   - Audit all strings in `settings_view.rs`, `settings_more.rs`, `settings_extra.rs`.
   - Strip all ellipses `...` and remove all bracketed parenthetical expressions `(...)`.
   - Rewrite explanations to be short (1 sentence), concrete, and in plain everyday Vietnamese.
2. **Nav Reorganization (`settings_view.rs`)**:
   - Refine tab labels in `NAV` to everyday words:
     - "Khóa PIN" -> "Khóa ứng dụng"
     - "Lưu trữ" -> "Dung lượng"
     - "Trò chuyện" -> "Tin nhắn"
     - "Trả lời tự động" -> "Tự trả lời"
     - "Tùy chỉnh" -> "Lối tắt"
     - "Thư viện" -> "Hình dán"
     - "Studio" -> "Vẽ hình"
     - "Hướng dẫn" -> "Trợ giúp"
   - Keep group names clear: "Tài khoản", "Trò chuyện", "Tiện ích", "Mở rộng".
3. **Component Simplification**:
   - `settings_more.rs`:
     - Clean `ChatPrefsSettings`: short labels, simple paste options, clear descriptions.
     - Clean `NotificationSettings`: simple notification toggles, clear sound picker.
     - Clean `PrivacySettingsPanel`: clear contact permissions, simplified privacy text.
     - Clean `LockSettings`: simple password protection explanation, replace PBKDF2/salt tech jargon with plain Vietnamese ("Mật khẩu số bí mật", "Chỉ lưu trên máy này").
     - Clean `AutoReplySettings`: simple auto-reply text.
     - Clean `AdvancedLookSettings`: short descriptions on sliders, remove `(0px đến 32px)`, friendly reset button.
     - Clean `SyncMotionSettings`: simple device sync text.
   - `settings_view.rs`:
     - Clean `SettingsTab::Security`: replace raw algorithm strings (`ECDH P-256 / HKDF-SHA-256 / AES-256-GCM`, `ECDSA P-256 / SHA-256`) with user-friendly plain descriptions ("Mã hóa hai đầu chuẩn cao nhất", "Đang bật an toàn").
   - `settings_extra.rs`:
     - Clean `CustomizationSettings`: simple left rail shortcuts text.
4. **Automated Verification**:
   - Write Playwright E2E test `app/e2e/spec044-settings-ux-simplification.mjs`.
   - Inspect text across Settings tabs to verify:
     - Zero `...` in UI text.
     - Zero `(...)` in UI labels and descriptions.
     - Zero jargon keywords (`PBKDF2`, `salt`, `ECDH`, `AES-256`, etc.).
