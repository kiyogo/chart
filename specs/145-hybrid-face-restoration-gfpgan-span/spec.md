# Spec 145: Hybrid Multi-Expert AI Image Restoration (`NomosUni SPAN` + `YuNet` Face Detection + `GFPGANv1.4` WebGPU + Adaptive Mask Blending)

## 1. Problem Statement
General super-resolution models (`SPAN`, `Real-ESRGAN`, `LSDIRCompact`) only see local 3×3 patches and lack semantic priors for human eyes, eyelashes, pupils, and lips. On heavily compressed low-resolution AVIF photos (`512×384`, ~14 KB — 29× smaller than PNG), a general upscaler alone cannot match Picsart-grade facial clarity.

## 2. Hybrid Multi-Expert Architecture (Zero Server GPU, 100% On-Device E2EE)
1. **Stage 1 (Instant Algorithmic `< 150ms`)**: AMD FSR 1.0 EASU + Regularized RL Deconvolution renders an instant crisp preview.
2. **Stage 2A (Background / Hair / Clothing / Jewelry Expert)**: Pure-Rust WebGL2 6-MRT FP16 **`NomosUni SPAN 2× / 4×`** (`CC-BY-4.0`, 802–833 KB) cleanly restores hair, clothing, silver necklace, and preserves intentionally blurred bokeh backgrounds with zero crosshatch.
3. **Stage 2B (Dedicated Face Expert — Lazy-Loaded WebGPU Only)**:
   - **Face Detection**: OpenCV Zoo **`YuNet` (`face_detection_yunet_2023mar.onnx`, MIT, 228 KB)** detects up to 4 faces (`score >= 0.55`, NMS) and outputs exact 5-point landmarks (`left_eye`, `right_eye`, `nose`, `left_mouth`, `right_mouth`).
   - **Canonical 5-Point Alignment**: Umeyama 2D least-squares similarity transform aligns each face to `GFPGANv1.4`'s `512×512` FFHQ template.
   - **Face Restoration**: **`GFPGANv1.4.onnx` (`Apache-2.0`)** runs via `onnxruntime-web` on client **WebGPU** (~1.6s/face), lazy-downloaded on demand and cached permanently in browser `CacheStorage` (never bundled into Git/installer, never runs simultaneously with SPAN, gracefully skipped when WebGPU is unavailable or on non-face images).
   - **Adaptive Elliptical Feathered Mask Blending + Local Skin-Tone Color Lock**: Inverse-warps the restored face back onto the `NomosUni SPAN` canvas, locks mean RGB inside the core face ellipse (`85%` color transfer) to eliminate tone seams, and blends via a smooth cosine-feathered elliptical mask (`alpha = 0.82` for `< 720px` heavily compressed photos, `0.55` for `>= 720px` clean photos) so facial identity is preserved and bokeh backgrounds are never touched.

## 3. Acceptance Criteria
1. Bundle `face_detection_yunet_2023mar.onnx` (`228 KB`, MIT) and `ort.all.min.js` (`764 KB`, MIT) while keeping large weights (`GFPGANv1.4.onnx`, `.wasm`) out of Git via `.gitignore` and lazy-cached in `CacheStorage`.
2. Implement `app/web/face_restorer.js` (`window.__chatwFaceRestorer`) with NMS multi-face support, Umeyama alignment, WebGPU `GFPGANv1.4`, local skin-tone color lock, elliptical cosine feathering, and `AbortSignal` cancellation.
3. Wire Stage 2B cleanly into `app/web/ai_restorer.js` (`v22`), `app/web/index.html`, `app/build-web.sh`, and `app/web/sw.js` (`chatw-static-v22`).
4. Register real SHA-256 checksums for `yunet_face_det_2023mar` (`MIT`) and `gfpgan-v1.4-identity-safe` (`Apache-2.0`) in `crates/chatw-media-core/src/ai/registry.rs`.
5. Run real end-to-end Playwright verification (`app/e2e/spec145-hybrid-face-e2e.mjs`) on the crushed `512×384` (14 KB) AVIF portrait AND `1024×768` (33 KB) AVIF portrait, visually inspecting 400%-zoom PNG crops via `read_file` to confirm Picsart-grade crisp eyes, eyelashes, hair, necklace, and zero seams.
6. Auto-commit and `git push origin main` (Rule 1).
