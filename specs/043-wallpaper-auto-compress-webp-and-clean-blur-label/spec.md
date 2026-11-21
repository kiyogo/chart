# Feature Specification — Spec 043: Wallpaper Auto-Compression (WebP, >1MB Compress, Max 2MB) & Clean Blur Label

## Problem Statement
1. **Uncompressed Wallpaper Uploads Exceeding Limits**:
   - In previous versions, selecting a high-resolution wallpaper image larger than 2MB failed with the error banner "Ảnh lớn hơn 2 MB" because files were read raw without compression.
   - User requirement:
     - If an image is larger than 1MB (`file.size > 1 MB`), automatically compress it down into high-quality WebP format (`.webp`).
     - Maximum allowed size is strictly 2MB; images exceeding 2MB even after compression or oversized images are rejected with "Ảnh lớn hơn 2 MB".
     - The output format must be `.webp`.
2. **Remove English Parenthetical Label "(Frosted glass)"**:
   - The toggle switch currently displays `Làm mờ kính khung (Frosted glass)`.
   - User requested: "xóa cái (fro ....) đi chứ" — remove the `(Frosted glass)` English text and leave pure, clean Vietnamese: `Làm mờ kính khung`.

## Acceptance Criteria
1. **Auto-Compression & WebP Encoding for Custom Wallpapers**:
   - Implement `process_selected_wallpaper_webp(input_id: &str) -> Result<(Vec<u8>, String), String>` in `platform/mod.rs`.
   - When an image is picked:
     - Decode via canvas / `createImageBitmap`.
     - Automatically scale dimensions if max dimension exceeds 2560px for optimal screen display.
     - Compress to `image/webp` with adaptive quality (0.82 / 0.75 / 0.60) if file > 1MB.
     - Strict size validation: ensure final payload is at most 2MB (`2 * 1024 * 1024` bytes).
     - Format guaranteed to be `image/webp` with extension `.webp`.
   - Used by both app wallpaper upload (`app-wp-file`) in `settings_more.rs` and chat wallpaper upload (`chat-wp-file`) in `chat_extras.rs`.
2. **Clean Pure Vietnamese Toggle Label**:
   - Change toggle label in `settings_more.rs` from `Làm mờ kính khung (Frosted glass)` to `Làm mờ kính khung`.
3. **Automated Verification**:
   - Playwright test verifying:
     - The toggle row displays purely `Làm mờ kính khung` without `(Frosted glass)`.
     - Selecting an image > 1MB triggers client compression into WebP and uploads smoothly if <= 2MB.
     - Files exceeding 2MB after compression correctly trigger the "Ảnh lớn hơn 2 MB" error.
   - Regression suites for Spec 040, 041, 042 continue to pass 100%.
   - Rebuild release web WASM bundle and Linux desktop `.deb` package.
   - Commit with Conventional Commits and push to `origin/main` per AGENTS.md Rule #1.
