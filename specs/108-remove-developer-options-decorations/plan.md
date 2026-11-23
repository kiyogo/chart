# Plan 108: Kế hoạch thực hiện xóa các chi tiết rườm rà ở Tùy chọn nhà phát triển

## 1. Thiết kế kỹ thuật
- **File**: `app/src/components/settings_about_dev.rs`
  - Bỏ `div { class: "settings-group-title", "Developer options" }`.
  - Cấu trúc lại thẻ `about-dev-card`:
    ```rust
    div { class: "settings-group",
        div { class: "about-dev-card",
            label { class: "about-dev-toggle-row",
                span { class: "about-dev-title", "Developer mode" }
                input {
                    r#type: "checkbox",
                    class: "switch",
                    aria_label: "Developer mode",
                    checked: dev_mode,
                    onchange: move |e| {
                        let on = e.checked();
                        build_taps.set(0);
                        prefs_ctx.update(|p| p.developer_mode = on);
                    },
                }
            }
        }
    }
    ```
  - Xóa bỏ `if dev_mode { div { class: "about-dev-footer", ... } }`.
  - Bỏ `BoltIcon` khỏi danh sách import icons.
- **File**: `app/assets/main.css`
  - Đảm bảo `.about-dev-toggle-row` có căn giữa dọc `align-items: center` và cách biệt `justify-content: space-between`.
  - Giữ CSS sạch sẽ, không có bất kỳ border-bottom hay border-top thừa nào.

## 2. Kiểm thử & Đóng gói
- Biên dịch thử nghiệm WebAssembly:
  `cargo check -p chatw-app --target wasm32-unknown-unknown`
- Đóng gói bản phát hành:
  `./app/build-web.sh release`
- Viết kịch bản Playwright E2E:
  `app/e2e/spec108-remove-developer-options-decorations.mjs`
  Chụp ảnh màn hình kiểm chứng lưu vào `verify_108/`.
- Commit và git push lên `origin main`.
