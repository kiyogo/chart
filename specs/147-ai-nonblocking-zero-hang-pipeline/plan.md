# Plan 147: Zero-Hang Non-Blocking Hybrid AI Image Restoration Pipeline

## Architecture Changes

### 1. `app/web/ai_restorer.js` (`PIPELINE_VERSION = 'hybrid-span-gfpgan-v24'`)
- Add `withTimeout(promise, ms, label)` utility.
- Harden `callAiMediaWorker(payload, transfers, timeoutMs = 6000)` with a per-call timeout that cleans up `aiWorkerPending`, terminates a wedged worker, and returns `null` so the caller seamlessly falls back to 2D canvas without hanging.
- Harden `detectHardware()` (`getBattery` 1000ms), `fetch(imageUrl)` (10000ms), `loadModelBytesCached` (10000ms), and `predecodeImageUrl` (2500ms).
- Harden `enqueueExclusive(fn, timeoutMs = 18000)` so every queued job is guaranteed to settle within 18s and advance the queue.
- **Decouple Stage 2B (`face_restorer.js`) from the critical `0..100%` path**:
  - Immediately encode Stage 2A (`NomosUni SPAN`) result, emit `reportProgress(100)`, store in `sessionResultCache` + IndexedDB, and return `entry` so the UI reveals the crisp AI image in `~1s` and releases `enqueueExclusive` for the next image!
  - If `!sel.isAnime` and `window.__chatwFaceRestorer?.restoreFacesOnCanvas` exists, snapshot the Stage 2A canvas into a dedicated background `faceQueue` (separate from `enqueueExclusive`!).
  - When `faceQueue` finishes refining faces with `YuNet + GFPGANv1.4`, encode the refined canvas, predecode, update `sessionResultCache` + IndexedDB under the same `cacheKey`, and invoke `meta.onRefined(refinedUrl, metaInfo)` so the UI seamlessly swaps in the refined portrait with zero loading overlay.

### 2. `app/web/face_restorer.js`
- Add `withTimeout` to `loadOrt()` (4s) and `detectFaces()` `sess.run` (4s).
- Add single-flight `gfpganInitPromise` so concurrent background jobs never double-fetch `GFPGANv1.4.onnx`.
- Ensure `restoreFacesOnCanvas` never throws and always cleans up temporary canvases.

### 3. `app/src/components/rich.rs`
- Wrap chat bubble (`run_ai_pipeline`) and Lightbox (`upscaling_ai`) JS `eval` blocks in `try { ... } catch (err) { dioxus.send({ type: 'failed', error: String(err) }); }`.
- Handle `"refined"` event from `onRefined` to seamlessly upgrade `src` (and Lightbox `GLOBAL_LIGHTBOX`) when background Stage 2B finishes.
- After `while let Ok(v) = eval.recv().await` exits, unconditionally reset `ai_loading.set(false)` / `upscaling_ai.set(false)` if still `true`.

### 4. `app/web/sw.js`
- Bump `STATIC_CACHE_NAME = 'chatw-static-v24'` to guarantee clean cache rollover.
- Explicitly bypass `/models/` and `/ort/` in the `fetch` handler so Service Worker never touches large ONNX/WASM assets.
