# Spec 124: Normalize Settings Scale, Navigation Proportions and Typography

## Problem Statement
In previous scale adjustments (Spec 120), Settings navigation was excessively reduced:
- `.settings-nav` was clamped to a narrow width (`16rem` / `256px`), with tablet `@media (max-width: 1023px)` collapsing it to `13rem` (208px), and server settings collapsing to `12.75rem` (204px).
- Item height was reduced to `2.125rem` (34px), horizontal padding to `10px`, gap to `8px`, and font size to `13px` (`var(--text-sm)`).
- This resulted in Vietnamese labels like "Dữ liệu & sao lưu" and "Riêng tư & bảo mật" crowding or clipping against the right border, search input looking compressed, and the entire Settings interface feeling noticeably shrunken and out of proportion with the rest of the application.

## Goals & Acceptance Criteria
1. **Spacious Settings Navigation Width**:
   - Introduce `--settings-nav-width: clamp(18rem, 21vw, 22rem);` in `app/assets/theme.css`.
   - Default `.settings-nav` and `.server-settings-stage .settings-nav` to `width: var(--settings-nav-width, 18.5rem); min-width: 17.5rem;`.
   - Remove aggressive `13rem` collapse at `@media (max-width: 1023px)`; ensure settings nav maintains at least `clamp(16rem, 20vw, 19rem)` on medium viewports.
2. **Comfortable Item Proportions & Typography**:
   - Update `.settings-nav-item` to `min-height: 2.5rem; height: 2.5rem;` (40px).
   - Padding: `0 14px;` (or `0 var(--space-3)`).
   - Gap: `12px;` between icon and label.
   - Font size: `var(--text-md)` (14.5px - 15px) with `font-weight: var(--weight-medium)`.
   - Border radius: `var(--radius-lg)` (10-12px).
   - Icon dimensions: `20px x 20px` in `.settings-nav-item svg`.
3. **Harmonious Search & Settings Body**:
   - `.settings-search input`: `min-height: 2.375rem; font-size: var(--text-md); padding-left: 2.35rem; border-radius: var(--radius-lg);`.
   - `.settings-body`: maintain generous padding `clamp(1.25rem, 2.5vw, 2rem)`.
   - `.settings-row`: `min-height: 3rem; padding: var(--space-2) 0;`.
4. **Responsive Mobile Safeguard**:
   - Retain full mobile responsive behavior at `<= 767px` where settings navigates between category list and detail view.
5. **No Regressions**:
   - All tests pass (E2E Playwright tests, zero console errors, smooth resizing).
