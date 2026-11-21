# Technical Plan — Spec 045: Adaptive Contrast & Foolproof Aesthetic Compensation Engine

## Architecture Overview
Inspired by open-source adaptive glass & perceptual contrast architectures (`open-glass-ui`, `contrast-color`, `Liquid-Glass-Android`, Apple Vibrancy / WCAG 2.1):

### 1. Unified Real-Time Contrast Engine in `app/src/platform/mod.rs`
- Register a global, idempotent JS engine `window.__chatwRecomputeContrast()` invoked by BOTH `apply_appearance()` and `apply_customization()` (as well as `prefers-color-scheme` change listener and wallpaper image pixel sampler callback).
- Store customization parameters in `window.__chatwCustom`:
  `{ wpKind, wpColor, wpImageUrl, dim, panelOpacity, panelGlass, customAccent, bubbleSelf, bubbleOther }`.
- **Pixel Sampler**:
  - For `wpKind === 'image'`, cache 32×32 canvas analysis in `window.__chatwWpMetrics[url]`:
    - `meanB`: overall perceived brightness (`0.299*r + 0.587*g + 0.114*b` / 255)
    - `railB`: leftmost 12.5% strip brightness (under `.app-rail`)
    - `stdB`: brightness standard deviation (detects high-contrast busy art/photos when `stdB > 0.16`)
  - Trigger instant recompute on image load.
- **Effective Composite Brightness**:
  - `effectiveDim = (wpKind === 'none') ? 0 : dim`
  - `backdropB = meanB * (1 - effectiveDim)`
  - `railBackdropB = railB * (1 - effectiveDim)`
  - `panelEffectiveB = (1 - alpha) * backdropB + alpha * panelBaseB`
- **Adaptive Tone & Scrim Attributes on `<html>`**:
  - `data-surface-tone`: `"dark"` if `panelEffectiveB < 0.48` else `"light"` (when `wpKind === "none"` and `alpha === 1.0`, matches `data-theme` directly).
  - `data-rail-tone`: `"dark"` if `railBackdropB < 0.50` else `"light"`.
  - `data-wp-busy`: `"true"` if `wpKind === "image" && stdB > 0.16` else `"false"`.
  - `data-panel-scrim`: `"true"` if `wpKind !== "none" && (alpha < 0.72 || (wpKind === "image" && stdB > 0.16 && alpha < 0.88))` else `"false"`.
  - `data-rail-scrim`: `"true"` if `wpKind === "image" && stdB > 0.16 && effectiveDim < 0.55` else `"false"`.
- **WCAG 2.1 Color Compensator (HSL Lightness Search)**:
  - `compensateAccent(hex, surfaceTone)`:
    - Computes surface luminance (`L_surface = 1.0` for light, `0.013` for dark).
    - Adjusts HSL lightness so `--accent` has contrast $\ge 3.5:1$ against `surfaceBg`, `--accent-hover` shifts subtly for hover feedback, `--on-accent` picks `#0F172A` or `#FFFFFF` for maximum contrast ($\ge 5.0:1$), `--accent-text` achieves $\ge 5.2:1$ against `surfaceBg`, and `--accent-soft` / `--accent-soft-strong` harmonize with `surfaceTone`.
  - `compensateBubble(hex, defaultBg, chatBackdropHex)`:
    - Gently nudges bubble lightness if `contrastRatio(bubble, chatBackdrop) < 1.16` so bubbles never vanish invisibly into the background.
    - Sets `--msg-sent-text`, `--msg-sent-meta`, `--msg-recv-text`, `--msg-recv-meta` for strict WCAG AA/AAA legibility on EVERY chosen color.

### 2. Adaptive Surface CSS Tokens & Scrim Rules in `app/assets/main.css`
- **Adaptive Dark Surface Override** (`html[data-surface-tone="dark"]` when `data-theme="light"`):
  - Cleanly overrides `--text`, `--text-primary` (`#F4F6FA`), `--text-secondary` (`#CBD3E1`), `--text-muted` (`#97A3B8`), `--bg-hover`, `--bg-active`, `--bg-input`, `--bg-surface-hover`, `--bg-elevated`, `--bg-selected`, `--accent-soft`, `--accent-soft-strong`, `--accent-text`, `--text-selected`, `--text-link`, `--msg-recv`, `--msg-recv-text`, `--msg-recv-meta` so the ENTIRE workspace (settings, sidebar, chat, buttons, tiles, inputs, popovers) harmoniously switches to luminous white-on-dark glass — zero blinding white buttons, zero black-on-dark text!
- **Adaptive Light Surface Override** (`html[data-surface-tone="light"]` when `data-theme="dark"`):
  - Cleanly overrides foreground text and translucent card surfaces to crisp charcoal `#111827` / `#3D4657` on bright translucent snow-white cards so Dark Theme + bright transparent wallpaper never shows white-on-white!
- **Adaptive Rail Tone (`html[data-rail-tone="dark"]` / `light` & `data-rail-scrim="true"`)**:
  - `.app-rail .rail-item` icon color adapts independently to the wallpaper behind the rail!
  - On busy photo wallpapers (`data-rail-scrim="true"`), `.app-rail` gets a sleek subtle translucent vertical capsule matching `railTone` so every rail icon stays crystal clear.
- **Protective Card & Floating Text Scrims (`html[data-panel-scrim="true"]`, `html[data-wp-busy="true"]`)**:
  - Raises inner card/row/tile/button surface floor (`78%` on calm wallpapers, `88%` on busy high-contrast photos) using clean luminous white (`rgba(255,255,255,...)`) in light tone and sleek deep slate (`rgba(16,20,30,...)`) in dark tone — WITHOUT adding `backdrop-filter: blur` when `data-panel-glass="false"` and WITHOUT adding ugly borders or shadows!
  - Gives `.settings-nav-item`, `.settings-nav-title`, `.settings-group-title`, `.settings-page-title`, `.chat-item`, and `.chat-header` info/actions crisp adaptive translucent pill plates when `panel_opacity` is very low (`data-panel-scrim="true"`) over a wallpaper.
- **Inline Group Heading Icon & Chat Wallpaper `wp-night` Adaptive Tokens**:
  - `.settings-group-title { display: flex; align-items: center; gap: 8px; }`.
  - `.chat-column.wp-night` and `.settings-preview-chat.wp-night` receive crisp dark-surface ink & composer tokens so `"Đêm sao"` chat wallpaper always shines in Light Mode.

### 3. Live Preview Sync in `app/src/components/settings_view.rs`
- Apply `wp-{prefs.wallpaper}` class onto `.settings-preview-chat` so the user sees their chosen chat wallpaper, bubble shape, bubble colors, and auto-compensated text colors live inside Appearance Settings!

### 4. Automated Verification (`app/e2e/spec045-adaptive-contrast.mjs`)
- Playwright E2E test verifying WCAG computed contrast ratios (`>= 4.5:1`) across EVERY nasty stress-test scenario from the user's screenshots:
  1. Light theme + `"Đêm sao"` + 24% transparency (Image 1): verifies `data-rail-tone="dark"` (crisp white rail icons) & high-contrast light cards.
  2. Light theme + `"Chấm bi"` + 100% transparency + 80% dim (Image 4): verifies `data-surface-tone="dark"`, white text on dark background (`contrast >= 7:1`), no blinding white buttons.
  3. Light theme + High-contrast busy anime-like half-white/half-navy uploaded `.webp` photo + 100% transparency + 0% dim + glass blur OFF (Image 6): verifies `data-wp-busy="true"`, `data-panel-scrim="true"`, `backdrop-filter === "none"` (no forced blur!), and crisp legible cards/nav/titles (`contrast >= 7:1`).
  4. Light theme + Busy uploaded photo + 100% transparency + 35% dim (Image 7): verifies automatic flip to `data-surface-tone="dark"` with luminous crisp white typography.
  5. Dark theme + Pure white custom wallpaper (`#FFFFFF`) + 100% transparency + 0% dim: verifies automatic flip to `data-surface-tone="light"` so white text never vanishes on white!
  6. Extreme custom colors: `custom_accent = #FFFF00` (neon yellow) & `#FFFFFF` (white) in Light Mode, `#000000` (black) in Dark Mode; `bubble_self = #FFFFFF` & `bubble_other = #000000`: verifies WCAG contrast >= 4.5:1 on buttons, sent/received bubbles, AND timestamps (`bubble-meta`)!
  7. Chat wallpaper `"Đêm sao"` in Light Mode: verifies `.settings-preview-chat.wp-night` & `.chat-column.wp-night` header/bubble readability.
  8. Full regression sweep (`spec028`, `spec040`, `spec041`, `spec042`, `spec043`, `spec044`).
