# Spec 105 Technical Plan: Clean Simple HTML Legal Documents & Product Vision

## 1. Architecture & Code Changes
1. **`app/src/components/legal.rs`**:
   - Add `LegalDocKind::Vision` ("Tầm nhìn sản phẩm").
   - Eliminate hero banner box (`.legal-doc-hero`).
   - Eliminate tags, order badges, and checkmark icons.
   - Strip all colons `:` and all parenthetical annotations `(...)`.
   - Implement clean simple HTML rendering:
     ```rust
     div { class: "legal-simple-doc",
         for item in items.iter() {
             div { class: "legal-simple-item",
                 div { class: "legal-simple-title", "{item.title}" }
                 div { class: "legal-simple-detail", "{item.detail}" }
             }
         }
     }
     ```
2. **`app/src/components/settings_view.rs`**:
   - Add `AboutSub::Vision` to `enum AboutSub`.
   - Update `SubTabBar` items to 6 items:
     - `("overview", "Overview")`
     - `("terms", "Điều khoản dịch vụ")`
     - `("privacy", "Chính sách bảo mật")`
     - `("community", "Tiêu chuẩn cộng đồng")`
     - `("vision", "Tầm nhìn sản phẩm")`
     - `("guide", "Guide")`
   - Map search query keywords for "vision", "tam nhin", "san pham" to `AboutSub::Vision`.
3. **`app/src/components/settings_about_dev.rs`**:
   - Add "Tầm nhìn sản phẩm" row in the Legal card with `on_open_vision` handler.
4. **`app/src/pages/login.rs`**:
   - Add "Tầm nhìn sản phẩm" link in `auth-legal-links`.
5. **`app/assets/main.css`**:
   - Add flat, clean, minimalist styles for `.legal-simple-doc`, `.legal-simple-item`, `.legal-simple-title`, `.legal-simple-detail`.
6. **Automated Verification**:
   - Playwright E2E script `app/e2e/spec105-clean-legal-and-product-vision.mjs`.
   - Verify no `:` or `(...)` exists in legal text.
   - Verify no `.legal-doc-hero` exists.
   - Verify "Tầm nhìn sản phẩm" is rendered and accessible.
   - Capture screenshots.
