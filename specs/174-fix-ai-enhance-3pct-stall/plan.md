# Technical Plan: Spec 174 — Eliminate 3% AI Image Enhancement Stalls

## Affected Files
1. `app/src/storage/realesrgan_gpu.rs`
   - Add atomic `ACTIVE_GPU_JOB_GEN: AtomicU64` + `CANCELLED_BEFORE_GEN: AtomicU64` (or per-job token checking `start_ms`, `budget_ms`, and latest generation/abort helper `abort_gpu_jobs`) so timed-out/superseded Rust async futures exit immediately on the next `await` point without touching the next job's `GL_ENGINE` textures.
   - Tighten `single_pass_ok` threshold to `w * h <= 520_000` (and `ow <= 2560 && oh <= 2560`) so large images tile safely within VRAM & TDR limits on integrated/ANGLE GPUs.
   - Bound `await_gl_fence_nonblocking` with both remaining GPU budget (`deadline_ms`) and per-fence cap (e.g., max ~1200ms per fence / total `max_gpu_budget_ms = 5500ms`), checked inside `run_tile_gl_async` and `run_span_tile_gl_async` before/after every block batch!
   - Expose `wasm_bridge.rs` cancel/reset or deadline-safe execution so timeouts cleanly trigger Stage 1 fallback (`FSR 1.0 EASU + RL`).

2. `app/web/ai_restorer.js`
   - Implement priority two-lane queue (`highQueue` for `forceRun: true` / `priority === 'HIGH'`, `bgQueue` for background auto-enhance): `high` jobs run before pending `bg` jobs.
   - Add `inFlightJobs` (`Map<cacheKey, { promise, subscribers }>`) so identical image requests never duplicate or queue behind themselves and all callers receive monotonic live progress.
   - Replace `reportProgress(2)` with monotonic progress stages: `4%` on queue start, `7%` after fetch/decode, `10%` model selected/loading, `14..88%` neural GPU (or `12..88%` Stage 1), `92%` encoding, `100%` decoded output ready.
   - Guard `detectHardware()` / `predecodeImageUrl()` / `loadModelBytesCached()` / `openDb()` hangs with strict timeouts and call `wasm.abortRealEsrganGpu()` (or bump generation) inside `withTimeout` expiry so no orphaned Rust GPU task collides with the next queued job.

3. `app/web/storage_engine.js`
   - Add `req.onblocked` and bounded `5000ms` timeout in `openDb()` (resetting `dbPromise = null` on failure/timeout) so IndexedDB contention never freezes `getAiOutput` / `getAiModel`.

4. `app/src/components/rich.rs`
   - In `RichImage::run_ai_pipeline`:
     - Guard against duplicate concurrent runs (`if *ai_loading.peek() { return; }`) and only emit monotonic progress (`p32 > *ai_progress.peek()` or terminal `100`).
     - Pass `priority: 'HIGH'` when triggered interactively (or live incoming), and `priority: 'NORMAL'` for background history `image/avif` passes.
     - Add a Rust-side watchdog deadline (e.g. `22_000ms` via `gloo_timers` / JS `Promise.race` guarantee + Rust `eval` fallback timer) so `ai_loading` is guaranteed to resolve/clear even if an unexpected JS/WASM failure occurs.
   - In `MediaLightbox::run_lb_enhance`:
     - Handle missing `original_src` cleanly by clearing `auto_enhance_on_open` instead of leaving `is_lb_enhancing = true` at `3%`.
     - Ensure monotonic progress (`p32 > *lb_progress.peek()`) and `priority: 'HIGH'`.
