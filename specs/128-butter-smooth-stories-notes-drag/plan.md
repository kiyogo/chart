# Plan 128: Butter-Smooth Kinetic Dragging for Stories & Notes Strip

## 1. CSS Refinement (`app/assets/main.css`)
- Remove `scroll-snap-type: x proximity;` from `.notes-strip`. Set `scroll-snap-type: none !important;`.
- Remove `scroll-snap-align: start;` from `.note-item`.
- Set `overscroll-behavior-x: contain;`, `will-change: scroll-position;`, `touch-action: pan-y;`.
- In `.notes-strip.dragging`, ensure `scroll-behavior: auto !important;`, `cursor: grabbing !important;`.
- In `.feed-story-rail`, ensure `scroll-snap-type: none;`, `scroll-behavior: auto;` during drag.

## 2. Controller Refinement (`app/src/components/navigation/conversations.rs`)
- Implement:
  - `setPointerCapture` and `releasePointerCapture`.
  - `requestAnimationFrame` scheduled scroll updates (`scheduleScroll`).
  - Smooth deadzone re-anchoring (`startX = currentX; startScroll = el.scrollLeft`).
  - Velocity tracking with sliding window timestamp samples.
  - Exponential decay kinetic momentum (`v *= 0.94`) on `pointerup`.
  - Smooth wheel handler (`delta * 0.75`).
  - Robust click suppression (`el.dataset.justDragged = '1'`).

## 3. Feed Story Rail Refinement (`app/src/components/feed_view.rs`)
- Upgrade `feed-story-rail` with the same rAF kinetic drag controller and pointer capture.

## 4. Verification
- Playwright E2E script `app/e2e/spec128-butter-smooth-stories-drag.mjs`:
  - Locates `#conv-notes-strip`.
  - Simulates mouse pointerdown, dragging across multiple positions, and pointerup.
  - Verifies that `scrollLeft` increases smoothly without jumping back or stalling.
  - Verifies kinetic momentum glide after release.
  - Verifies that clicking without dragging still opens modals as intended.
  - Captures screenshot artifacts.
