# Technical Implementation Plan: Spec 055

1. **`app/src/platform/mod.rs` (`CONTRAST_ENGINE_JS`)**:
   - Fix `preset-aurora` brightness/chroma metrics (`meanB = 0.94, wpChroma = 0.14`), `preset-sunset` (`meanB = 0.86`), `preset-ocean` (`meanB = 0.87`), `preset-mint` (`meanB = 0.91`) to accurately match their clean radiant light bases.
   - Gate `needsFramedControls` strictly on genuinely busy uploaded photos at high transparency (`wpBusy && alpha < 0.38`), eliminating 18 cluttered grey boxes on smooth wallpapers while keeping `0.00px` size shift on busy photos.
   - Replace white-washed `--accent-soft` with true vivid translucent Accent color:
     - On all smooth wallpapers & themes (`!wpBusy`): Light tone `rgba(vr, vg, vb, 0.14..0.18)` (`--accent-soft-strong` `+0.06`), Dark tone `rgba(vr, vg, vb, 0.24..0.30)`, `--adaptive-active-ring: transparent` (zero boxy border!).
     - On genuinely busy uploaded photos at high transparency (`wpBusy && alpha < 0.55`): Light tone vibrant frosted accent `makeCleanTint(effAccentHex, false, 0.24, 0.72)`, Dark tone `rgba(vr, vg, vb, 0.34)`.
   - Clean, flat, borderless `.btn-secondary` / `.btn-ghost` tokens:
     - Smooth light surfaces: `rgba(15, 23, 42, 0.06)` -> hover `0.10` -> active `0.14`, `--adaptive-btn-ring: transparent`.
     - Smooth dark surfaces: `rgba(255, 255, 255, 0.10)` -> hover `0.16` -> active `0.22`, `--adaptive-btn-ring: transparent`.
     - Busy uploaded photos (`wpBusy`): crisp frosted glass `rgba(255, 255, 255, 0.64)` (light) / `rgba(24, 30, 44, 0.62)` (dark) + delicate subtle rim.
   - Clean neutral switch track (`rgba(15, 23, 42, 0.16)` light / `rgba(255, 255, 255, 0.22)` dark).
2. **`app/assets/main.css`**:
   - Fix `.settings-col .inline-form > input:not([type="color"])` so `<input type="color">` next to `Tải ảnh của bạn` is never stretched to `11rem`.
   - Remove unwanted forced grey inset box borders on `.btn-secondary` / `.btn-ghost` / `input.switch:not(:checked)` on smooth surfaces (`--adaptive-btn-ring` defaults to `transparent`, switch has `box-shadow: none`).
3. **`app/src/components/settings_more.rs`**:
   - Update default `wp_color` fallback from gloomy `#1F2A44` to fresh `#60A5FA`.
4. **`app/e2e/spec052-visual-and-contrast-audit.mjs`**:
   - Verify vivid non-white translucent accent active pills on BOTH `Cực quang` (100% transparency!) AND `Hoàng hôn` (72% transparency), ZERO grey boxes around idle segmented buttons on smooth presets, compact `<input type="color">` width (`<= 48px`) next to `Tải ảnh của bạn`, AND `0.00px` size shift when transitioning frameless -> framed on a genuinely busy uploaded photo!
