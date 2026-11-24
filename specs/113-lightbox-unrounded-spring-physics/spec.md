# Spec 113: Strictly Unrounded Lightbox Images & True-Physics Damped Harmonic Spring Motion

## 1. Problem Statement
1. **Unrounded Image Corners**: Lightbox images must never show rounded corners (`border-radius: 0px`) in any state (idle, dragging, zoomed, rotated, or during AI enhancement).
2. **Robotic CSS Snap-Back vs True Physical Spring Motion**:
   - Currently, releasing a dragged Lightbox image uses a static fixed-duration CSS `transition: transform 280ms cubic-bezier(...)` that ignores pointer release velocity (causing an abrupt dead stop at `mouseup` before reversing) and updates `pan_x`/`pan_y` via full Dioxus VDOM re-renders on every `mousemove` pixel.
   - Users want authentic, ultra-smooth physical spring return ("hiệu ứng chạy về vị trí cũ mượt hơn nữa đúng vật lý") inspired by Apple iOS `UIScrollView` / `CASpringAnimation` and PhotoSwipe v5.

## 2. Goals & Acceptance Criteria
1. **Strict `0px` Corners Across All Lightbox Layers**:
   - `.lightbox-img-frame`, `.lightbox-img`, `.ai-gen-overlay.lightbox-ai-gen`, `.lightbox-ai-gen .ai-gen-blur-img`, and `.lightbox-ai-gen .ai-gen-frost` all strictly enforce `border-radius: 0 !important;`.
2. **True Pure-Rust Mass-Spring-Damper Physics Engine (`60/120/144/240Hz`)**:
   - Real pointer velocity tracker (`PointerVelocityTracker`) computing instantaneous release velocity `(vx, vy)` in `px/s` over the recent pointer motion window (naturally decaying to `0` if held stationary before release).
   - Pure-Rust sub-stepped Hooke's-law damped harmonic oscillator (`LightboxSpringEngine`):
     - $a = -k (x - x_{\text{target}}) - c \cdot v$ with $m = 1.0$, pan stiffness $k = 340.0$, damping ratio $\zeta = 0.88$ ($c = 2\zeta\sqrt{k}$), zoom/rotate stiffness $k_z = 420.0$, $\zeta_z = 0.94$.
     - When zoomed (`zoom > 1.0`) and flicked inside clamped pan bounds, applies natural exponential friction deceleration ($v' = v e^{-\lambda \Delta t}$) seamlessly transitioning into the boundary spring if momentum crosses an edge.
     - Releasing an off-center/overscrolled drag injects real hand velocity `(vx, vy)` directly into the spring so flicks carry authentic inertia before arcing smoothly back and cushioning to rest.
     - Grabbing mid-flight (`onmousedown` while animating) cancels the spring immediately at the exact live `(x, y)` coordinates with 0px teleport.
   - Driven via `web_sys::window().request_animation_frame(...)` writing `transform` directly to `.lightbox-img` (and `.lightbox-ai-gen` when mounted) with **zero Dioxus VDOM re-renders during drag or spring flight**, syncing exact settled coordinates back cleanly when at rest (`0%` idle CPU).
3. **100% Compatibility & Verification**:
   - Pure-Rust unit tests verifying velocity tracking, flick momentum carry, spring convergence within `< 420ms`, critically damped zoom/rotation settling, and mid-flight grab continuity.
   - Playwright E2E verifying `border-radius: 0px`, smooth multi-frame physical trajectory during spring-back, flick momentum overshoot + smooth return to exact `(0, 0)`, mid-flight grab, and 100% pass across Spec 101, Spec 112, Spec 111, Spec 110, and Clean AI UI tests.
