# Technical Implementation Plan: Spec 054

## Architecture & Files to Modify

1. **`app/src/platform/mod.rs` (`CONTRAST_ENGINE_JS`)**:
   - Add `hueDiffDeg(h1, h2)` helper (`Math.min(d, 360 - d)`).
   - Fix `wpBusy`: strictly `c.wpKind === 'image' && (stdB > 0.14 || edgeB > 0.048 || (darkRatio > 0.16 && brightRatio > 0.16))`.
   - Separate `needsFramedControls = c.wpKind !== 'none' && alpha < 0.40` from `needsHalo = c.wpKind !== 'none' && alpha < 0.36 && (worstLc < 62 || (c.wpKind === 'image' && wpBusy))`.
   - Set `data-framed-controls` attribute alongside `data-text-halo`.
   - Refine `--adaptive-text-halo`:
     - Light tone (only triggered on genuinely busy dark/mixed photos or `worstLc < 62`): tight crisp `0 0 1px rgba(0, 0, 0, ${coreA})` (no smudgy vertical offset drop shadow).
     - Dark tone: clean `0 1px 2px rgba(0, 0, 0, ${darkCoreA}), 0 0 4px rgba(0, 0, 0, ${darkSoftA})`.
   - Replace chalky opaque translucent control palette with Hue-Harmonized Liquid Glass tokens:
     - Compute `accentH` vs `wpH` (`domHex`), `isHarmonious = wpChroma < 0.10 || hueDiffDeg(accentH, wpH) <= 65`.
     - Light colored surface:
       - `--bg-hover`: `rgba(255, 255, 255, ${(0.24 + lumRamp * 0.06).toFixed(2)})`
       - `--bg-active`: `rgba(255, 255, 255, ${(0.38 + lumRamp * 0.08).toFixed(2)})`
       - `--bg-surface-hover`: `rgba(255, 255, 255, ${(0.32 + lumRamp * 0.08).toFixed(2)})`
       - `--accent-soft`: if `isHarmonious`, `makeCleanTint(effAccentHex, false, 0.16, 0.50 + lumRamp * 0.10)`; else clean luminous pearl glass `makeCleanTint(effAccentHex, false, 0.04, 0.56 + lumRamp * 0.08)`!
       - `--accent-soft-strong`: slightly higher alpha (`+0.10`) for crisp pressed state.
       - `--adaptive-active-ring`: delicate specular `rgba(255, 255, 255, 0.44)` (NEVER a boxy blue outline!).
       - `--adaptive-btn-bg/hover/active/ring`: airy translucent glass `0.38 -> 0.54 -> 0.66`, rim `rgba(255, 255, 255, 0.42)`.
       - `--adaptive-ctrl-bg/hover/ring` (when `needsFramedControls`): sheer `0.24 -> 0.40`, rim `rgba(255, 255, 255, 0.34)`.
     - Dark translucent surface:
       - `--bg-hover`: `rgba(255, 255, 255, 0.09)`
       - `--bg-active`: `rgba(255, 255, 255, 0.15)`
       - `--accent-soft`: luminous translucent accent crystal (`rgba(vr, vg, vb, 0.28 + lumRamp * 0.06)` when harmonious/dark-neutral, or clean frosted `rgba(255, 255, 255, 0.16)` with soft accent tint when over a bright-hue photo), ring `rgba(255, 255, 255, 0.14)`.
       - `--adaptive-btn-bg/hover/active/ring`: `rgba(255, 255, 255, 0.11 -> 0.18 -> 0.25)`, rim `rgba(255, 255, 255, 0.15)`.
       - `--adaptive-ctrl-bg/hover/ring` (when `needsFramedControls`): `rgba(255, 255, 255, 0.08 -> 0.15)`, rim `rgba(255, 255, 255, 0.14)`.

2. **`app/assets/main.css`**:
   - Add Vietnamese Descender & Diacritic Safety rule (`line-height: 1.5; padding-block: 0.18em; margin-block: -0.18em;`) across `.settings-label`, `.settings-nav-item > span:not(.chevron)`, `.chat-item-name`, `.chat-item-preview`, `.chat-header-title`, `.chat-header-meta`, `.sidebar-title`, `.sidebar-server-name`, `.category-name`, `.channel-row-name`, `.quick-label`, `.account-name`, `.user-status-name`, `.section-tab span:not(.section-badge)`, `.section-row .grow`, `.pinned-bar-preview`, `.suggest-sub`, `.result-name`, `.voice-participant-name`, `.composer-context-text`, `.truncate`.
   - Switch idle framed segmented button rules from `html[data-text-halo="true"]` to `html[data-framed-controls="true"]` while keeping zero-shift `border: none !important; box-shadow: inset 0 0 0 1px ...`.
   - Refine light-mode `preset-sunset`, `preset-ocean`, `preset-mint`, `preset-aurora` backgrounds to paint over crisp `#FFFFFF` base so pastel gradients look luminous and pure (keeping dark mode deep and atmospheric).

3. **E2E Verification (`app/e2e/spec052-visual-and-contrast-audit.mjs` + `app/e2e/spec053-themes-accents-frameless-header.mjs`)**:
   - Add automated verification of:
     - `.settings-label` (`Bong bóng người khác`) generous clip-box vertical breathing room (`lineHeight >= 1.45 * fontSize` AND `paddingBottom >= 2px` so `g` descender cannot be clipped).
     - Smooth preset wallpapers (`Đại dương`, `Hoàng hôn`) at `72%` and `80%` transparency have `data-text-halo="false"` and `text-shadow: none` (zero blurry shadow smudge!), while a genuinely busy high-contrast uploaded image activates `data-text-halo="true"`!
     - `data-framed-controls="true"` activates at `72%` transparency with exact `0.00px` button size shift.
     - Harmonious translucent glass hover (`alpha <= 0.34`, never chalky `0.54` slab) and translucent active pill (`alpha <= 0.68`, never opaque `0.85` cold-blue sticker) over `Hoàng hôn`!
   - Visually inspect zoomed Chromium screenshots of `Bong bóng người khác`, `Hoàng hôn` nav hover+active, `Đại dương`, `Đêm sao`, and `Sáng`/`Xám tro`/`Tối`/`Đen huyền`!
