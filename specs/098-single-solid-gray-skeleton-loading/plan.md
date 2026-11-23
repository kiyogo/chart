# Spec 098 Technical Implementation Plan: Single Solid Gray Skeleton Loading

## 1. Architecture
- Remove all `linear-gradient` references from skeleton loading selectors in `app/assets/main.css`.
- Introduce `--bg-skeleton` in `app/assets/theme.css`:
  - Light theme: `#E5E7EB`
  - Dark theme: `#2D3748`
- Configure `.skeleton-shimmer, .skeleton-box, .skeleton-circle, .skeleton-text, .skeleton-avatar`:
  - `background: var(--bg-skeleton, #E5E7EB) !important;`
  - `background-image: none !important;`
  - `animation: skeleton-pulse 1.8s ease-in-out infinite;`
- Update `@keyframes skeleton-pulse` and `@keyframes skeleton-shimmer` to pulse only `opacity: 1` -> `opacity: 0.55` -> `opacity: 1`.
- Ensure all skeleton avatars, badges, lines, message bubbles, story circles, and detail cards display this exact solid gray.

## 2. File Changes
1. `app/assets/theme.css`:
   - Add `--bg-skeleton` across light and dark theme sections.
2. `app/assets/main.css`:
   - Update lines 11925-11968 to enforce solid gray and remove all gradient shimmer definitions.
3. `app/e2e/spec098-solid-gray-skeleton.mjs`:
   - Create automated verification script to test:
     - Computed style `background-color` is solid gray.
     - Computed style `background-image` is `none`.
     - Capture screenshot into `verify_098/`.

## 3. Deployment
- Run `./app/build-web.sh release`.
- Run automated Playwright E2E verification.
- Auto-push to `origin main` following rule 1.
