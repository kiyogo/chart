# Spec 147: Zero-Hang Non-Blocking Hybrid AI Image Restoration Pipeline

## 1. Problem Statement
Users still observe chat images and lightbox previews freezing at `90%` (active image) and `3%` (queued images) on real desktop browsers with WebGPU enabled:
1. Stage 2B (`GFPGANv1.4`, 325 MB ONNX) executed **synchronously inside `enqueueExclusive`** before `reportProgress(100)`, freezing the active image at `90%` for 6–15+ seconds and starving every subsequent image in the chat at `3%`.
2. Several `await` calls inside `ai_restorer.js` (`callAiMediaWorker`, `fetch`, `loadModelBytesCached`, `detectHardware`, `predecodeImageUrl`) and `face_restorer.js` (`loadOrt`, `detectFaces`) lacked hard timeouts, allowing a single stalled worker or GPU op to permanently deadlock `enqueueExclusive`.
3. `app/src/components/rich.rs` lacked `try/catch` around `restoreImageFromUrl` and lacked post-loop cleanup after `while let Ok(v) = eval.recv().await`, so any JS exception or aborted eval left `ai_loading = true` (`3%` / `90%`) permanently.

## 2. Goals & Acceptance Criteria
1. **Zero `90%` / `3%` Hangs**: Every image MUST reach `100%` (Stage 1 Algorithmic + Stage 2A Pure-Rust WebGL2 `NomosUni SPAN`) in `< 2.5s` on desktop GPU without ever stalling at `90%` or starving other images at `3%`.
2. **Non-Blocking Background Stage 2B (`GFPGANv1.4`)**: Face restoration (`YuNet` + `GFPGANv1.4` WebGPU) runs on a separate low-priority background queue **after** Stage 2A has already delivered `100%` to the UI and released `enqueueExclusive`. When Stage 2B completes, it seamlessly upgrades the displayed image via `onRefined` and updates both RAM and IndexedDB caches.
3. **Guaranteed Timeout & Recovery on Every Async Boundary**:
   - `callAiMediaWorker`: 6s timeout with automatic worker termination & fallback to main-thread canvas.
   - `fetch(imageUrl)` / `loadModelBytesCached`: 10s timeout.
   - `detectHardware()` (`navigator.getBattery`): 1s timeout.
   - `predecodeImageUrl`: 2.5s timeout.
   - `enqueueExclusive`: hard 18s per-job watchdog timeout so no single image can ever deadlock the queue.
   - `face_restorer.js`: strict timeouts on `loadOrt` (4s), `detectFaces` (4s), and single-flight `gfpganInitPromise`.
4. **Guaranteed UI Cleanup (`rich.rs`)**: Both chat bubble and Lightbox JS blocks wrapped in `try/catch`, plus unconditional `ai_loading.set(false)` / `upscaling_ai.set(false)` after `eval.recv()` exits.
5. **E2EE & Zero Server GPU Intact**: 100% on-device execution, zero plaintext upload, zero regression in image quality.
