# Technical Plan: Spec 156

1. **`app/web/ai_restorer.js`**:
   - Set `maxInEdge = 900` on hardware WebGL2 and `maxOutEdge` default `2560` so 4× neural upscaling runs in a single GPU pass and encodes/decodes rapidly without compositing multi-tile 4320px textures.
   - Gate `faceSnapCanvas` 2D canvas readback & `enqueueFaceBg` strictly on `typeof meta.onRefined === 'function' && maxDim < 480 && !faceCandidateIsAnime` so Lightbox & normal photos never block the main thread for `1,007 ms` loading `ort.all.min.js` + `YuNet` + `GFPGANv1.4`.
   - Yield a clean UI slice (`await yieldUiSlice()`) before `createImageBitmap(res.canvas)`.
2. **`app/src/storage/realesrgan_gpu.rs`**:
   - Raise `single_pass_ok` from `w * h <= 600_000` to `w * h <= 1_100_000` so standard HD/900px photos process in 1 single GPU pass with zero tile-overlap overhead.
3. Rebuild web bundle (`./app/build-web.sh`), re-run `diag_lb_lag.mjs` in real Chrome WebGL2 to confirm the `1,007 ms` freeze is gone and Lightbox AI is fast & silky smooth, commit & push.
