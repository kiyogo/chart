# Spec 115 Technical Plan

## 1. Borderless UI & Hide Scrollbar Under Lightbox (`app/assets/main.css`, `app/web/canvas_scrollbars.js`)
- `app/assets/main.css`:
  - Enforce `border: none !important; outline: none !important; box-shadow: none !important;` on `.lightbox-img-frame`, `.lightbox-img`, `.ai-gen-overlay`, `.ai-gen-frost`, `.ai-gen-pct`, `.rich-image`, `.rich-image-stage`, `.rich-image-btn`, `.rich-image-full`, `.rich-image-enhance-corner`.
  - Add `body:has(.lightbox) #canvas-scrollbar-overlay { display: none !important; opacity: 0 !important; pointer-events: none !important; }` and `body:has(.lightbox) .panel-resizer { opacity: 0 !important; pointer-events: none !important; }`.
- `app/web/canvas_scrollbars.js`:
  - Change `overlayCanvas.style.zIndex` from `'99999'` to `'8000'`.
  - Add `isLightboxOpen()` guard in `render()`, `scroll`, `pointermove`, `pointerdown` to immediately clear canvas & hover states whenever `.lightbox` is present.

## 2. Strict End-to-End `.svg` Upload Blocking
- **Backend** (`web/backend/src/storage/router.rs`, `dedup_engine.rs`, `references.rs`):
  - Add `pub fn is_blocked_svg_upload(file_name: Option<&str>, mime: Option<&str>, content_type: Option<&str>, bytes: Option<&[u8]>) -> bool` checking `.svg`/`.svgz` extensions, `svg` MIME types, and `<svg` / `<?xml` + `<svg` magic bytes (plus unit tests).
  - Return `Err(AppError::BadRequest("svg_upload_not_allowed"))` in `ingest_media_bytes`, `handle_cdc_plan`, `handle_cdc_commit`, and `handle_dedup_check`.
- **Frontend** (`app/web/media_optimizer.js`, `app/src/platform/mod.rs`, `app/src/components/rich.rs`, `app/src/components/composer.rs`, `app/src/components/profile_modal.rs`, `app/src/components/settings_view.rs`, `app/src/pages/chat.rs`):
  - Add `isBlockedSvgFile(name, mime, u8)` in `media_optimizer.js` and enforce in `pickInstantPreview`, `optimizeFromInput`, `processSingleFile`, and `optimizePastedBytes`.
  - Block `.svg`/`.svgz`/`image/svg+xml`/`<svg` in `platform::read_selected_file_raw`, `process_selected_wallpaper_webp`, and `paste_watch`.
  - Narrow `<input type="file">` `accept` attributes from `image/*` to explicit raster MIME types (`image/png,image/jpeg,image/webp,image/avif,image/gif`).
  - Surface `"Không hỗ trợ gửi tệp SVG"` via `GLOBAL_MEDIA_NOTICE` -> `ChatPage` toast and composer `notice`.

## 3. Ultra-Sharp Real-ESRGAN v8 Shader (`app/src/storage/realesrgan_gpu.rs`, `app/web/ai_restorer.js`)
- Upgrade `GL_FIDELITY_CAS_FS` in `app/src/storage/realesrgan_gpu.rs`:
  - Sample 8-tap radius-2 ring (`(±2,0)`, `(0,±2)`, `(±2,±2)`) to compute true edge endpoints `[mn5, mx5]`.
  - Stronger reciprocal AMD FidelityFX CAS (`peak = -0.28 * uColor1.z * gate1`).
  - Stronger 8-tap isotropic radius-2 optical deconvolution (`0.94 * uColor1.z * gate2`) + subtle dark-ink stroke tightener, strictly clamped inside `[mn5, mx5]` (zero halos!).
  - Boost `uColor1.z` sharpness factor (`1.32` anime, `1.22` photo) while keeping 96% smooth Wavelet Color Anchor intact.
- Bump `PIPELINE_VERSION` to `'rust-realesrgan-upscayl-v8'` in `app/web/ai_restorer.js` and `cnnEngine` version tag in `wasm_bridge.rs`.

## 4. Build, Restart Backend & Verify
- Run backend & app `cargo test`.
- Rebuild backend (`cargo build --manifest-path web/backend/Cargo.toml`) and restart live server daemon on port `8085`.
- Build release WASM (`./app/build-web.sh release`).
- Run new `app/e2e/spec115-borderless-no-scrollbar-block-svg-ultra-sharp.mjs` asserting:
  1. Zero borders/box-shadows on Lightbox & AI overlay (`borderWidth: 0px`, `boxShadow: none`).
  2. Canvas scrollbar hidden (`display: none`, `zIndex <= 8000`) while Lightbox is open.
  3. `.svg` blocked on BOTH frontend picker (photos & attachments + renamed `.svg` as `.png`) AND backend `POST /api/attachments` (`400 svg_upload_not_allowed`), zero server object created.
  4. Real-ESRGAN v8 produces razor-sharp 4K output (`3344×1884`) with higher Tenengrad/Laplacian edge sharpness, SSIM `>= 0.99`, and low color drift — inspect real screenshot!
- Re-run Spec 114, Spec 113, Spec 112, Spec 101, and Clean AI UI regression suites.
