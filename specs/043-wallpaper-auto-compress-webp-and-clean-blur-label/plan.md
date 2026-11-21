# Technical Plan — Spec 043: Wallpaper Auto-Compression (WebP, >1MB Compress, Max 2MB) & Clean Blur Label

## Architectural Overview
1. **Client-side Image Compression Pipeline**:
   - High-resolution user photos often weigh between 3MB and 15MB.
   - Using HTML5 Canvas + `createImageBitmap`, we can downsample ultra-large dimensions to a sensible 2K/4K max (2560px) and encode using the native browser `image/webp` encoder with configurable quality.
   - A multi-tier compression algorithm checks if `file.size > 1 * 1024 * 1024`:
     - Initial pass at quality `0.82`.
     - Secondary pass at quality `0.75` if resulting blob > 1MB.
     - Tertiary pass at quality `0.60` if resulting blob > 2MB.
     - If final blob is still > 2MB, returns `error: 'too_large'`.
   - The output mime type is strictly `"image/webp"`.
2. **Settings UI Polish**:
   - In `app/src/components/settings_more.rs`:
     - Change label to `"Làm mờ kính khung"`.
     - Update wallpaper file picker `onchange` handler to call `platform::process_selected_wallpaper_webp`.
   - In `app/src/components/chat_extras.rs`:
     - Update chat wallpaper file picker to call `platform::process_selected_wallpaper_webp`.
3. **E2E Testing**:
   - In `app/e2e/spec043-wallpaper-compress-webp.mjs`, create test covering:
     - Pure label check: `Làm mờ kính khung` exists, does not contain `(Frosted glass)`.
     - Uploading a synthetic large PNG image (> 1.2 MB), verifying it is compressed to WebP and accepted.
