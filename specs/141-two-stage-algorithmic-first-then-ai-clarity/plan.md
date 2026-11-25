# Technical Plan: Spec 141 — 2-Stage Progressive Pipeline (Algorithmic First → 4× Photo AI + Post-AI Optical Clarity)

## 1. Browser Orchestrator (`app/web/ai_restorer.js`)
- Bump `PIPELINE_VERSION = 'progressive-alg-then-lsdir3-rl-v18'`.
- In `restoreImageFromUrl(imageUrl, meta)`:
  1. **Stage 1 — Instant Algorithmic Sharpening** (when `typeof meta.onStage1 === 'function'` and `wasm.upscaleFsr1` is available):
     - Compute `stage1W`, `stage1H` (2× capped at 2048px for `< 1280px` inputs, 1× native for high-res).
     - Run `wasm.upscaleFsr1(srcU8, procW, procH, stage1W, stage1H, 1.0, null)` (~35ms on WebGL2 GPU).
     - Encode fast off-main-thread JPEG@0.92 via `aiMediaWorker` (`type: 'encode'`) -> `stage1Url = URL.createObjectURL(stage1Blob)`.
     - Pre-decode `stage1Url` and invoke `meta.onStage1(stage1Url)` + `reportProgress(28)`!
     - Store `stage1Fallback = { blob: stage1Blob, width: stage1W, height: stage1H }` so if Stage 2 fails offline, we already have a crisp finished output with 0 wasted work!
  2. **Stage 2 — 4× Photo AI Super-Resolution (`4xLSDIRCompact3`) + Post-AI Optical RL Clarity**:
     - Select model via `wasm.selectRealEsrganModel` (`lsdir_compact3_x4` for all photos/portraits/scenes, `realesr_animevideov3` strictly for pure flat anime).
     - Load 1.18 MB FP16 weights from IndexedDB cache (`loadModelBytesCached`).
     - Run `wasm.restoreImageRealEsrgan(srcU8, procW, procH, sel.modelId, modelBytes, origProfileJson, scaledProgress)` where `scaledProgress` maps `0..100` → `30..85`.
     - **Post-AI Optical RL Clarity Pass** (`85..95`):
       - Read the clean, block-free 4× AI output canvas (`outW × outH`, capped safely to `maxPostEdge = 2560` if needed for speed, or full `outW × outH` up to `3840` on real GPU!).
       - Run `wasm.upscaleFsr1(aiRgba, postW, postH, postW, postH, 1.15, null)` (3-iteration edge-masked Luma RL Deconvolution + dual-scale micro-clarity directly on the clean block-free AI pixels!).
     - Encode final 4K JPEG@0.95 (or WebP@0.92 if alpha) off-main-thread via `aiMediaWorker`.
     - Return `entry` with `stage1Delivered: true`, `modelId: sel.modelId`, `modelLabel: 'Thuật toán → AI 4× (LSDIR v3 + Optical RL)'`, cache in memory & IndexedDB!

## 2. Progressive UI Wiring (`app/src/components/rich.rs`)
- **Chat (`RichImage`)**:
  - Pass `onStage1: (url) => { try { dioxus.send({ type: 'stage1', url }); } catch (_) {} }` into `restoreImageFromUrl`.
  - Handle `"stage1"` in the `while let Ok(v) = eval.recv()` loop:
    - Update `src.set(Some(stage1_url.to_string()))` immediately so the user sees the crisp algorithmic photo right away while AI refines it!
- **Lightbox (`GlobalLightbox`)**:
  - Pass `onStage1: (url) => { try { dioxus.send({ type: 'stage1', url }); } catch (_) {} }` into `restoreImageFromUrl`.
  - Handle `"stage1"` in `run_lb_enhance`:
    - Update `GLOBAL_LIGHTBOX` `src` to `stage1_url` immediately (and keep `upscaling_ai = true` with live progress badge) so the user visibly sees Stage 1 Algorithmic crispness first, then seamless swap to Stage 2 4× AI + Optical Clarity!

## 3. Service Worker (`app/web/sw.js`)
- Bump `STATIC_CACHE_NAME = 'chatw-static-v18'`.

## 4. Verification (`app/e2e/spec141-two-stage-progressive-alg-then-ai.mjs`)
- Verify `onStage1` fires FIRST (`< 180ms`) with a valid `blob:` URL (Stage 1 Algorithmic).
- Verify Stage 2 completes with `lsdir_compact3_x4` + Post-AI RL Optical Clarity (`2048×1536` for 512×384 input, `3840×2880` for 1024×768 input), measurably higher Laplacian sharpness than Stage 2 without post-RL, `SSIM >= 0.98`.
- Visually inspect Stage 1 and Stage 2 400%-zoom face crops via `read_file`.
