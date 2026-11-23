# Spec 104 Technical Plan: Terms, Privacy Policy & Community Standards

## 1. Component Architecture
1. **New component `app/src/components/legal.rs`**:
   - `enum LegalDocKind { Terms, Privacy, Community }`
   - `LegalContent(kind: LegalDocKind)` rendering the crisp, numbered manifesto points with beautiful badges, icons, and cards.
   - `LegalModal(doc: Option<LegalDocKind>, on_close: EventHandler<()>)` for standalone popups on the login page.
   - `LegalPanel(kind: LegalDocKind)` for embedding directly in Settings tabs.

2. **`app/src/components/settings_view.rs`**:
   - Extend `enum AboutSub`:
     ```rust
     enum AboutSub {
         Overview,
         Terms,
         Privacy,
         Community,
         Guide,
     }
     ```
   - Update `SubTabBar` under `SettingsTab::About`:
     - `("overview", "Overview")`
     - `("terms", "Terms of service")`
     - `("privacy", "Privacy policy")`
     - `("community", "Community standards")`
     - `("guide", "Guide")`
   - Map query search words for terms, dieu khoan, privacy, chinh sach, bao mat, community, tieu chuan, cong dong to the respective subtab.

3. **`app/src/components/settings_about_dev.rs`**:
   - In `AboutOverviewPanel`, add a dedicated "Legal & ethics" card with 3 buttons linking to:
     - `on_open_terms`
     - `on_open_privacy`
     - `on_open_community`

4. **`app/src/pages/login.rs`**:
   - Render interactive links under `auth-legal`:
     - "Điều khoản dịch vụ"
     - "Chính sách bảo mật"
     - "Tiêu chuẩn cộng đồng"
   - Clicking opens the `LegalModal` dialog with smooth backdrop and close button.

5. **Style & CSS (`app/assets/main.css`)**:
   - Sleek minimalist styling for `.legal-card`, `.legal-item`, `.legal-item-num`, `.legal-item-text`, `.legal-modal-backdrop`, `.legal-modal-card`.

6. **Playwright E2E Test (`app/e2e/spec104-terms-privacy-community.mjs`)**:
   - Test login page links -> opens modal for Terms, verifies "Thứ nhất : Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!".
   - Log in and open Settings -> About.
   - Click subtabs "Điều khoản", "Chính sách bảo mật", "Tiêu chuẩn cộng đồng".
   - Verify content, styling, and responsiveness.
   - Take screenshots.
