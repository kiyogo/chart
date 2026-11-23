# Spec 097 Technical Implementation Plan: Avatar Skeleton Shimmer & Stable Chat Frame

## 1. Architecture Overview
- **Issue 1**: When an avatar has an image URL (`url_clean.is_some()`), while the image is resolving asynchronously from IndexedDB/Web Crypto/server, `Avatar` rendered a solid colored circle with the initial letter/digit (e.g. "1" or "N"). Once resolved, it popped into the real photo.
  - **Solution**: In `app/src/components/avatar.rs`, when `url_clean.is_some() && blob_src().is_none() && !img_failed()`, render a circular shimmer skeleton (`div.user-avatar.skeleton-circle.skeleton-shimmer`) with exact matching dimensions (`{size}px`) and `border-radius: 50%`.
  - Only show initial letter/number if `url_clean.is_none()` (no avatar set) or `img_failed()` (image load failed).
- **Issue 2**: When clicking any conversation in the chat list, the chat container momentarily disappeared / collapsed to empty wallpaper for ~1 second before reappearing.
  - **Solution**: In `app/src/pages/chat.rs`, remove `if dm_loading_initial() { SkeletonDetails {} } else` from line 4642.
  - Keep `DmDetailsPanel` and `SelfDetailsPanel` mounted immediately. They already hold the active friend/self metadata.
  - Inside `MessageList`, `SkeletonMessages` already provides smooth shimmer loading in the central stream without collapsing the layout.
  - Update `SkeletonDetails` in `app/src/components/skeleton.rs` to wrap itself in `aside.details-panel` so any future rendering adheres to the responsive grid / absolute drawer rules.

## 2. File Changes
1. `app/src/components/avatar.rs`:
   - Add avatar shimmer skeleton rendering during active image resolution.
2. `app/src/pages/chat.rs`:
   - Remove `SkeletonDetails` conditional from `.chat-main-container`.
   - Remove unused import of `SkeletonDetails`.
3. `app/src/components/skeleton.rs`:
   - Update `SkeletonDetails` root element to `aside.details-panel.skeleton-details-wrap`.
4. `app/e2e/spec097-avatar-shimmer-and-stable-frame.mjs`:
   - Comprehensive Playwright test verifying:
     1. Avatar loading renders `div.user-avatar.skeleton-circle.skeleton-shimmer` before resolving to `img.user-avatar`.
     2. Switching conversation maintains `.chat-column` and `.details-panel` visible at 1024px and 1440px viewport widths with 0ms blank wallpaper collapse.
     3. Captures screenshots into `verify_097/`.

## 3. Verification & Safety
- Build with `./app/build-web.sh release`.
- Run E2E test with Playwright.
- Commit and push to `origin main` with link to commit.
