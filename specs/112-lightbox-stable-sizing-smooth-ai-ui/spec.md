# Spec 112: Deterministic Zero-Jump Lightbox Sizing, Jitter-Free Media Rendering & Refined AI Enhancement Overlay

## 1. Problem Statement
Users currently experience three severe visual/UX defects when viewing and enhancing images in ChatW:
1. **Lightbox Frame Size Jump (Small -> Huge Pop)**: Clicking an un-enhanced wire AVIF image (e.g. `836×471`) opens the Lightbox at its tiny intrinsic wire size (`836×471`) centered inside a huge dark viewport. The instant Real-ESRGAN 4× (`3344×1884`) finishes, the Lightbox suddenly jumps/explodes to full viewport fit (`~1440×811` / `1214×684`). Toggling between the enhanced image and original compressed image via the SVG button also causes the photo to shrink and re-expand violently.
2. **Pervasive UI Jitter, Flicker & Stutter**:
   - Frame-0 flash: opening an un-enhanced history image renders raw unblurred tiny pixels for 1–2 frames before snapping the blur+overlay on.
   - Animating CSS `mask-position` + `filter: drop-shadow` on `.ai-gen-dots-wave` forces expensive main-thread repaints during heavy WebGL2 GPU work, causing visible stutter.
   - Animating `filter: blur(16px) -> blur(0)` + `transform: scale(1.06) -> scale(1.0)` on `<img class="lightbox-img">` / `<img class="rich-image-full">` simultaneously with swapping to an undecoded 4K JPEG causes scale bounce, blank decode flash, and main-thread blur re-rasterization jank.
   - `GlobalImageLightbox` re-fires `use_effect(use_reactive!(|cur_src| ...))` on every `src` swap, re-running DOM measurement timers and clamping pan/bounds mid-transition.
   - `RichImage` chat bubbles lack explicit pre-computed pixel width (`width: min(Wpx, 100%)`), allowing bubbles to collapse/expand between placeholder, wire AVIF, and 4K image states.
   - `transform` was placed on `.lightbox-img-frame` instead of `.lightbox-img` (breaking Spec 078 / Spec 101 transform invariants) and `.lightbox-img-frame` applied `border-radius: 6px` instead of unrounded `0px` (Spec 101).
3. **Harsh / Ugly AI Enhancement Loading Visuals**:
   - Large high-contrast white/blue polka dots over muddy grey blur with harsh edge clipping look noisy and unpolished compared to a clean frosted-glass aurora lens aesthetic.

## 2. Goals & Acceptance Criteria
1. **Deterministic Frame-0 Final Sizing (`0.0px` Jump)**:
   - Before painting frame 0 of `GlobalImageLightbox`, compute the exact final post-enhancement target dimensions `(final_w, final_h)` from `(width, height, orig_width, orig_height, can_ai_enhance)` and fit them synchronously into the live viewport (`min(92vw, 1440px) × 86vh`) using `web_sys::window()` (with live `resize` updates).
   - Lock `.lightbox-img-frame` to exact `width: {fit_w}px; height: {fit_h}px;` and `.lightbox-img` to `width: 100%; height: 100%; object-fit: contain; border-radius: 0;`.
   - Sender originals (e.g. `960×540` in Spec 078, `800×600` in Spec 101) keep their exact native size when smaller than viewport limits, preserving 100% backward compatibility with Spec 078 & Spec 101.
   - **Metric**: `|Δwidth| <= 1px`, `|Δheight| <= 1px`, `|Δx| <= 1px`, `|Δy| <= 1px` across: (1) Frame-0 Open -> (2) Mid-Enhancement -> (3) Post-4K Completion -> (4) Toggle Original <-> Enhanced!
2. **Zero-Jitter, Zero-Flash, 60fps Compositor-Only Rendering**:
   - Frame-0 immediate overlay activation when `auto_enhance_on_open` is true (zero raw blurry flash).
   - Off-DOM `img.decode()` pre-decoding in `ai_restorer.js` before swapping `src` into DOM (zero blank decode flash).
   - Static isolated frosted layer (`isolation: isolate; backdrop-filter: blur(22px)`) + sibling GPU-composited animations (`transform: translate3d(...)` and `opacity` ONLY — ZERO `mask-position`, ZERO `filter` animation, ZERO `scale(1.06)` bounce on `<img>`).
   - Silky `320ms cubic-bezier(0.22, 1, 0.36, 1)` opacity crossfade reveal (`is-revealing`) when 4K finishes, then clean DOM unmount at `360ms`.
   - Smooth 4-layer GPU flushes in `realesrgan_gpu.rs` so progress `%` advances smoothly without freezing at `6%` while keeping full speed (~600ms).
   - Pre-computed exact chat bubble box (`width` + `aspect-ratio` + `isolation: isolate; contain: layout paint;`) in `RichImage` so chat bubbles never shift height or scroll position.
   - Apply `transform: translate3d(...) scale(...) rotate(...)` directly on `.lightbox-img` (and mirrored on `.lightbox-ai-gen` when active) with `border-radius: 0` so Spec 078 & Spec 101 zoom/pan/rotate/unrounded guarantees remain 100% intact.
3. **Refined, Modern AI Enhancement Overlay Visuals**:
   - Rich deep frosted obsidian veil letting the real photo's warm colors glow softly underneath without washed-out grey soup or bleeding edges.
   - Subtle, edge-feathered fine `1px` micro-dot field (never loud coarse polka dots) + slow gliding translucent aurora glow + sleek diagonal luminous light sweep (GPU `transform: translate3d` only) + hairline inner light rim.
   - Minimal frosted glass `tabular-nums` progress pill (zero `...`, zero AI/model text clutter).
   - Lightbox header remains strictly minimalist: filename ONLY on the left + single custom SVG toggle icon + zoom/rotate/download/close on the right.
