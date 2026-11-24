# Spec 113 Technical Plan

## 1. Pure-Rust Spring Physics Module (`app/src/components/rich.rs`)
- Add `LightboxSpringEngine` + `VelocitySample` ring buffer (pure Rust, zero JS physics math):
  - Fields:
    - Live state: `x, y, vx, vy, zoom, vz, rot, vrot`
    - Targets: `tx, ty, tzoom, trot`
    - Bounds: `base_w, base_h, vw, vh`
    - Drag state: `dragging: bool`, `drag_start_x, drag_start_y, drag_origin_x, drag_origin_y`, `samples: Vec<(f64, f64, f64)>` (timestamp ms, client_x, client_y)
  - Methods:
    - `begin_drag(cx, cy, now_ms)`: cancels any active spring flight right at current live `(x, y)`, clears velocity `(0, 0)`, records drag origin & initial sample.
    - `move_drag(cx, cy, now_ms) -> bool`: updates live `(x, y)` directly, appends velocity sample (pruning > 95ms old), returns `did_move_past_threshold`.
    - `end_drag(now_ms)`: computes release velocity `(vx, vy)` in `px/s` from velocity buffer (zeroes out if last motion was > 65ms ago), clamps velocity to `[-4200, 4200]`, updates spring target `(tx, ty)` using `clamp_lightbox_pan`.
    - `set_targets(tx, ty, tzoom, trot)`: smoothly springs zoom/rotate/reset/pan to new targets.
    - `step(dt_sec) -> bool`: sub-steps (`h <= 0.004s`) physical motion:
      - Inside bounds when zoomed & moving freely: smooth exponential inertia glide (`v *= (-5.2 * h).exp()`) updating target dynamically until reaching edge or stopping.
      - Past bounds / returning to target: Hooke's law damped harmonic oscillator (`k = 340.0, zeta = 0.88`) on `(x, y)` and near-critical spring (`k = 420.0, zeta = 0.94`) on `(zoom, rot)`.
      - Returns `true` while still moving; when settled (`|dx| < 0.35 && |dy| < 0.35 && |vx| < 8.0 && |vy| < 8.0 && |dz| < 0.0008 && |vz| < 0.01 && |dr| < 0.06 && |vr| < 2.0`), snaps cleanly to exact target values and returns `false`.
    - `format_transform() -> String`: outputs exact `translate3d({x}px, {y}px, 0) scale({z}) rotate({r}deg)` formatting settled integer values cleanly (`0px`, `90deg`, `1`, `2`) so Spec 078 & Spec 101 string assertions remain 100% compatible.
- Direct-DOM `requestAnimationFrame` driver (`#[cfg(target_arch = "wasm32")]`):
  - Applies `format_transform()` directly to `.lightbox-img` and `.lightbox-ai-gen` inside `move_drag` and each `rAF` tick — **0 Dioxus re-renders during drag or spring animation!**
  - Cancels cleanly when Lightbox closes or switches images (`generation` counter).
  - Respects `prefers-reduced-motion` / `html[data-reduce-motion="true"]` (instant snap when reduced motion is enabled).

## 2. Strict `0px` Unrounded Lightbox CSS (`app/assets/main.css`)
- Enforce `border-radius: 0 !important;` on `.lightbox-img-frame`, `.lightbox-img`, `.ai-gen-overlay.lightbox-ai-gen`, `.lightbox-ai-gen .ai-gen-blur-img`, `.lightbox-ai-gen .ai-gen-frost`.
- Remove CSS `transition: transform 280ms ...` from `.lightbox-img` and `.ai-gen-overlay.lightbox-ai-gen` so CSS transitions NEVER fight the 120Hz Pure-Rust physics engine! (Keep `transition: opacity 360ms cubic-bezier(0.22, 1, 0.36, 1)` on `.ai-gen-overlay` for the smooth 4K reveal.)

## 3. Verification
- Rust unit tests (`cargo test spec080_lightbox_clamp_tests`):
  - Velocity tracking on flick vs stationary pause.
  - Physical inertia carry on flick release + smooth spring return to `(0, 0)`.
  - Convergence under `420ms` from `220px` and `400px` displacements.
  - Mid-flight grab retains exact live coordinates.
- New Playwright E2E (`app/e2e/spec113-lightbox-unrounded-spring-physics.mjs`):
  - Verifies `border-radius: 0px` across all Lightbox layers.
  - Verifies continuous multi-frame physical trajectory on slow drag release AND fast flick momentum overshoot + smooth damped return to exact `(0, 0)`.
  - Verifies mid-flight grab catches cleanly at intermediate coordinates without snapping/teleporting.
  - Re-runs Spec 101 & Spec 112 to verify 100% green regression!
