# Tasks: Spec 145 — Hybrid `NomosUni SPAN` + `YuNet` + `GFPGANv1.4` WebGPU

- [x] **T1**: Stage `face_detection_yunet_2023mar.onnx` (228 KB, MIT), licenses, and `ort` JS/MJS loaders in `app/public/`; add large `.onnx`/`.wasm` runtime blobs (`GFPGANv1.4.onnx`, `ort-wasm-*.wasm`) to `.gitignore` and compute real SHA-256s.
- [x] **T2**: Update `crates/chatw-media-core/src/ai/registry.rs` with `yunet_face_det_2023mar` and real SHA-256 for `gfpgan-v1.4-identity-safe`.
- [x] **T3**: Implement `app/web/face_restorer.js` (lazy ORT + WebGPU `GFPGANv1.4` + `YuNet` multi-face NMS + Umeyama 5-point alignment + local skin-tone color lock + adaptive elliptical feathered mask blending + `CacheStorage` caching).
- [x] **T4**: Integrate Stage 2B Hybrid Face Restoration into `app/web/ai_restorer.js` (`v22`), `app/web/index.html`, `app/build-web.sh`, and `app/web/sw.js` (`chatw-static-v22`).
- [x] **T5**: Run `cargo test -p chatw-media-core` and `./app/build-web.sh`.
- [x] **T6**: Run `app/e2e/spec145-hybrid-face-e2e.mjs` on real 14 KB (`512×384`) and 33 KB (`1024×768`) AVIF portraits and visually inspect 400%-zoom PNG crops via `read_file`.
- [x] **T7**: Check off all tasks, `git add .`, commit, and `git push origin main` (Rule 1).
