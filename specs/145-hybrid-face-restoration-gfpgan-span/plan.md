# Implementation Plan: Spec 145 — Hybrid `NomosUni SPAN` + `YuNet` + `GFPGANv1.4` WebGPU

## 1. Assets & `.gitignore` Hygiene
- Copy `scratch/hybrid_models/face_detection_yunet_2023mar.onnx` (228 KB, MIT) → `app/public/models/face_detection_yunet_2023mar.onnx` + `LICENSE-MIT-YUNET.txt` + `LICENSE-APACHE-2.0-GFPGAN.txt`.
- Copy `ort.all.min.js` + `ort-wasm-simd-threaded.jsep.mjs` → `app/public/ort/` (tracked) and `ort-wasm-simd-threaded.jsep.wasm` + `GFPGANv1.4.onnx` → local `app/public/` (added to `.gitignore` so Git stays lightweight while local dev/E2E serves them directly!).

## 2. Registry Update (`crates/chatw-media-core/src/ai/registry.rs`)
- Add `yunet_face_det_2023mar` (`MIT`, 232,589 bytes, real SHA-256) and update `gfpgan-v1.4-identity-safe` (`Apache-2.0`, 340,256,686 bytes, real SHA-256).

## 3. Hybrid Face Restoration Module (`app/web/face_restorer.js`)
- Expose `window.__chatwFaceRestorer = { restoreFacesOnCanvas(canvas, opts), isWebGpuAvailable(), warmCheck() }`.
- Lazy-loads `/ort/ort.all.min.js` on first portrait invocation; caches `YuNet` & `GFPGANv1.4` `InferenceSession` instances (session pooling, zero re-compile!).
- Persistent `CacheStorage` (`chatw-ai-models-v1`) for `GFPGANv1.4.onnx` with `/models/GFPGANv1.4.onnx` → HuggingFace CDN fallback.
- Multi-face detection with IoU NMS (`maxFaces = 4`, `score >= 0.55`, `minFacePx = 36`).
- Umeyama 5-point similarity alignment → WebGPU `GFPGANv1.4` → local core-ellipse skin-tone color lock (`85%`) → smooth cosine elliptical mask (`alpha = 0.82` for `< 720px` crushed photos, `0.55` for `>= 720px` clean photos) → inverse transform back onto `NomosUni SPAN` canvas.

## 4. Pipeline Integration (`app/web/ai_restorer.js`, `app/web/index.html`, `app/build-web.sh`, `app/web/sw.js`)
- Include `/face_restorer.js` in `app/web/index.html` and `app/build-web.sh`.
- In `app/web/ai_restorer.js` (`PIPELINE_VERSION = 'hybrid-span-gfpgan-v22'`): after Stage 2A (`NomosUni SPAN 2×/4×`) produces a clean canvas for a photo/portrait (not anime, not text/QR, not low-battery, not aborted), invoke `window.__chatwFaceRestorer.restoreFacesOnCanvas(canvas, { srcMaxDim, signal })` sequentially as Stage 2B!
- Bump `STATIC_CACHE_NAME = 'chatw-static-v22'` in `app/web/sw.js`.

## 5. Verification
- `cargo test -p chatw-media-core`
- `./app/build-web.sh`
- `node app/e2e/spec145-hybrid-face-e2e.mjs` (verifies crushed `512×384` 14 KB AVIF + `1024×768` 33 KB AVIF through full production `window.__chatwAiRestorer.restoreImageFromUrl` with WebGPU `GFPGANv1.4` + `NomosUni SPAN`, inspecting 400%-zoom PNG crops via `read_file`).
- Commit & `git push origin main`.
