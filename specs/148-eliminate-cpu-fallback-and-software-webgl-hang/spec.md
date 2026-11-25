# Spec 148: Eliminate Synchronous WASM CPU Fallback & Software WebGL2 Hangs

## 1. Root Cause Found
1. In `app/src/storage/realesrgan_gpu.rs:3256`, if WebGL2 was unavailable, lost context, returned any error, or `width * scale > max_tex_size`, `restore_image_realesrgan_rust` fell through to **synchronous single-threaded WASM CPU `run_span_tiled`**, freezing the entire browser main thread for 2–5 minutes (blocking all `setTimeout` watchdogs and locking every other chat image at `3%`).
2. In default Linux Chrome without `--enable-unsafe-webgpu`, ANGLE falls back to **SwiftShader (`webgl2Software: true`)**, where 4× SPAN takes ~1000 ms/tile (60–90+ seconds per image), starving the queue at `3%`!
3. `window.__chatwStorage.getAiOutput` and `getAiModel` had no timeout guards.

## 2. Acceptance Criteria
1. **Zero Main-Thread CPU Fallback Freeze**: `restore_image_realesrgan_rust` NEVER falls through to synchronous WASM CPU inference on the browser main thread; it returns a clean `Err` so Stage 1 (`AMD FSR 1.0 EASU 2× + RL Deconvolution`) delivers `100%` in `< 300ms`.
2. **Software WebGL2 (`SwiftShader` / `llvmpipe`) Fast Path**: When `webgl2Software === true` or `!webgl2Available`, skip multi-tile neural SPAN and deliver Stage 1 at `100%` in `< 400ms` per image.
3. **Hardware GPU Wall-Clock Budget**: Inside the WebGL2 tile loop, enforce a hard `5500ms` time budget so an overloaded/throttled GPU can never stall the queue.
4. **IndexedDB Timeout Guards**: `getAiOutput`, `getAiModel`, and `getSenderOriginalFromCache` all have `<= 2000ms` timeouts.
5. **Verified on BOTH Default Chrome (SwiftShader) AND Hardware WebGPU Chrome**: 3 concurrent `512×384` images MUST ALL reach `100%` in `< 3s` on SwiftShader AND `< 4s` on Hardware GPU!
