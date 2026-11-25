# Tasks: Spec 156

- [x] Eliminate unconditional `faceSnapCanvas` & `enqueueFaceBg` 1s main-thread stall in `app/web/ai_restorer.js`
- [x] Optimize `maxInEdge` (`900`), `maxOutEdge` (`2560`), and `single_pass_ok` (`1_100_000`) across `ai_restorer.js` and `realesrgan_gpu.rs`
- [x] Rebuild web bundle and verify with Chrome `diag_lb_lag.mjs` LongTask profiler
- [x] Commit & push to `origin/main`
