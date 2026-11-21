# Implementation Tasks — Spec 043: Wallpaper Auto-Compression (WebP, >1MB Compress, Max 2MB) & Clean Blur Label

- [x] **T001: Implement `process_selected_wallpaper_webp` in `app/src/platform/mod.rs`**
  - Add client-side canvas compression pipeline.
  - Scale max dimension to 2560px.
  - Convert to `image/webp`. If file size > 1MB, apply adaptive compression quality (0.82 / 0.75 / 0.60).
  - Enforce strictly <= 2MB max size limit (`2 * 1024 * 1024`), returning `Err("too_large")` if exceeded.

- [x] **T002: Update File Pickers in `settings_more.rs` and `chat_extras.rs`**
  - Use `platform::process_selected_wallpaper_webp("app-wp-file")` for app wallpaper.
  - Use `platform::process_selected_wallpaper_webp("chat-wp-file")` for chat wallpaper.

- [x] **T003: Clean Toggle Label in `settings_more.rs`**
  - Change `label: "Làm mờ kính khung (Frosted glass)"` to `label: "Làm mờ kính khung"`.
  - Update `app/e2e/spec042-transparency-toggle-blur.mjs` locator to match pure label `Làm mờ kính khung`.

- [x] **T004: Automated E2E Verification**
  - Create `app/e2e/spec043-wallpaper-compress-webp.mjs`.
  - Verify label is cleanly `Làm mờ kính khung`.
  - Test uploading image > 1MB, ensuring it is compressed into WebP and applied.

- [x] **T005: Rebuild, Regression Verification & Auto-Push**
  - Build release web WASM bundle (`./app/build-web.sh release`).
  - Run Playwright test suites (Spec 043, 042, 041, 040).
  - Build Linux desktop release binary and package `.deb`.
  - Commit with Conventional Commits and push to `origin/main` per AGENTS.md Rule #1.
