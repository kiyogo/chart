# Tasks: Spec 146 — Fix 90% / 3% Hybrid AI Hang
- [x] **T1**: Add strict timeouts, `requestDevice()` probe, circuit breaker, and remove 340 MB `CacheStorage.put` in `app/web/face_restorer.js`.
- [x] **T2**: Add 9s `Promise.race` guard around Stage 2B in `app/web/ai_restorer.js` (`v23`) and bump `app/web/sw.js` (`chatw-static-v23`).
- [x] **T3**: Build (`./app/build-web.sh`) and run E2E verifying 3 concurrent images complete 100% without hanging at 3% or 90% in standard Chrome (no Vulkan flag).
- [x] **T4**: Check off tasks, commit, and `git push origin main`.
