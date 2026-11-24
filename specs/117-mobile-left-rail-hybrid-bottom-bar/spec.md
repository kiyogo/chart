# Spec 117: Mobile Hybrid Left Rail + Core Bottom Bar Architecture

## Problem Statement
In Spec 116, all 8+ navigation features (`AppRail`) were placed horizontally into a single bottom bar (`flex-direction: column-reverse`), which contradicted the desired mobile ergonomics:
- Feature/workspace navigation (`Cộng đồng`, `Họp nhóm`, `Mới`, `Vico AI`, `Trình cắm`, pinned servers/groups, plugin apps, settings) belongs in a **compact vertical rail on the LEFT side** (just like Discord mobile / desktop ChatW).
- Only **one compact part** (the core everyday tabs: `Tin nhắn`, `Bạn bè`, `Tìm kiếm`, `Thông báo`, `Tôi`) should live in the **bottom navigation bar**.

## Goals & Acceptance Criteria
- **AC-1 (Left Vertical Rail on Mobile `<= 767px`)**:
  - `.workspace-shell` keeps `flex-direction: row` on mobile.
  - `.app-rail` stays on the **LEFT** (`width: 56px`, vertical column, positioned cleanly above the bottom bar with safe-area top padding) containing feature/workspace items (`Cộng đồng`, `Họp nhóm`, `Mới`, `Vico AI`, `Trình cắm`, `left_apps`, `custom_pages`, `rail-pins`, `Settings`), and hides (`hidden-mobile`) when inside an active chat.
- **AC-2 (Core Bottom Bar — Only 1 Part at Bottom)**:
  - Dedicated `.mobile-bottom-bar` visible ONLY on mobile (`<= 767px`) when not inside an active chat (`!hidden_mobile`), fixed at bottom with `env(safe-area-inset-bottom)`.
  - Contains 5 evenly distributed core tabs (`Tin nhắn`, `Bạn bè`, `Tìm kiếm`, `Thông báo`, `Tôi` avatar) with crisp icon + short label, each `>= 68px` wide and `>= 48px` tall.
  - Core items already in `.mobile-bottom-bar` (`Tin nhắn`, `Bạn bè`, `Tìm kiếm`, `Thông báo`, `rail-account`) are tagged with `rail-mobile-in-bottom` and hidden from the left `.app-rail` on `<= 767px` so there is ZERO duplication between left rail and bottom bar!
- **AC-3 (Full-Screen Active Chat & Zero Overflow)**:
  - Inside an active chat (`hidden_mobile == true`), BOTH `.app-rail` and `.mobile-bottom-bar` hide, giving the chat full `100vw × 100dvh`.
  - Zero horizontal document overflow (`scrollWidth <= innerWidth`) across every view on `390×844` and `360×800`.
- **AC-4 (Zero Desktop Regressions)**:
  - On desktop (`>= 768px`), `.mobile-bottom-bar` is `display: none` and `.app-rail` renders 100% identically to before.
