# Tasks — Spec 163

- [x] T1. Upgrade AI sharpening strength (dual-scale 1px CAS + 2px micro-clarity) in `app/src/storage/realesrgan_gpu.rs` & `app/web/ai_restorer.js` and bump `PIPELINE_VERSION`.
- [x] T2. Eliminate image-send message jitter (`messages.rs` stable key, `rich.rs` seamless `PendingSenderMediaList`, `adaptive_surface.js` smooth/instant conflict fix, `chat.rs` instant same-tick handoff & uncompressed upload).
- [x] T3. Implement silent sender local-file -> uncompressed upload -> local cache -> silent predecoded swap (`media_optimizer.js`, `media_worker.js`, `core_bridge.js`, `platform/mod.rs`, `chat.rs`).
- [x] T4. Wire persistent receiver/sender local cache + server `X-ChatW-Cold-Compressed: 1` / `;cold15d=1` header + automatic AI sharpening on cache miss / new device (`router.rs`, `rpc_bin.rs`, `core_bridge.js`, `recovery.rs`, `rich.rs`).
- [x] T5. Harden cross-platform adaptive storage cache quota across Web, Android, iOS, Windows (`GetDiskFreeSpaceExW`), Linux, macOS + native `source_path` fallback (`native.rs`, `storage_engine.js`).
- [x] T6. Add unit/integration tests (`X-ChatW-Cold-Compressed` in `spec158` test), run full workspace verification (`cargo test`, `cargo check -p chatw-app`, `node --check`, `./app/build-web.sh`), restart daemon, commit and push to `origin main`.
