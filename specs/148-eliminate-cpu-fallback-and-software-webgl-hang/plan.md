# Plan 148: Eliminate Synchronous WASM CPU Fallback & Software WebGL2 Hangs

1. `app/src/storage/realesrgan_gpu.rs`:
   - Reject `probe.is_software` or `!probe.webgl2_available` immediately before entering neural SPAN/SRVGGNetCompact GPU loop, returning `Err("software_webgl2_use_stage1")` (so `ai_restorer.js` delivers Stage 1 at `100%` in `< 250ms`).
   - Add `js_sys::Date::now()` wall-clock budget (`5500.0` ms) inside the WebGL2 tile loop.
   - Remove the synchronous main-thread WASM CPU `run_span_tiled` / `restore_image_realesrgan_cpu` fallthrough at line 3256 (return `Err("webgl2_unavailable_use_stage1")`).
2. `app/web/ai_restorer.js` (`PIPELINE_VERSION = 'hybrid-span-gfpgan-v25'`):
   - Wrap `getAiOutput` and `getAiModel` in `withTimeout(..., 2000)`.
   - Skip Stage 2A neural SPAN when `!hw.webgl2Available || hw.webgl2Software` so SwiftShader browsers immediately deliver Stage 1 at `100%`.
3. `app/src/components/rich.rs`:
   - Add 1500ms timeout around `getSenderOriginalFromCache` in `run_ai_pipeline`.
4. `app/web/sw.js`:
   - Bump `STATIC_CACHE_NAME = 'chatw-static-v25'`.
5. Rebuild `./app/build-web.sh`, run `cargo test -p chatw-media-core`, and verify in BOTH default Chrome (SwiftShader) AND Hardware WebGPU Chrome on 3 concurrent `512×384` images.
