# Spec 114 Technical Plan

## 1. WebGL2 Sync & PBO Features (`app/Cargo.toml`)
- Add `"WebGlSync"` and `"WebGlBuffer"` to `web-sys` features in `app/Cargo.toml`.

## 2. Non-Blocking Async FenceSync + PBO + Chunked Weight Upload (`app/src/storage/realesrgan_gpu.rs` & `wasm_bridge.rs`)
- Add `await_gl_fence_nonblocking(gl, sync)` polling `gl.client_wait_sync_with_u32(sync, 0, 0)` every `requestAnimationFrame` (`0ns` timeout -> ZERO main-thread block!), cleaning up `gl.delete_sync(Some(sync))` cleanly.
- Add `ensure_model_uploaded_async(model_id, model_bytes)`: parses CWSR1 once and uploads FP16 weight textures in 4-layer batches yielding a browser task between batches (`< 6ms` per slice).
- In `run_tile_gl_async`:
  - After each 4-layer `step_body_layers` flush, drop a `gl.fence_sync` and `await_gl_fence_nonblocking` before emitting the next smooth `%` progress tick!
- Replace synchronous `read_gate_thumbnail` with 2-phase Async PBO:
  - `begin_gate_thumbnail_pbo(ow, oh, gw, gh)` -> blits `ow×oh -> hw×hh -> qw×qh -> gw×gh`, creates `PIXEL_PACK_BUFFER`, issues async `gl.read_pixels_with_i32(0, 0, gw, gh, RGBA, UNSIGNED_BYTE, 0)`, returns `(pbo, sync, byte_len)` in `< 0.2ms`!
  - `await_gl_fence_nonblocking` -> waits for GPU + DMA without stalling main thread!
  - `finish_gate_thumbnail_pbo(pbo, byte_len)` -> calls `gl.get_buffer_sub_data_with_i32_and_u8_array` on the already-completed PBO (`~0.1ms`!) and deletes the PBO.
- Export `warmupRealEsrganGpu()` in `app/src/storage/wasm_bridge.rs` (initializing the WebGL2 context & compiling shaders once cleanly during idle time).

## 3. Off-Main-Thread Worker Decode/Encode + Idle Pre-Warm (`app/web/ai_restorer.js`)
- Add a tiny persistent inline Blob Web Worker (`aiMediaWorker`) using `OffscreenCanvas`:
  - `decode`: receives `Blob` + `maxInEdge` -> `createImageBitmap` + `OffscreenCanvas` resize + `getImageData` -> returns `{ rgba, procW, procH, rawW, rawH }` via zero-copy Transferable `[rgba.buffer]`! (Fallback to main thread if `OffscreenCanvas` unsupported.)
  - `encode`: receives zero-copy Transferable `ImageBitmap` + `hasAlpha` + `maxOutEdge` -> `OffscreenCanvas` + `convertToBlob({ type: 'image/jpeg', quality: 0.93 })` -> returns encoded `Blob` with **0ms main-thread JPEG encode stall!**
- Schedule `requestIdleCallback` (fallback `setTimeout(..., 1500)`) after boot to call `window.__chatwWasmMedia.warmupRealEsrganGpu()` AND pre-fetch cached `.cwsr` model bytes into RAM if already in IndexedDB.
- Yield `nextFrame()` cleanly around WASM calls so UI paint frames never drop.

## 4. Clean Frame-0 Lightbox Paint & Darkroom Backdrop (`app/src/components/rich.rs`, `app/assets/main.css`)
- In `GlobalImageLightbox::run_lb_enhance`: await 2 `requestAnimationFrame` paints before invoking `restoreImageFromUrl` so the Lightbox opens with instant silky smoothness.
- In `app/assets/main.css`: replace full-screen `1600×960` `backdrop-filter: blur(14px)` on `.lightbox` with crisp darkroom scrim `background: rgba(7, 9, 14, 0.94);` (eliminates the `40ms` full-screen backdrop blur fade hitch!).

## 5. Verification
- Run `app/e2e/spec114-zero-jank-lightbox-ai-profile.mjs` measuring live `requestAnimationFrame` gaps from click -> 4K completion, asserting `maxGapMs < 55ms` (down from `500ms`!).
- Re-run `spec113`, `spec112`, `spec101`, `verify_clean_ai_ui_and_flows`, and `cargo test`.
