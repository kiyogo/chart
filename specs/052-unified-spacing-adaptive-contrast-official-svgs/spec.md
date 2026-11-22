# Feature Specification: Unified 8px Panel Spacing, Adaptive Contrast & Image Analysis Engine, Official Web SVG Logos & Chat Header Refinement (Spec 052)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: In Progress

## 1. Problem Statement & User Requirements

1. **Chat Item Spacing, Crisp 1px Nameplate Hover/Active Border & Larger List Avatar (Image 1)**:
   - Report the exact left/right pixel inset of `.chat-item` inside the chat sidebar (`8px`).
   - When a `.chat-item` has a chat frame decoration (`.has-nameplate`), hovering must NOT wash out the decoration with a grey fill; instead, it must show ONLY a crisp, thin (`1px`, never thick) translucent grey border (`inset 0 0 0 1px`), and clicking/selecting it must show ONLY a crisp, thin (`1px`) colored accent border (`inset 0 0 0 1px var(--accent)`).
   - Increase the avatar size in `.chat-item` so it expands closer to the top, bottom, left, and right edges of the item.

2. **Unified 100% Equal `8px` Left/Right/Top Panel Insets Across the Entire Web App (Images 2–8)**:
   - Every panel across the entire web app (`.nav-sidebar`, `.settings-nav`, `.settings-body`, `.feed-column`, `.meet-column`, `.notifications-page`, `.download-page`, `.search-page`, `.plugin-panel`, `.details-panel`) must have the exact same left, right, and top internal inset (`8px` / `var(--panel-inset)`) as the chat sidebar in Image 1 and Image 8.

3. **Larger, Bolder Settings Section Titles Matching "Tin nhắn" (Images 5, 7, 8, 9, 13)**:
   - All `.settings-group-title` headers in Settings must be larger and bolder (`font-size: var(--text-xl)`, `font-weight: var(--weight-bold)`, `color: var(--text-primary)`), matching the visual prominence of the `"Tin nhắn"` sidebar title in Image 8.

4. **Upgraded Adaptive Contrast & Wallpaper Image Luminance/Color Analysis Engine (Image 9)**:
   - Analyze both app wallpapers and chat wallpapers (including uploaded images, solid colors, and presets) for regional luminance, contrast variance, edge density, bimodal dark/light ratio, and dominant color tint.
   - When panel transparency increases past a threshold or over busy artwork, darken text ink (`#05070D` / `#111827`) and apply a multi-layer dark/dual-polarity ambient glow halo (`text-shadow`) behind text and controls so no user customization ever makes text hard to read.

5. **Left Rail (`.app-rail`) Transparency & Blur Synchronization (Image 10)**:
   - Remove the hardcoded `rgba(255, 255, 255, 0.78)` scrim override on `.app-rail` and bind `.app-rail` directly to `--bg-primary`, `--panel-opacity`, and `--panel-blur` so it stays 100% synchronized with `.settings-nav` and `.settings-body`.

6. **Full-Bleed Color Picker Swatch Button, Left-Aligned "Mặc định" Button & Framed Buttons Everywhere (Images 5, 11, 12)**:
   - Remove the inner black border and inner padding inside `<input type="color">` so the swatch button renders 100% full-bleed with the selected color.
   - Place the `"Mặc định"` reset button to the **left** of the color picker button rather than the right.
   - Convert all frameless `.btn-ghost` buttons across the entire web app to framed buttons matching `.btn-secondary`.

7. **Remove All Guide Banners & Short Descriptions + Ultra-Concise Labels (Images 5 & 13)**:
   - Remove the 3-step cloud guide banner (`"Sao lưu đám mây riêng tư trong 3 bước..."`) and all explanatory `.settings-sublabel` / `.settings-row-desc` descriptions across the entire web app, and shorten all setting labels to their most concise form.

8. **Authentic Official Web SVG Brand Logos & Complete 33-Language SVG Flags (Images 6 & 13)**:
   - Replace hand-drawn approximations in `app/src/components/brand_logos.rs` with authentic official vector SVG paths fetched from Simple Icons / Flag-Icons / Wikimedia (Google Drive, OneDrive, Dropbox, Nextcloud, WebDAV, Amazon S3, Cloudflare R2, MinIO, NAS, Apple iCloud, Linux Tux, Windows, macOS, Android, iOS, Chrome, Firefox, Safari, Edge, Brave, Opera, Google, Passkey, and all 33 language flags so zero language tiles fall back to grey text codes).

9. **Non-Overlapping Date Separator & Year Formatting Rule in Chat (Image 14)**:
   - Fix `.date-separator` stacking/overlapping bug by removing `position: sticky; top: 0;`.
   - Format dates in the current year as `"{day} tháng {month}"` (e.g., `"20 tháng 11"`) and dates in previous/other years as `"{day} tháng {month} năm {year}"` (e.g., `"22 tháng 11 năm 2025"`).

10. **Self-Chat Inline Device Pills & True Floating Chat Header (Zero Top Bar)**:
    - Remove `"Thiết bị:"` label and the separate `.device-strip` row below `.chat-header`; render the device pills inline inside `.chat-header` to the right of the `"Chính mình"` title block with clean spacing.
    - Remove the full-width horizontal top header bar and double-white `::after` stripe completely in chat views (`.chat-column:has(> .chat-messages)`): float `.chat-header` (`position: absolute; top: var(--panel-inset); pointer-events: none; background: transparent`) over full-height `.chat-messages` (starting at `top: 0`), rendering the left identity group (avatar + name + E2EE), inline device chips, and right actions group as sleek floating frosted glass capsules backed by a soft progressive top mist (`::before` with `linear-gradient`, `backdrop-filter: blur(14px)`, and `mask-image` mirroring `.composer::before` at the bottom). Non-chat views (e.g., Enterprise `Tổng quan`) remain in normal flow without overlap.

11. **Meet Panel (`.meet-column`) Bottom Rounded Corners & Viewport Gap**:
    - Ensure `.meet-column` never touches the bottom of the browser viewport: restore `border-radius: var(--panel-radius)` on all 4 corners and `margin-bottom: 0` so it sits spaced away from the bottom edge just like `.app-rail` and normal menu panels.

12. **Zero-Shift Button Framing (`0.00px` Size Stability Across Frameless -> Framed Transitions)**:
    - When any button (`.segmented button`, `.btn-secondary`, `.btn-ghost`, `.btn-small-secondary`, `.btn-avatar-upload`) transitions between frameless and framed states (e.g., dragging panel transparency across the framing threshold), its width and height MUST NOT change by even `1px` (`0.00px` delta). Never toggle CSS `border-width`; render all button frames strictly via `box-shadow: inset 0 0 0 1px ...` with `border: none`.

13. **Harmonious Luminous Frosted Button & Control Palette Over Customized Wallpapers**:
    - Eliminate muddy dark-alpha (`10%`–`16%` black) fills on `.btn-secondary`, `.btn-ghost`, and `--bg-hover` over colored or photo wallpapers, and eliminate harsh dark-grey borders (`rgba(15, 23, 42, 0.18)`) on framed segmented options.
    - Replace them with smooth, wallpaper-harmonized luminous frosted glass surfaces (`--adaptive-btn-bg`, `--adaptive-btn-hover`, `--adaptive-btn-active`, `--adaptive-btn-ring`, `--adaptive-ctrl-bg`, `--adaptive-ctrl-hover`, `--adaptive-ctrl-ring`) computed from the wallpaper's dominant chroma and effective surface luminance.

14. **Crystal-Clear, Non-Muddy Active / Selected / Clicked Pill & Button Colors Over Any Wallpaper**:
    - Prevent low-alpha complementary hue cancellation (e.g., `16%` blue over `84%` warm pink `Hoàng hôn` collapsing into muddy grey-purple `#CBBED3`) on `.settings-nav-item.active`, `.segmented button.active`, `.wallpaper-tile.active`, `.theme-card.active`, `.chat-item.selected`, `.channel-row.selected`, `.rail-item.active`, and pressed (`:active`) button states.
    - Pre-normalize the active/pressed accent tint in perceptual space against pure white (light tone) or deep slate (dark tone) at high frosted clarity with a delicate inset accent rim (`--adaptive-active-ring`), while keeping Spec 048 / Spec 050 pure-white opaque and lightly-translucent tokens 100% intact.
    - Keep `--accent-soft` airy and delicate (`9%` in light mode, `15%` in dark mode, `10%` weight in translucent light mode, `18%` in translucent dark mode) so active/selected pills never look heavy, dark, or loud.
