# Feature Specification — Spec 038: Chat Full-Bleed Bottom Canvas & Redesigned Larger Icons

## 1. Problem Statement
1. **Bottom Bar Clipping in Chat**: On desktop/web, `.workspace-shell` has `padding-bottom: var(--shell-gap)` (`11.25px`) and `.chat-column` has `overflow: hidden` + bottom border-radius. When messages, images, or rich cards scroll underneath the floating `.composer` bar, they are clipped horizontally `11.25px` above the bottom edge of the viewport, creating an unwanted horizontal strip ("cái thanh bên dưới") instead of letting messages and images touch the bottom edge.
2. **Small, Thin Navigation & UI Icons**: In the left `.app-rail` (and composer/header toolbars), navigation SVG icons render at `18px–20px` with thin `1.8px` strokes and wide internal margins, appearing small and weak next to `40×40px` app/plugin logos and user avatars.

## 2. Requirements & Acceptance Criteria
- **AC-1 (Zero Bottom Strip on Chat Canvas)**: `.chat-column` extends flush to the bottom edge of the viewport (`bottomGapToViewport <= 1px`, `border-bottom-left-radius: 0`, `border-bottom-right-radius: 0`), with no opaque/tinted horizontal bar under `.composer`.
- **AC-2 (Messages & Images Touch Bottom Edge)**: `.chat-messages` extends flush to the bottom of `.chat-column` (`bottomGapToViewport <= 1px`), allowing scrolling messages, images, and cards to pass seamlessly behind the transparent floating `.composer` pill all the way to the bottom edge of the screen.
- **AC-3 (Redesigned Larger, Prettier Icons)**:
  - All primary SVG icons in `app/src/components/icons.rs` are redesigned with fuller `2..22` optical geometry, smooth `2.0px` rounded strokes, and richer duotone fills (`0.22` opacity + refined specular/accent detail).
  - Left `.app-rail` navigation icons increase from `18–20px` to `26px` (top nav) and `24px` (bottom nav), and `.rail-brand` logo mark increases from `22px` to `28px`.
  - Composer and action icons increase proportionally (`22px` in composer, `18px`–`20px` in toolbars) while preserving explicit `size: 16` voice-status-panel SVGs and equal-height `.account-menu.rail-menu` buttons.
- **AC-4 (Zero Regressions)**: All existing Playwright E2E suites (`plugin-platform-e2e`, `plugin-system-check`, `borderless-shadowless-check`, `spec030-ui-check`, `scrollbar-switch-check`) and Rust unit tests pass 100%, with updated Linux Desktop & Web release builds installed.
