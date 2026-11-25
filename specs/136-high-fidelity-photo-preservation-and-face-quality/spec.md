# Feature Specification: Spec 136 — High-Fidelity Photo Preservation & Face Quality

## 1. Problem Statement
The user reported that AI image enhancement continues to severely distort high-detail portraits, particularly around the eyes and facial features:
- `"nó vẫn thế vẫn quá xấu nó vẫn không được như ảnh 2 ảnh gốc ,...."` (It's still like that, still too ugly, it still can't match Image 2 original...)
- Attached references:
  - `https://github.com/dinhkhactientl2/upscale-pro` (Upscale Photo Pro - offline upscale and face preservation)
  - `https://github.com/ncoevoet/facet` (Facet - AI photo scoring & face sharpness preservation)
- Attached images:
  1. **Image 1** (`media_1795549605637_e22ee9ba.jpg`): The distorted output in ChatW with deformed double-crease lower eyelid wrinkles, alien pupil shapes, and heavily smudged plastic skin.
  2. **Image 2** (`media_1795549648014_4417d69b.jpg`): The pristine 1024x768 camera original featuring natural skin pores, individual eyelashes, pink aegyo-sal makeup, and natural hair strands.

### Root Cause Analysis
1. **Destructive Upload Downscaling**:
   - `adaptiveMaxEdge` in `app/web/media_worker.js` and `app/web/media_optimizer.js` forcefully downscaled any image with `maxEdge > 900` to 50% (`Math.round(maxEdge * 0.5)`).
   - A 1024x768 photo was crushed down to **512x384** (discarding 75% of all pixels!).
2. **Brutal Quantization at Low Resolution**:
   - `media_worker.js` and `media_optimizer.js` compressed the already-halved 512x384 image with `quality = 0.38`.
   - In 512x384 at q=0.38, each eye was reduced to roughly 20x12 blocky pixels with heavy quantization noise.
3. **Destructive 4× Neural Super-Resolution on Degraded Inputs**:
   - When opened in Lightbox or restored, 4× Real-ESRGAN attempted to hallucinate missing pixels from the blocky 20x12 eye.
   - Real-ESRGAN interpreted AVIF compression artifacts as edges, hallucinating grotesque alien eyelid folds, smearing skin pores into flat plastic, and creating Image 1.
4. **Lack of Selective High-Res Bypass**:
   - As demonstrated in `ncoevoet/facet` and `dinhkhactientl2/upscale-pro`, when an image already has high resolution and sharp facial features, applying 4× generative neural super-resolution degrades quality rather than improving it. High-resolution images benefit from native-resolution edge sharpening (AMD FidelityFX CAS) rather than generative 4× scaling.
5. **Stale Service Worker Caching**:
   - Outdated Service Worker cache versions served stale JS bundles to users who had already visited the site.

---

## 2. Goals & Acceptance Criteria
- [ ] **High-Resolution Photo Preservation (Zero Downscale under 1920px)**:
  - For photos and portraits under 1920px (e.g. 1024x768, 1280x720, 1920x1080), preserve 100% full original resolution (`tw = origW, th = origH`).
  - Only downscale images exceeding 2048px (e.g. 48MP raw camera captures) down to 1920px max dimension.
  - In `data_saver` mode, clamp to max 1280px (never 512px).
- [ ] **High-Fidelity AVIF Quality (`q = 0.78 - 0.82`)**:
  - Increase default AVIF quality for photos/portraits from `0.38` to `0.80` (80%).
  - At q=0.80, AVIF retains crisp eyelashes, skin pores, natural eye reflections, and makeup with SSIM > 0.985, while keeping file size tiny (~35-60 KB for 1024x768).
  - In `crates/chatw-media-core/src/avif.rs`, increase candidate quality for `Portrait` and `Photo` to `78.0..84.0` and eliminate the aggressive 540px clamp for standard photos.
- [ ] **Smart Face & High-Resolution Restoration Strategy**:
  - When an image is already high-resolution (`procW >= 960 || procH >= 720`) and has not been downscaled (`targetWidth <= width`):
    - Do NOT run destructive 4× Real-ESRGAN super-resolution that blows it up to 4096px and alters facial features.
    - Instead, apply 1× native AMD FidelityFX CAS (Contrast Adaptive Sharpening) to enhance clarity without mutating eye geometry or smoothing skin.
  - When an image is genuinely low-resolution (`procW < 720 && procH < 720` or downscaled `width < targetWidth`), continue using Real-ESRGAN 4× with GeneralX4V3.
- [ ] **Service Worker Cache Invalidation**:
  - Bump `STATIC_CACHE_NAME` in `app/web/sw.js` and `app/dist/sw.js` to `chatw-static-v14` so all clients receive updated scripts immediately.
- [ ] **E2E Verification & Mandatory Auto-Push (Rule 1)**:
  - Run Playwright E2E verification test uploading Image 2 and confirming that transmitted resolution is 1024x768, quality is pristine, and Lightbox renders sharp natural eyes without artifacts.
  - Automatically commit and push all changes to GitHub `origin main`.
