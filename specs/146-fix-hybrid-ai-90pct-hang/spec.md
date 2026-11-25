# Spec 146: Fix Hybrid AI 90% / 3% Queue Hang (Timeout Guards, Non-Blocking Stage 2A Delivery & Safe Cache)

## 1. Root Cause
User screenshots showed one image stuck at `90%` and subsequent images stuck at `3%`:
- Line 679 of `app/web/ai_restorer.js` emitted `reportProgress(90)` and awaited `window.__chatwFaceRestorer.restoreFacesOnCanvas(...)` inside `enqueueExclusive` with **no timeout**.
- Inside `app/web/face_restorer.js`, `CacheStorage.put` on the 340 MB `GFPGANv1.4.onnx` or WebGPU session creation without `--enable-features=Vulkan` can hang indefinitely, blocking `enqueueExclusive` so all other queued images stay stuck at `3%`.

## 2. Fix & Acceptance Criteria
1. **Strict Timeout & Circuit Breaker in `app/web/face_restorer.js`**:
   - Never call `CacheStorage.put` for > 50 MB models (avoids Chromium CacheStorage quota/disk hang).
   - Wrap `YuNet`, `GFPGANv1.4` session creation, and inference in strict `Promise.race` timeouts (4s for `YuNet`, 8s for `GFPGANv1.4`).
   - If WebGPU `GFPGANv1.4` times out or fails once, trip a session circuit breaker (`gfpganDisabled = true`) so subsequent photos complete instantly via `NomosUni SPAN` without ever retrying a broken WebGPU driver!
2. **Zero-Hang Queue & Progressive Delivery in `app/web/ai_restorer.js` (`v23`)**:
   - Add a 7-second `Promise.race` guard around Stage 2B (`restoreFacesOnCanvas`) so an image can **never** hang at `90%` and **never** wedge `enqueueExclusive` at `3%`.
   - Bump `PIPELINE_VERSION = 'hybrid-span-gfpgan-v23'` and `app/web/sw.js` `STATIC_CACHE_NAME = 'chatw-static-v23'`.
3. **E2E Verification**:
   - Verify BOTH normal Chrome (without `--enable-features=Vulkan`, simulating the user's real browser!) AND WebGPU-enabled Chrome complete 100% smoothly with **ZERO hang at 90% or 3%**, testing 3 consecutive images in parallel!
