# Tasks 148: Eliminate Synchronous WASM CPU Fallback & Software WebGL2 Hangs

- [x] T1: Reproduce 60–90s SwiftShader hang on default Linux Chrome (`512×384` SPAN 4×) and locate synchronous WASM CPU fallback in `realesrgan_gpu.rs:3256`.
- [x] T2: Remove synchronous main-thread WASM CPU fallback and add software-WebGL2 guard + `5500ms` tile budget in `app/src/storage/realesrgan_gpu.rs`.
- [x] T3: Add IndexedDB timeouts (`getAiOutput`, `getAiModel`) and software-WebGL2 fast Stage 1 path in `app/web/ai_restorer.js` (`v25`).
- [x] T4: Add `getSenderOriginalFromCache` timeout in `app/src/components/rich.rs` and bump `app/web/sw.js` to `chatw-static-v25`.
- [x] T5: Build `./app/build-web.sh`, run `cargo test -p chatw-media-core` (42/42 passed), and verify BOTH default Chrome (SwiftShader) AND Hardware WebGPU Chrome on 3 concurrent `512×384` images.
- [x] T6: Commit and push to `origin/main`.
