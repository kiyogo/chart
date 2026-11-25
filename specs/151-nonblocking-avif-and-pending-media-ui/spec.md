# Feature Specification: Fast Single-Pass WASM AVIF & Right-Aligned Visible Pending Media Status (Spec 151)

**Feature Branch**: `main`  
**Created**: 2026-11-25  
**Status**: In Progress

## Problem Statement

When sending photos in Chrome where `OffscreenCanvas.convertToBlob('image/avif')` is unsupported:
1. `wasm_bridge.rs` mapped `"fast"` to `AvifCompressionMode::UltraAdaptive` (3-candidate SSIM verification loop) and `avif.rs` allowed multi-candidate SSIM re-encoding up to 2.5 MP even with `threads: Some(1)`. On a `1600x1200` JPEG, main-thread `encodeAvifAdaptive` took **27,657ms (~28s)**, freezing the browser UI.
2. `media_optimizer.js` retried native canvas AVIF export repeatedly even after the worker detected lack of browser support, and did not yield a paint frame before synchronous WASM encoding.
3. `PendingSenderMediaList()` rendered with non-existent class `self` instead of `sent` (causing pending uploads to appear left-aligned), and `.rich-image-full` (`z-index: 1`) painted over the `.chip-privacy` status badge (`z-index: auto`), making `"Đang tối ưu"` / `"Đang gửi"` / `"Gửi thất bại"` completely invisible so users saw an apparently frozen image.
4. `send_dm_media`, `send_group_media`, and `send_channel_media` awaited IndexedDB persistence (`put_local_object` and `commit_sender_original`) before sending the encrypted message, while `send_dm_text` returned silently if keys were missing.

## Acceptance Criteria

1. Single-threaded WASM AVIF encoding on interactive photos completes cleanly in ~1–3s (no 27s multi-candidate UI freeze), while preserving `ftypavif` output and `origProfileJson` metadata.
2. `PendingSenderMediaList()` aligns on the right (`.message-row.sent`) with `z-index: 2` overlay badge + spinner clearly visible above the preview image.
3. Post-upload local IndexedDB persistence runs in background `spawn` so message dispatch happens immediately upon upload completion; `send_dm_text` surfaces clear toasts if keys/session are missing.
4. Benchmark and live Chrome DM upload verification confirm sub-3s AVIF encode and instant DM delivery with visible pending UI.
