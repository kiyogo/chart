# Spec 156: Eliminate Lightbox Image & AI Sharpen Lag

## Problem Statement
Profiling `GlobalImageLightbox` open + clicking the AI sharpen button in Chrome (`diag_lb_lag.mjs`) revealed:
1. A **`1,007 ms` main-thread `longtask` (`1,067 ms` rAF freeze)** right after Lightbox AI finishes, triggered by `ai_restorer.js` unconditionally scheduling `enqueueFaceBg` (`face_restorer.js`: loading `ort.all.min.js`, compiling `YuNet` WASM, and initializing `GFPGANv1.4.onnx` WebGPU on the main thread) even when `meta.onRefined` is unset (as in Lightbox) and when the image already has sharp high-res `NomosUni` / `SPAN` reconstruction.
2. Unnecessary **`136 ms` + `81 ms` main-thread stalls** caused by synchronous `snap.getContext('2d').drawImage(res.canvas)` readback before `createImageBitmap`, oversized `1080px` -> `4320px` tile splitting above `600k` pixels (`single_pass_ok`), and `3840px` output encode/predecode in Lightbox.

## Acceptance Criteria
1. Clicking an image and clicking AI sharpen produces **zero** `> 100 ms` main-thread `longtask` or rAF stalls (`1,007 ms` freeze completely eliminated).
2. Hardware WebGL2 single-pass threshold covers up to `1_100_000` pixels with `maxInEdge = 900` (and `maxOutEdge` capped at `2560` unless overridden), avoiding wasteful 4-tile overlaps while keeping full QHD/Retina clarity.
3. Heavy 325 MB ONNX face post-pass (`face_restorer.js`) never runs unless `typeof meta.onRefined === 'function'` and source max dimension `< 480px`.
