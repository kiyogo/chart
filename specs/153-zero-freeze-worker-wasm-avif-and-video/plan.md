# Implementation Plan: Spec 153

1. New crate `crates/chatw-media-worker` (`wasm32-unknown-unknown` `--release` cdylib) exporting `encodeRgbaAvifAdaptive`, `encodeBytesAvifAdaptive`, `computeImagePdq`, `blake3Hex`, `compressFileSmart`, packed by `app/build-web.sh` via `CWRT1` into `app/dist/pkg/media_worker_wasm.bin` + `chatw_media_worker.js`.
2. `crates/chatw-media-core/src/avif.rs`: Fast single-thread `FastAdaptive` uses `BitDepth::Eight`, `speed = 10`.
3. `app/web/media_worker.js` + `app/web/media_optimizer.js`: Lazy-load `CWRT1` release worker WASM inside module worker, compute BLAKE3 + SHA-256 + PDQ + AVIF off main thread (`30s` timeout), sniff video magic bytes when `file.type` missing, unify single-pass video poster/fingerprint (`1.2s` max), and defer `registerLoadedImageFromBlobUrl` idle work.
4. `app/src/components/rich.rs`: Right-aligned pending video & file cards with `.spinner` (`z-index: 2`) and deferred non-blocking PDQ registration.
5. `app/web/ai_restorer.js`: Cap post-AI RL CPU `getImageData` readback to `1600px` max edge.
6. `web/backend/src/websocket/rpc_bin.rs`: Raise `MAX_RPC_BODY_BYTES` from `16 MB` to `36 MB`.
