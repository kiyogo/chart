# Spec 174: Fix AI Image Enhancement (`Làm nét ảnh`) Stalling at 3%

## Problem Statement
When enhancing images ("Làm nét ảnh" on chat images, auto-enhancing incoming/compressed AVIF images, or opening unenhanced images in the Lightbox), progress frequently freezes at `3%` (`● 3%` badge over the frosted blur overlay).

## Root Causes Identified
1. **Queue Starvation & Early `3%` Emission**: `RichImage::run_ai_pipeline` emits `{ type: 'start', pct: 3 }` before calling `window.__chatwAiRestorer.restoreImageFromUrl`. Multiple auto-enhancing `image/avif` chat images enter a single unprioritized FIFO `enqueueExclusive` queue, leaving interactive user clicks ("Làm nét ảnh" / Lightbox) queued behind background jobs at `3%` (and resetting backwards to `2%` when `reportProgress(2)` runs).
2. **Duplicate In-Flight Work**: Concurrent enhancement requests for the same `cacheKey` queue redundantly instead of sharing the active promise and forwarding progress updates.
3. **Unbounded GPU Single-Pass & Fence Wait / Orphaned WASM Futures**:
   - In `realesrgan_gpu.rs`, `single_pass_ok` allows up to `1024×1024` 4× (`4096×4096` output + 7 RGBA16F texture arrays) without checking `max_gpu_budget_ms`, and `await_gl_fence_nonblocking` can spin up to `600 × 8ms = 4.8s` per fence across 7+ fences (`> 18s`).
   - When JS `withTimeout(..., 18000)` times out, the orphaned Rust `future_to_promise` keeps running and calling `with_gl_engine`, racing with subsequent jobs.
4. **Decode Stage Stall at `6%` & Unbounded `createImageBitmap` / `toBlob`**:
   - Even after jumping ahead of the queue, decoding inside `ai_restorer.js` re-fetched the blob and waited up to `10s` (`img_fetch`) + `10s` (`img_blob`) + `6s` (`callAiMediaWorker`) + unbounded `createImageBitmap(blob)` instead of directly reading from the already-decoded DOM `<img>` or falling back quickly via `HTMLImageElement`.
5. **No Rust-Side Eval Deadline / Re-Entry Guard & Lightbox Edge Cases**:
   - `RichImage` lacks a double-trigger guard and Rust-side watchdog around `eval.recv()`, and `MediaLightbox` leaves `is_lb_enhancing` permanently stuck at `3%` if `original_src` is missing while `auto_enhance_on_open` is `true`.

## Acceptance Criteria
1. Interactive enhancements (`forceRun: true` from chat corner button or Lightbox) take priority over background auto-enhance jobs and start advancing smoothly past `3%` without waiting behind background queues.
2. Duplicate concurrent calls for the same `cacheKey` deduplicate in-flight and share live progress + final result.
3. Progress is strictly monotonic (`>= 3%` once started, advancing through decode `6%` -> model selection/load `10..15%` -> GPU/FSR1 `16..90%` -> encode `92..100%`) — never regresses to `2%`.
4. `realesrgan_gpu.rs` caps single-pass texture dimensions safely (`w * h <= 520_000`), enforces `max_gpu_budget_ms` and cooperative job generation cancellation inside every tile/block/fence loop, and cleanly cleans up `GL_ENGINE` active tiles on cancellation/budget exceed so Stage 1 (FSR 1.0 + RL) delivers a crisp result immediately.
5. `RichImage` and `MediaLightbox` enforce a Rust-side watchdog timeout and clean error/fallback recovery so no overlay ever remains stuck at `3%`.
6. Verified via release WASM build, `cargo check`, Playwright GPU + software WebGL2 tests (`0` page errors, `3% -> 100%` completion under concurrency), live deploy to `https://chat.kiyo.vn/`, and pushed to `origin/main`.
