# Spec 118 Technical Plan

1. **`app/assets/main.css`**:
   - Remove all `border-right` / `border-top` / `border-bottom` lines from `.app-rail`, `.rail-bottom`, `.rail-pins`, `.mobile-bottom-bar`, and `.settings-mobile-bar` on mobile (`<= 767px`).
   - Add frosted-glass / translucent wallpaper support for `.app-rail` and `.mobile-bottom-bar` under `html[data-app-wp]:not([data-app-wp="none"])`, `html[data-panel-glass="true"]`, and `html[data-panel-translucent="true"]` using `color-mix(in srgb, var(--bg-surface) var(--panel-opacity, 72%), transparent)` + `backdrop-filter: blur(var(--panel-blur, 18px)) saturate(1.35)`.
   - Enforce uniform icon dimensions on mobile:
     - `.app-rail .rail-item:not(.has-logo) svg`: `width: 24px !important; height: 24px !important;` (applies uniformly to both `.rail-nav` and `.rail-bottom` Settings gear!).
     - `.app-rail .rail-brand svg, .app-rail .rail-item.has-logo svg`: `width: 28px !important; height: 28px !important; border-radius: 8px;`.
     - `.mobile-tab-icon svg, .mobile-tab-icon .user-avatar`: `width: 22px !important; height: 22px !important; min-width: 22px !important; min-height: 22px !important; font-size: 0.6875rem !important;`.
2. **`app/e2e/spec116-mobile-ui-optimization.mjs`**:
   - Add strict assertions verifying:
     - `0px` borders on `.app-rail`, `.rail-bottom`, `.mobile-bottom-bar`.
     - Exact uniform icon dimensions (`24×24px` left rail outline icons, `28×28px` logo icons, `22×22px` all 5 bottom tab icons including avatar).
     - Light + Frosted Wallpaper screenshot (`11_light_glass_wallpaper_390x844.png`) confirming seamless frosted white glass on `.app-rail` + `.nav-sidebar` + `.mobile-bottom-bar`!
