# Spec 114: Zero-Jank Non-Blocking Lightbox AI Pipeline (WebGL2 FenceSync + Async PBO + Worker Encode/Decode + Idle Pre-Warm)

## 1. Problem Statement
Real-browser `requestAnimationFrame` profiling when clicking an un-enhanced image to open Lightbox exposed severe main-thread stalls:
- **`40ms` hitch at `t = 74ms`**: full-screen `1600×960` `backdrop-filter: blur(14px)` during Lightbox opacity fade-in + immediate synchronous `getImageData`.
- **`120ms` freeze at `t = 201ms`**: first-run synchronous WebGL2 shader compilation (`6` GLSL programs) + synchronous upload of all 18 FP16 weight textures right on click.
- **`500ms` half-second tab freeze at `t = 830ms`**: synchronous `gl.readPixels()` inside `read_gate_thumbnail()` forcing a full GPU pipeline barrier (`glFinish`) on the main thread while 16 Real-ESRGAN 4K convolution layers execute, immediately followed by synchronous main-thread `canvas.toBlob()` 6.3MP JPEG encoding.

## 2. Goals & Acceptance Criteria
1. **Eliminate the `500ms` GPU Readback & JPEG Encode Freeze**:
   - Replace synchronous `gl.read_pixels_with_opt_u8_array` with WebGL2 **Async `PIXEL_PACK_BUFFER` (PBO) + non-blocking `gl.fenceSync(SYNC_GPU_COMMANDS_COMPLETE)`** polled via `gl.clientWaitSync(sync, 0, 0)` on `requestAnimationFrame` (`0.05ms`/frame CPU).
   - Wait non-blockingly on `gl.fenceSync` between 4-layer GPU chunks so Chromium's compositor stays silky 60/120fps and progress `%` reflects real GPU execution smoothly.
   - Offload both source image `blob -> RGBA` decoding AND final 4K `ImageBitmap -> JPEG/WebP Blob` encoding to a persistent background **Web Worker (`OffscreenCanvas`)** with zero-copy Transferables (`ArrayBuffer` / `ImageBitmap`).
2. **Eliminate the `120ms` First-Click Compile/Upload Hitch**:
   - Expose `warmupRealEsrganGpu()` from Pure-Rust WASM and run incremental idle pre-warming (`requestIdleCallback`) after boot so shaders are already compiled before the user clicks an image.
   - Chunk first-time FP16 weight texture uploads (`4` layers per slice) with browser yields so even cold-start never stalls a frame.
   - Yield 2 `requestAnimationFrame` frames after Lightbox mount before launching AI work so Frame-0 open animation paints cleanly with zero stutter.
3. **Eliminate the `40ms` Full-Screen Backdrop-Blur Fade Hitch**:
   - Replace full-viewport `1600×960` `backdrop-filter: blur(14px)` on `.lightbox` with a clean deep darkroom obsidian scrim (`rgba(7, 9, 14, 0.94)`).
4. **Hard Quantitative Performance Gate**:
   - Re-run the exact same `requestAnimationFrame` main-thread stall profiler on clicking an un-enhanced image:
     - **Max single main-thread `rAF` gap MUST drop from `500ms` to `< 55ms` (≥ 9× reduction, zero half-second freeze!)**
     - Total GPU enhancement speed remains ultra-fast (`~600–900ms` real RX 590 GPU).
     - All existing tests (`cargo test`, Spec 113, Spec 112, Spec 101, Clean AI UI, Spec 111) pass 100%!
