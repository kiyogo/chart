# Tasks - Spec 123: Pure Real-ESRGAN AI Super-Resolution with Zero Chromatic Aberration

- [x] Task 1: Rewrite `GL_FIDELITY_CAS_FS` in `app/src/storage/realesrgan_gpu.rs` with pure-luma CAS, zero deconvolution, and zero darkLine clamping <!-- id: 1 -->
- [x] Task 2: Normalize `color1[2]` intensity in `app/src/storage/realesrgan_gpu.rs` to 1.0 <!-- id: 2 -->
- [x] Task 3: Rebuild web client bundle via `./app/build-web.sh` <!-- id: 3 -->
- [x] Task 4: Run Playwright E2E verification test `app/e2e/spec123-pure-ai-super-res.mjs` and verify zero chromatic fringing and clean AI edges <!-- id: 4 -->
- [x] Task 5: Mandatory auto-push to `origin main` following Rule 1 <!-- id: 5 -->
