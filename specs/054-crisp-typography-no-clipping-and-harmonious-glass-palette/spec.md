# Feature Specification: Zero-Clipping Vietnamese Typography, Clean Readable Text Without Shadow Smudge, and Harmonious Translucent Glass Color System (Spec 054)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: Completed

## 1. Problem Statement & User Feedback

From the user's latest visual screenshots (`Bong bóng người khác` zoomed on `g` descender + Settings nav on translucent `Hoàng hôn`):
1. **Clipped `g` Descender & Ugly Blurry Text Shadow (`"chữ g nó cứ bị cắt cắt á ,và quá xấu ,..."`)**:
   - Single-line labels with `overflow: hidden; text-overflow: ellipsis; white-space: nowrap;` and tight `line-height: 1.35` (especially `.settings-label` blockified inside `.settings-row > .grow { display: flex; flex-direction: column; }`, `.settings-nav-item > span`, `.chat-item-name`, etc.) physically slice off the bottom loop of `g` (and descenders `p, q, y, j` / Vietnamese under-dots `ạ, ệ, ị, ọ, ụ, ự`).
   - `CONTRAST_ENGINE_JS` set `wpBusy = true` on **every** wallpaper preset when `transparency > 35%`, forcing `data-text-halo="true"` and applying a muddy offset dark `text-shadow` (`0 1px 1.5px rgba(0,0,0,0.34)`) behind crisp black text on smooth, high-contrast pastel wallpapers (`Hoàng hôn`, `Đại dương`, `Bạc hà`, `Cực quang`, `Giấy`) where APCA contrast is already effortlessly readable (`|Lc| > 72`).
2. **Discordant / Chalky / Opaque Interactive Colors Over Translucent Wallpapers (`"và các màu nó vẫn bị quá xấu"`)**:
   - Hover (`--bg-hover`) over colored translucent light wallpapers used `rgba(255, 255, 255, 0.46..0.64)`, producing a flat, chalky opaque cream/beige slab.
   - Active/selected pills (`--accent-soft`) used `rgba(224, 236, 254, 0.85)` + a blue inset outline border (`--adaptive-active-ring`), producing an opaque cold sky-blue plastic sticker that clashes sharply against warm wallpapers (`Hoàng hôn`, `Bạc hà`, photos).
   - Dark translucent active pills used `80–88%` opaque dark blocks instead of luminous translucent glass.
   - Light-mode preset wallpapers (`Hoàng hôn`, `Đại dương`, `Bạc hà`, `Cực quang`) composited over cool grey `#E9EDF3`, making warm gradients look dusty/muddy instead of fresh and radiant.

## 2. Acceptance Criteria

1. **Zero Descender / Diacritic Clipping Everywhere**:
   - Every single-line truncated text element (`.settings-label`, `.settings-nav-item > span:not(.chevron)`, `.chat-item-name`, `.chat-item-preview`, `.chat-header-title`, `.chat-header-meta`, `.sidebar-title`, `.sidebar-server-name`, `.category-name`, `.channel-row-name`, `.quick-label`, `.account-name`, `.user-status-name`, `.section-tab span:not(.section-badge)`, `.section-row .grow`, `.pinned-bar-preview`, `.suggest-sub`, `.result-name`, `.voice-participant-name`, `.composer-context-text`, `.truncate`) uses generous `line-height: 1.5` plus balanced vertical clip-box expansion (`padding-block: 0.18em; margin-block: -0.18em;`) so `g, p, q, y, j, ệ, ọ, ự, ế, ồ` never touch the clip edge while layout geometry remains 100% unchanged.
2. **Clean Text-Shadow Gating (APCA + True Image Complexity Only)**:
   - `wpBusy` is `true` **only** for genuinely complex/high-variance uploaded images (`c.wpKind === 'image' && (stdB > 0.14 || edgeB > 0.048 || (darkRatio > 0.16 && brightRatio > 0.16))`), NEVER for smooth preset gradients or solid colors.
   - `needsHalo` (`data-text-halo="true"`) activates **only** when `c.wpKind !== 'none' && alpha < 0.36 && (worstLc < 62 || (c.wpKind === 'image' && wpBusy))`.
   - Smooth high-contrast presets (`Hoàng hôn`, `Đại dương`, `Bạc hà`, `Giấy`, `Cực quang`, `Chấm bi`, `Đêm sao`) render **100% crisp, razor-sharp ink (`#000000` / `#FFFFFF`) with `text-shadow: none`** even at `72–85%` transparency!
   - Separate `data-framed-controls="true"` (`c.wpKind !== 'none' && alpha < 0.40`) keeps 0.00px size-stable inset frames on idle segmented buttons at high transparency without adding text-shadow.
3. **Harmonious Apple Liquid Glass / Arc-Caliber Translucent Color System**:
   - **Light Colored Translucent Surfaces**:
     - Hover (`--bg-hover`): airy sheer luminous glass (`rgba(255, 255, 255, 0.24..0.30)`) — translucent, silky, preserves wallpaper warmth, never a 56% chalky slab.
     - Active / Selected (`--accent-soft`): true luminous translucent glass with **hue-harmony awareness** — NO 85%-opaque cold blue sticker, NO boxy colored border ring!
       - Analogous hue (`hueDiff <= 65°`): fresh translucent accent-tinted frosted glass (`alpha ~ 0.46–0.56`, subtle accent tint `0.16`, soft white specular highlight).
       - Clashing / complementary hue (`hueDiff > 65°`, e.g., Blue accent over Pink `Hoàng hôn`): luminous frosted pearl glass (`rgba(255, 255, 255, 0.56–0.64)` with whisper ~`4%` accent clarity + delicate specular rim `inset 0 0 0 1px rgba(255, 255, 255, 0.42)`, `0 1px 2px rgba(15, 23, 42, 0.04)`).
     - Secondary buttons (`.btn-secondary`, `.btn-ghost`, `.btn-avatar-upload`): sheer luminous frosted glass (`0.36..0.46` idle -> `0.54..0.64` hover -> `0.66..0.74` active) with crisp specular white rim `inset 0 0 0 1px rgba(255, 255, 255, 0.40)`.
     - Framed segmented idle buttons (`data-framed-controls="true"`): ultra-sheer glass (`0.22..0.28` idle -> `0.38..0.46` hover) with crisp specular rim, maintaining exact `0.00px` size stability from frameless -> framed.
   - **Dark Translucent Surfaces**:
     - Hover (`--bg-hover`): sheer `rgba(255, 255, 255, 0.09)`.
     - Active (`--accent-soft`): luminous translucent accent glass (`rgba(vr, vg, vb, 0.28..0.34)` when harmonious, or frosted glass `rgba(255, 255, 255, 0.16)` softly tinted with accent, zero heavy 85% opaque dark mud block!) with subtle `inset 0 0 0 1px rgba(255, 255, 255, 0.12)` glass sheen.
     - Secondary buttons: sheer `rgba(255, 255, 255, 0.10)` -> `0.17` hover -> `0.24` active.
   - **Clean Radiant Light-Mode Preset Wallpapers**:
     - Light-mode `Hoàng hôn`, `Đại dương`, `Bạc hà`, `Cực quang` composite cleanly on `#FFFFFF` (not cool grey `#E9EDF3`) so warm pinks/peaches/mints stay vibrant, clear, and joyful.
4. **Zero Regressions**:
   - All 4 Playwright E2E suites (`spec048`, `spec050`, `spec052`, `spec053`) pass 100% with 0 page errors; updated `spec052` verifies zero `g`-descender clipping, zero shadow on smooth readable presets, halo activation on genuinely busy high-contrast uploaded images, 0.00px button size shift, and harmonious non-opaque translucent glass colors.
