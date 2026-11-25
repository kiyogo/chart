# Spec 158: 15-Day Inactivity Cold Image Compression Algorithm

## Problem Statement
Images currently undergo lossy AVIF compression immediately upon upload, degrading fresh, actively viewed images. The user requested that images remain completely as-is (`vẫn như cũ`) while actively used, and only get compressed automatically if untouched/unused by users for **15 days**.

## Acceptance Criteria
1. **Pure-Rust Policy Core (`chatw-media-core`)**: Define `COLD_IMAGE_COMPRESSION_IDLE_DAYS = 15` (`15 * 86_400` secs) and `evaluate_cold_image_policy(...)` returning `KeepActive` while `now - last_accessed < 15 days` and `CompressCold` once `idle >= 15 days` (skipping already-AVIF, pinned, non-image, and E2EE ciphertext).
2. **Active / Fresh Upload Preservation**: Default uploads (`qualityMode != 'data_saver'` and `<= 24 MB`) keep original image bytes as-is (`original_passthrough`), without eager day-0 lossy compression.
3. **Access Tracking & Backend 15-Day Compression**: Track `last_accessed_at` on `media_objects` & `media_references` (throttled updates on reads in `serve_media_reference`), plus hourly `compress_inactive_images_15d` background migration converting non-E2EE server images idle `>= 15 days` into Adaptive AVIF (`avif_cold_15d`).
4. **Client-Side 15-Day Compactor (`storage_engine.js`)**: Automatically compact cached local images & `sender_orig:*` entries (including E2EE-decrypted local caches) in OPFS/IndexedDB after 15 days of zero access via `/media_worker.js`, leaving active (`< 15 days`) images untouched.
