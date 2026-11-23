# Spec 098: Single Solid Gray Skeleton Loading Across All Views

## 1. Problem Statement & User Request
- **User Request**:
  > "tất cả chỉ hiện đúng 1 màu xám cho khung xương load thôi nhé ,sửa lại toàn bộ đi"
- **User Attachment**:
  - `media_1795445635420_134c50e4.png`: Shows the sidebar conversation list, story avatar circles, and rail profile avatar rendering skeletons with a 2-tone shiny linear gradient (`rgba(15, 23, 42, 0.06)` to `0.13` and back). This gradient created noticeable white stripe highlights across circles and lines, looking glossy/striped rather than clean, flat, and uniform.
- **Requirement**:
  - Eliminate all multi-tone linear gradients, sweeping shine highlights, and striped reflections on loading skeletons.
  - Skeletons across all views (stories, conversations, message bubbles, avatars, details panel, app rail) must display **EXACTLY 1 SOLID FLAT GRAY COLOR** (`--bg-skeleton: #E5E7EB` / `#E2E8F0` in light mode, `#2D3748` in dark mode).
  - Use smooth, uniform whole-element opacity breathing (`skeleton-pulse`) so every pixel of each skeleton element is always the exact same solid flat color with zero gradient bands.

## 2. Technical Solution
1. **CSS Variables (`app/assets/theme.css`)**:
   - Add `--bg-skeleton: #E5E7EB;` for light mode and default `:root`.
   - Add `--bg-skeleton: #2D3748;` for dark mode (`[data-theme="dark"]`, `html[data-surface-tone="dark"]`).
   - Add `--bg-skeleton: #E5E7EB;` for light surface tone on dark wallpapers (`html[data-surface-tone="light"]`).
2. **Unified Solid Gray Styles (`app/assets/main.css`)**:
   - Replace gradient definitions on `.skeleton-shimmer, .skeleton-box, .skeleton-circle, .skeleton-text, .skeleton-avatar`:
     - `background-color: var(--bg-skeleton, #E5E7EB) !important;`
     - `background-image: none !important;`
     - `background: var(--bg-skeleton, #E5E7EB) !important;`
   - Redefine `@keyframes skeleton-pulse` (and alias `@keyframes skeleton-shimmer`) to animate whole-element `opacity` between `1` and `0.55`:
     - Guarantees 100% color uniformity across every pixel at all times.
   - Respect `prefers-reduced-motion`.
3. **Verification**:
   - Recompile release WebAssembly bundle with `./app/build-web.sh release`.
   - Run Playwright E2E test verifying solid gray background (`#E5E7EB` / `rgb(229, 231, 235)`) and zero gradient `background-image: none`.
   - Capture screenshots and push to `origin main`.
