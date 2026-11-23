# Spec 097: Avatar Skeleton Shimmer Loading and Stable Chat Frame

## 1. Problem Statement & User Feedback
The user reported two specific bugs:
1. `"lỗi ở avatar nó không hiện đúng load"`:
   - When an avatar has an image URL (such as the user's avatar on the app rail or friend avatars in stories/lists), while the image is resolving from Web Crypto/IndexedDB/Network, it briefly displayed a solid colored circle with raw fallback letters (e.g. blue circle with "1" as seen in Image 1).
   - Once resolved, it popped into the real photo (Image 2).
   - Expected: When an avatar has an image URL that is currently loading, it should display a smooth, circular skeleton shimmer placeholder (`skeleton-circle skeleton-shimmer`) instead of flashing fallback letter initials.
2. `"khi bấm vô 1 tin nhắn mới là nó bị lỗi mất khung luôn một 1s gì đó"`:
   - When clicking on a conversation (e.g., `1231232`), for approximately 1 second while messages were fetching from the server, the entire central chat container disappeared completely (Image 3: entire main area collapsed to empty wallpaper).
   - After 1 second, the chat frame and right details panel suddenly appeared (Image 4).
   - Root Cause: In Spec 096, `if dm_loading_initial() { SkeletonDetails {} }` was placed directly as a child of `.chat-main-container` without being enclosed in `aside.details-panel`. On responsive widths (< 1100px, such as the user's 1024px viewport), `.details-panel` is `position: absolute`, whereas `.skeleton-details-wrap` with `width: 100%` in normal flex flow caused `.chat-column` to collapse to 0 width. Furthermore, `DmDetailsPanel` already has the recipient's known metadata and should remain mounted and stable.

## 2. Proposed Architecture & Solution
1. **Avatar Loading Shimmer (`app/src/components/avatar.rs`)**:
   - In `Avatar`, check if `url_clean.is_some() && blob_src().is_none() && !img_failed()`.
   - If loading, render `div.user-avatar.skeleton-circle.skeleton-shimmer` with matching dimensions (`{size}px`) and `border-radius: 50%`.
   - Only fall back to letter initials if `url_clean.is_none()` (no avatar configured) or `img_failed()` (image error 404).
2. **Stable Chat Frame During Conversation Loading (`app/src/pages/chat.rs`)**:
   - Remove standalone `if dm_loading_initial() { SkeletonDetails {} }` from line 4642.
   - Keep `DmDetailsPanel` and `SelfDetailsPanel` mounted with their proper `aside.details-panel` wrapper.
   - When switching conversations:
     - `.chat-column` stays 100% visible with header and composer.
     - `MessageList` displays `SkeletonMessages` while `dm_loading_initial()` is true and messages are empty.
     - `.details-panel` remains rock-solid in place.
   - Zero frame loss ("mất khung"), zero layout collapse.

## 3. Acceptance Criteria
- [x] Avatar with image URL shows circular shimmer skeleton while resolving; never flashes raw letter initials like "1".
- [x] Clicking any conversation in the list keeps the chat frame, header, message stream area (with skeleton bubbles), composer, and details panel immediately visible.
- [x] The 1-second blank wallpaper glitch (Image 3) is completely eliminated.
- [x] WebAssembly release build compiles cleanly.
- [x] Automated Playwright E2E tests verify avatar shimmer and stable chat transition without frame drops.
