# Spec 112 Technical Plan

## 1. Pure-Rust Deterministic Dimension Math (`app/src/components/rich.rs`)
- Extend `LightboxMedia`:
  - `pub width: u32` (wire/display width)
  - `pub height: u32` (wire/display height)
  - `pub orig_width: u32` (original width before wire downscale, `0` if unknown)
  - `pub orig_height: u32` (original height before wire downscale, `0` if unknown)
- Pure helper `compute_lightbox_target_dimensions(src_w, src_h, orig_w, orig_h, is_ai_eligible)`:
  - Preserves exact source aspect ratio (`w / h`).
  - If `is_ai_eligible` (`!is_sender_original && original_src.is_some()`): simulates the exact `ai_restorer.js` 4× output target (`min(src_max, 1080) * 4`, capped at `3840` on longest edge), and takes `max(orig, predicted_4x)` while preserving aspect ratio.
  - Otherwise (`is_sender_original` or non-AI): uses exact native `(orig_w, orig_h)` or `(src_w, src_h)` so native-sized images like `960×540` (Spec 078) and `800×600` (Spec 101) never upscale artificially!
- Pure helper `compute_lightbox_fit_box(final_w, final_h, vw, vh) -> (f64, f64)`:
  - `max_w = (vw * 0.92).min(1440.0).max(1.0)`
  - `max_h = (vh * 0.86).max(1.0)`
  - `scale = (max_w / final_w).min(max_h / final_h).min(1.0)`
  - Returns `((final_w * scale).round().max(1.0), (final_h * scale).round().max(1.0))`
- Pure helper `compute_chat_image_box(w, h) -> (u32, u32)`:
  - Fits `(w, h)` cleanly inside `352px × 352px` (`22rem × 22rem`) while preserving aspect ratio, preventing tall portraits from creating empty height gaps and preventing width collapse while loading.
- Synchronous viewport helper `read_viewport_size() -> (f64, f64)` using `web_sys::window()` on `wasm32` (fallback `(1440.0, 900.0)` for unit tests) so Frame 0 already knows exact `(vw, vh)` with zero async flash.
- Unit tests in `rich.rs` verifying:
  - `836×471` AI-eligible image fits `1440×811` in `1600×960` and `1214×684` in `1320×860` on Frame 0 (identical to post-4× `3344×1884`!).
  - `960×540` sender original stays `960×540` in `1600×960` (Spec 078 invariant!).
  - `800×600` sender original stays `800×600` (Spec 101 invariant!).
  - Toggle original <-> enhanced produces `0.0px` delta.

## 2. Zero-Flash Pre-Decode & Smooth Crossfade Reveal (`app/web/ai_restorer.js`, `app/src/components/rich.rs`, `app/src/components/details_panel.rs`)
- `app/web/ai_restorer.js`:
  - Add `predecodeImageUrl(url)` using `new Image(); img.decoding = 'async'; img.src = url; await img.decode().catch(() => {})` before resolving `restoreImageFromUrl` (both computed and IndexedDB cache paths).
- `app/src/components/rich.rs`:
  - Populate `width`, `height`, `orig_width`, `orig_height` when constructing `LightboxMedia` in `RichImage` and `SelfMediaTile` (`details_panel.rs`).
  - In `GlobalImageLightbox`:
    - Synchronously compute `(fit_w, fit_h)` from `media` + `viewport` signal (and `fallback_nat` only if `media.width == 0 || media.height == 0`).
    - Sync `bounds` directly to `(fit_w, fit_h, vw, vh)` whenever `(fit_w, fit_h, vw, vh)` changes — eliminating `use_effect(|cur_src|)` churn and `setTimeout(sendBounds, 80)` snap jitter!
    - Keep one clean rAF-debounced `window` `resize` listener only while Lightbox is open.
    - Frame-0 immediate enhancing overlay (`let is_lb_enhancing = upscaling_ai() || (media.auto_enhance_on_open && !media.ai_restored && !media.is_sender_original);`).
    - Smooth reveal state `lb_revealing`: on `'done'`, swap pre-decoded `src`, set `lb_revealing = true`, `upscaling_ai = false`, clear `lb_revealing` after `350ms` so `.ai-gen-overlay.is-revealing` dissolves smoothly at 60fps (`opacity: 1 -> 0`) then unmounts cleanly!
    - Put `style: "{img_style}"` (`transform: translate3d(...) scale(...) rotate(...)`) directly on `<img class="lightbox-img">` AND on `.ai-gen-overlay.lightbox-ai-gen` so zoom/pan/rotate track together seamlessly AND `spec078`/`spec101` assertions stay 100% green.
  - In `RichImage`:
    - Lock `.rich-image` container width & stage aspect-ratio deterministically from `compute_chat_image_box(data.w, data.h)`.
    - Add matching `ai_revealing` 340ms GPU opacity crossfade when live AI finishes.
- `app/src/storage/realesrgan_gpu.rs`:
  - Flush WebGL2 body layers in 4-layer batches (`chunk = 4`) on real GPU too (4 smooth non-blocking flushes on 16-layer anime model) so progress `%` updates cleanly without hanging at `6%` while keeping total runtime ~600ms.

## 3. Museum-Grade Compositor-Only AI Enhancement Overlay (`app/assets/main.css`)
- Remove `filter: blur(...)` and `transform: scale(1.06)` transitions from `.rich-image-full` and `.lightbox-img`.
- Remove `.ai-gen-dots-wave` `mask-position` animation and `filter: drop-shadow(...)`.
- Redesign `.ai-gen-overlay` layer stack inside `isolation: isolate; contain: layout paint;`:
  - `.ai-gen-frost`: static frosted crystal lens (`backdrop-filter: blur(22px) saturate(1.18) brightness(0.68)`, subtle dark radial depth veil, hairline inset luminous border). Cached ONCE by Chromium!
  - `.ai-gen-aurora`: sibling ABOVE frost — dual soft iridescent sapphire/indigo radial orbs drifting smoothly via GPU `transform: translate3d(...) scale(...)`.
  - `.ai-gen-dots-base`: subtle fine `1px` micro-dot pattern (`16px` pitch, low opacity `0.22`) with smooth radial edge feathering (`mask-image` static, NEVER animated!) so borders stay silky clean.
  - `.ai-gen-dots-wave`: sleek diagonal luminous light beam sweeping smoothly via GPU `transform: translate3d(-65%, 0, 0) -> translate3d(65%, 0, 0)`.
  - `.ai-gen-pct`: ultra-clean dark frosted capsule (`tabular-nums`, crisp luminous sapphire pulse indicator dot, zero `...`).
  - `.ai-gen-overlay.is-revealing`: `opacity: 0; transition: opacity 320ms cubic-bezier(0.22, 1, 0.36, 1); pointer-events: none;`.
- Enforce `.lightbox-img-frame { border-radius: 0; isolation: isolate; }` and `.lightbox-img { width: 100%; height: 100%; object-fit: contain; border-radius: 0 !important; }`.
- Respect `@media (prefers-reduced-motion: reduce)` and `html[data-reduce-motion="true"]`.

## 4. Verification Plan
1. `cargo test` unit tests for deterministic Lightbox sizing & pan clamping.
2. `./app/build-web.sh release` full WASM + CSS build.
3. Dedicated Playwright real-GPU verification script measuring exact `getBoundingClientRect()` pixel coordinates of `.lightbox-img-frame` and `.lightbox-img` at:
   - Frame 0 on Lightbox open (before/at start of AI),
   - Mid-enhancement,
   - Immediately after 4K completion,
   - After toggling Original <-> Enhanced,
   asserting `|Δw| <= 1px, |Δh| <= 1px, |Δx| <= 1px, |Δy| <= 1px` across ALL transitions, PLUS visual screenshot inspection of chat bubble enhancing, Lightbox enhancing, and 4K finished states!
4. Run ALL regression E2E suites:
   - `app/e2e/spec078-global-lightbox-centered-zoom-pan.mjs`
   - `app/e2e/spec101-infinite-drag-and-unrounded-lightbox.mjs`
   - `scratch/verify_clean_ai_ui_and_flows.mjs`
   - `app/e2e/spec111-hyper-dedup-cross-chat-ai.mjs`
   - `app/e2e/spec110-ultra-media-ai-cache.mjs`
5. Commit & push cleanly to `origin main`.
