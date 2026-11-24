# Spec 115: Borderless Lightbox/Media UI, Hide Scrollbars Under Lightbox, Strict SVG Upload Blocking & Ultra-Sharp Real-ESRGAN v8

## 1. Problem Statement
1. **Unwanted Borders / Frame Outlines**: Lightbox enhancing overlay (`.ai-gen-frost`, `.ai-gen-pct`), Lightbox photo shadow, and chat-image corner icon still show subtle visible hairline borders (`box-shadow: inset 0 0 0 1px ...`, `border: 1px solid ...`).
2. **Scrollbar Leaking Over Open Lightbox**: `canvas_scrollbars.js` mounts `#canvas-scrollbar-overlay` with `z-index: 99999` (above `.lightbox` at `9999`) and reads underlying scroll containers through `elementsFromPoint`, causing a vertical scrollbar to appear right on top of the open Lightbox.
3. **SVG Uploads Allowed**: File/image pickers (`accept="image/*"`, attachments picker), paste watcher, `media_optimizer.js`, and backend upload/dedup endpoints still allow `.svg`/`.svgz`/`image/svg+xml` files to be uploaded and sent.
4. **Stronger Real-ESRGAN Image Sharpening**: The 4× Real-ESRGAN post-pass (`GL_FIDELITY_CAS_FS`) was limited by 3×3 neighborhood clamping `[mn2, mx2]`, leaving soft 2–3px transitions less crisp than possible.

## 2. Goals & Acceptance Criteria
1. **Zero Borders / Frame Lines**:
   - Remove all borders, inset border shadows, outlines, and drop shadows on `.lightbox-img-frame`, `.lightbox-img`, `.ai-gen-overlay`, `.ai-gen-frost`, `.ai-gen-pct`, `.rich-image`, `.rich-image-stage`, `.rich-image-btn`, `.rich-image-full`, and `.rich-image-enhance-corner`.
2. **Zero Scrollbar / Resizer Leak While Lightbox Is Open**:
   - Lower `#canvas-scrollbar-overlay` z-index to `8000` (strictly below `.lightbox` `9999`).
   - Suppress scrollbar hover/scroll/pointer interception and clear canvas immediately whenever `.lightbox` is mounted in DOM.
   - Add CSS fail-safe `body:has(.lightbox) #canvas-scrollbar-overlay { display: none !important; }` and hide `.panel-resizer` while Lightbox is open.
3. **Strict End-to-End `.svg` Send/Upload Block (Frontend + Backend)**:
   - Reject `.svg`, `.svgz`, `image/svg+xml`, and `<svg` / XML-SVG magic bytes on BOTH client (`media_optimizer.js`, `platform::read_selected_file_raw`, `paste_watch`, `ComposerTools`, raster-only `accept` lists) AND server (`ingest_media_bytes`, `handle_cdc_plan`, `handle_cdc_commit`, `handle_dedup_check`).
   - Show crisp user feedback `"Không hỗ trợ gửi tệp SVG"` (zero `...`) without sending or creating any server object.
   - Built-in app vector `.svg` UI icons and logos remain 100% intact.
4. **Ultra-Sharp Real-ESRGAN v8 (`GL_FIDELITY_CAS_FS`) Without Altering Original**:
   - Upgrade `GL_FIDELITY_CAS_FS` with 8-tap radius-2 neighborhood endpoints `[mn5, mx5]`, stronger reciprocal AMD FidelityFX CAS (`peak = -0.28`), isotropic radius-2 deconvolution (`0.94`), and adaptive dark-ink line tightening — strictly clamped inside `[mn5, mx5]` (zero ringing halos!) while preserving the 96% Wavelet Low-Frequency Color Anchor.
   - Bump `PIPELINE_VERSION` to `'rust-realesrgan-upscayl-v8'` so caches refresh automatically.
   - Verify mathematically higher edge sharpness with preserved SSIM `>= 0.99` and ultra-low color drift.
