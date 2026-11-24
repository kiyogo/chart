# Plan 119: Mobile Unified Frosted Glass & Seamless Bottom Bar Layout

## 1. Architectural Strategy
1. **Vertical Seam Elimination**:
   - Align both `.app-rail` and `.nav-sidebar` heights to `calc(100dvh - 58px - env(safe-area-inset-bottom))` in `@media (max-width: 767px)`.
   - Set `.mobile-bottom-bar` to an elevated frosted glass surface (`var(--bg-surface)` at 88% opacity + `blur(20px) saturate(1.4)`) spanning full `100vw`.
   - Ensure neither the rail nor the sidebar bleeds down into the bottom bar's coordinate space.
2. **Unified Frosted Glass ("mờ trắng")**:
   - Disable `.app-rail::before` on mobile (`display: none !important;`).
   - Standardize `.app-rail` and `.nav-sidebar` backgrounds to use `var(--bg-surface)` with `var(--panel-opacity, 72%)` and identical blur parameters under wallpaper/glass modes.
   - Eliminate all borders between panels on mobile.
3. **Harmonious Icon Sizing**:
   - In `app/assets/main.css`:
     - Force `.rail-brand svg` and `.app-rail .rail-item.has-logo svg` to `24×24px`.
     - Force `.app-rail .rail-item:not(.has-logo) svg` to `24×24px`.
     - Force `.mobile-tab-icon svg`, `.mobile-tab-icon .user-avatar` to `22×22px`.
   - In `app/src/components/navigation/rail.rs`:
     - In Tab 5 of `.mobile-bottom-bar`: Check if `mob_avatar` has an actual image URL. If not, render `UserIcon { size: 22 }` instead of `Avatar` with a bold purple background.
4. **E2E Automated Verification**:
   - Extend `app/e2e/spec116-mobile-ui-optimization.mjs` with Spec 119 test suite.
   - Verify:
     - Rail height equals sidebar height (`=== 100dvh - 58px`).
     - Bottom bar is 100vw and has uniform color sampling across `x = [0, 54, 78, 200, 390]`.
     - Zero pixel variance across the bottom boundary seam.
     - Tab 5 icon size is 22×22px and matches other tabs.
     - Left rail icons all measure 24×24px.
