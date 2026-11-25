# Feature Specification: Zero-Freeze Worker WASM AVIF, Video Pending Preview & Receive Unblocking (Spec 153)

**Feature Branch**: `main`  
**Created**: 2026-11-25  
**Status**: In Progress

## Problem Statement

Even after lowering single-thread `ravif` speed to 9, synchronous WASM AVIF encoding (`encodeAvifAdaptive`) still blocked the browser's main UI thread for `~7.4s` on a `1600x1200` image (freezing cursor, hover, scrolling, and animations). Additionally:
1. Video uploads lacked right-aligned pending visual previews with active spinners, spent up to 5s loading metadata/seeking twice, missed magic-byte video detection when `file.type` was empty, and failed above `16 MB` due to `MAX_RPC_BODY_BYTES`.
2. Receiving images performed eager synchronous PDQ hashing on mount and unbounded `getImageData` post-RL readbacks up to `3840px`.

## Acceptance Criteria

1. Dedicated release-compiled `crates/chatw-media-worker` (`CWRT1`-packed `media_worker_wasm.bin`) runs AVIF encoding (both raw RGBA & full bytes), PDQ hashing, BLAKE3, and lossless file compression 100% inside `/media_worker.js` off the main UI thread (`0ms` main-thread encode block).
2. `FastAdaptive` single-thread AVIF uses 8-bit `speed = 10` for standard photos/UI to achieve sub-second background encoding while preserving genuine `ftypavif` output and `origProfileJson`.
3. Videos render a clean right-aligned pending preview with spinner, extract poster + fingerprint in one `<video>` pass (`<= 1.2s`), and CWBF2 supports up to `36 MB` RPC frames covering `25 MB` media attachments + E2EE overhead.
4. Main-thread UI responsiveness stays smooth during both sending and receiving, verified in headless Chrome + Playwright + unit tests.
