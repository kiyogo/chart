# Plan: Spec 146 — Fix 90% / 3% Hybrid AI Hang
- Update `app/web/face_restorer.js`: remove 340 MB `CacheStorage.put`, add `withTimeout` helper (4s YuNet, 7s GFPGAN), add `gfpganDisabled` circuit breaker on timeout/failure, and validate WebGPU `requestDevice()` works before attempting GFPGAN!
- Update `app/web/ai_restorer.js`: wrap Stage 2B in 8s `Promise.race` guard, bump `PIPELINE_VERSION = 'hybrid-span-gfpgan-v23'`.
- Update `app/web/sw.js`: bump `STATIC_CACHE_NAME = 'chatw-static-v23'`.
- Build (`./app/build-web.sh`) and run Playwright E2E testing 3 simultaneous images in BOTH standard Chrome (no Vulkan flag!) and WebGPU Chrome to guarantee 0% hangs!
- Commit and push to `origin main`.
