# Spec 128: Butter-Smooth Kinetic Dragging for Stories & Notes Strip

## 1. Problem Statement
The user reported:
> *"lỗi nó giật giật kéo nó không mượt gì luôn á"* (Bug: it's jittery/choppy, dragging it is not smooth at all!)
Accompanied by screenshots of grabbing and dragging the Stories & Notes horizontal carousel (`conv-notes-strip`).

### Root Causes of Jitter & Stutter:
1. **CSS Scroll Snap Conflict**: `.notes-strip` had `scroll-snap-type: x proximity;` and `.note-item` had `scroll-snap-align: start;`. Every time JavaScript updated `el.scrollLeft` during a mouse drag, the browser's CSS Scroll Snap engine fought the pointer movement by continuously attempting to pull the scroll position towards snap anchors, producing violent frame-by-frame stuttering and resistance.
2. **Synchronous Layout Thrashing**: Pointer move events were updating `el.scrollLeft` synchronously on every event. High-polling mice (500Hz-1000Hz) triggered hundreds of synchronous layout reflows per second, starving the render pipeline and dropping frame rates.
3. **Deadzone Initial Jump**: Dragging only started after `Math.abs(dx) > 5`, but upon crossing 5px, `el.scrollLeft` jumped abruptly by 6px in a single frame without re-anchoring `startX`.
4. **Missing Pointer Capture**: Without `el.setPointerCapture(e.pointerId)`, dragging over child avatar buttons or outside the strip could miss pointer events or trigger browser drag defaults.
5. **Missing Kinetic Momentum**: Releasing the mouse instantly stopped scrolling dead at the exact pixel with zero inertia, feeling unnatural and abrupt.
6. **Choppy Mouse Wheel**: Discrete addition of `deltaY` without dampening caused jarring jumps on mouse wheels.

## 2. Requirements & Acceptance Criteria
1. **Butter-Smooth 60/120fps Dragging**:
   - Batch scroll mutations using `requestAnimationFrame`.
   - Re-anchor `startX` upon passing the 4px threshold so there is zero initial jerk/jump.
   - Use `el.setPointerCapture(e.pointerId)` on `pointerdown` and `releasePointerCapture` on `pointerup/cancel`.
2. **Removal of Scroll-Snap Resistance**:
   - Eliminate `scroll-snap-type` and `scroll-snap-align` from `.notes-strip` and `.note-item` to remove layout snap fighting.
   - Add `will-change: scroll-position;` and `overscroll-behavior-x: contain;`.
3. **Natural Kinetic Momentum (Inertia)**:
   - Sample pointer velocity in a sliding window (last ~80ms).
   - On release, apply natural friction deceleration (`v *= 0.94`) via `requestAnimationFrame` until stopping smoothly.
   - Stop momentum if the user taps/clicks again or hits container boundaries.
4. **Smooth Wheel Support**:
   - Smooth mouse wheel scrolling with 0.75 dampening.
5. **Click Suppression on Drag**:
   - Prevent accidental opening of note/story modals when releasing a drag by intercepting clicks within 120ms of dragging.
6. **Unified Experience Across Rails**:
   - Apply the butter-smooth controller to both `StoriesNotesBar` (`conv-notes-strip`) and `feed_view.rs` (`feed-story-rail`).
7. **Verification & Auto-Push**:
   - E2E Playwright test simulating drag gestures, verifying scroll response, measuring smooth frame progression, and capturing screenshots.
   - Auto-commit and git push to `origin main` (Rule 1).
