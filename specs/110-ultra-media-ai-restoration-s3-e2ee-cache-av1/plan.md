# Plan 110: Kiến trúc kỹ thuật hệ thống Ultra Media + AI Restoration + S3 + E2EE + Smart Cache + AV1 Streaming

## 1. Cấu trúc Cargo Workspace (Tái sử dụng & Không trùng lặp)
Thêm duy nhất 1 crate dùng chung `crates/chatw-media-core` vào `[workspace] members` trong `Cargo.toml` gốc. Cả `web/backend` và `app` (Native + WASM) đều phụ thuộc vào `chatw-media-core` để loại bỏ hoàn toàn trùng lặp logic codec, sniffer, dedup, E2EE chunking, cache policy và AI routing.

```text
crates/chatw-media-core/
  Cargo.toml               # features: default (portable), native-codecs (ravif/rav1e/zstd/flac/lzma2/brotli), onnx-runtime (tract-onnx)
  src/
    lib.rs                 # Public API, MediaError, MediaProvenance (Original | Optimized | AiEnhanced)
    classifier.rs          # Magic-byte sniffer, Shannon entropy, rich ImageClass (10 classes), adaptive resolution policy
    quality.rs             # Real PSNR, SSIM, MS-SSIM, Laplacian edge sharpness, color drift, Anti-Degradation Gate
    avif.rs                # Genuine Adaptive Ultra AVIF Encoder (ravif/rav1e) + multi-candidate RD loop + Animated AVIF (avis)
    av1.rs                 # Genuine AV1 Video Encoder (rav1e) + Motion/Scene analyzer + IVF & fMP4/CMAF muxer + ABR planner
    audio.rs               # Pure-Rust FLAC lossless (flacenc/claxon) + Opus adaptive planner/parser + Wiener speech denoiser
    files.rs               # Smart multi-codec lossless compressor (ZSTD/Brotli/LZMA2) + CWCMP1 envelope + Bomb Guard + BLAKE3 check
    e2ee.rs                # Unified CWCH3 Chunked Streaming AEAD + Authenticated Stream Manifest (100% compatible with CWCH2 & v1)
    dedup.rs               # BLAKE3 + SHA-256 + BSD-3 PDQ 256-bit DCT image hash + vPDQ video fingerprint + E2EE Privacy Scope Guard
    cache.rs               # W-TinyLFU (Count-Min Sketch) + Segmented LRU + Cost-Aware Eviction + Adaptive Capacity + AI Output Cache Key
    streaming.rs           # E2EE Streaming Manifest + ABR Selector (EWMA bandwidth + buffer) + Smart Prefetch Planner
    ai/
      mod.rs               # Unified On-Device AI Restoration Pipeline + Progress/Cancel + Provenance tagging
      hardware.rs          # Real Auto Hardware Detector (CPU cores, SIMD, RAM, GPU/Vulkan/VA-API, battery/thermal hints)
      registry.rs          # License-enforced Model Registry (SPAN, Real-ESRGAN, Real-CUGAN, SwinIR, HAT, GFPGAN, NanoVSR, FlashVSR, BasicVSR++; blocks CodeFormer/RVRT/APISR/Upscayl)
      router.rs            # Intelligent AI Model Router (image class + sharpness + compression + device tier + battery/RAM + text/QR safety)
      tiling.rs            # Overlapping Tiled Inference with Hann-feathered seamless blending & cancellation
      spab_lite.rs         # Pure-Rust SPAB-Lite Residual Attention CNN (inspired by Apache-2.0 SPAN) for zero-download restoration
      onnx_runtime.rs      # Real `tract-onnx` ONNX graph loader, SHA-256 verifier, tiled NCHW tensor execution (native)
      video_ai.rs          # Temporal-consistent multi-frame Video AI scheduler (motion-aware EMA blending, anti-flicker, weak-device bypass)
```

## 2. Database Migration (`web/backend/migrations/0034_ultra_media_v2.sql`)
Additive migration 100% tương thích ngược:
- Mở rộng `media_objects`: `blake3`, `perceptual_hash`, `compression_codec`, `compression_ratio`, `quality_ssim`, `media_class`, `ai_recommended`, `gc_eligible_at`, `is_pinned`.
- Tạo `media_chunk_manifests` và `media_chunks` cho AV1/Opus streaming E2EE + ABR variants.
- Tạo `media_dedup_indexes` (chỉ áp dụng `is_e2ee = FALSE` trong shared scopes; có unique/index cho `blake3` và `perceptual_hash`).
- Mở rộng `device_storage_policies`: `media_quality_mode`, `ai_enhancement_enabled`, `data_saver_mode`, `auto_cache_cleanup`, `server_durable_dm`.

## 3. Backend Storage & S3 Upgrades (`web/backend/src/storage/` & `media_opt/`)
- **`s3.rs`**:
  - Triển khai thật S3 Multipart Upload (`InitiateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`) ký AWS SigV4.
  - Bổ sung exponential backoff retry cho lỗi mạng/5xx.
  - Sửa `get_object_stream` sử dụng `Body::from_stream(resp.bytes_stream())` thật (không nạp hết vào RAM).
  - Kiểm thử thật với một HTTP S3-compatible test server chạy ngay trong unit/integration tests (xác minh PUT/GET/Range/Multipart/Presign/Delete/Retry thật 100% qua mạng loopback!).
- **`router.rs`, `migration.rs`, `references.rs`**:
  - Vá lỗi thiếu `"s3_ready"` ở `serve_media_reference`, `release_object_reference`, `cleanup_relays_safely`.
  - Khi `storage.s3.config().is_ready()`, tự động lưu trực tiếp lên S3 cho durable media (`Server-First`), không thì fallback an toàn sang `local_cas`.
  - Hỗ trợ `server_durable_dm` (mặc định bật — lưu ciphertext E2EE bền vững trên server/S3 mà không giải mã, đồng bộ đa thiết bị tức thì).
  - Chống race condition trong `release_object_reference` bằng transaction `FOR UPDATE` + kiểm tra `is_pinned` + grace period `gc_eligible_at`.
  - Bổ sung API:
    - `DELETE /api/storage/uploads/{id}` (hủy upload + dọn multipart S3/local).
    - `POST /api/storage/dedup/check` (tra cứu dedup theo phạm vi an toàn, chặn tuyệt đối plaintext dedup với `is_e2ee = true`).
    - `GET /api/storage/references/{id}/manifest` & `POST /api/storage/references/{id}/manifest` (E2EE streaming manifest + ABR).
    - `POST /api/storage/gc/incomplete` (dọn dẹp upload bỏ dở và object hết hạn không còn tham chiếu).
- **`media_opt/mod.rs`**: Tái sử dụng `chatw_media_core` thay cho bộ nén ảnh cũ bị giới hạn 1920px và fallback JPEG/WebP.

## 4. Client Pipeline (Web WASM + WebGPU + Native Desktop/Mobile)
- **Web (`app/web/` + `app/src/`)**:
  - `app/web/ai_restorer.js`: Trình chạy AI trên thiết bị cho trình duyệt — tự phát hiện **WebGPU** (`navigator.gpu`), biên dịch **WGSL Compute Shader** cho khối phục hồi ảnh SPAB-Lite theo tile có overlap blending, fallback sang OffscreenCanvas/CPU SIMD Worker nếu không có WebGPU, kèm **Quality Anti-Degradation Check**, **Text/QR Protection**, và **AI Output Cache** trên IndexedDB/OPFS.
  - `app/web/storage_engine.js` (DB v5): Bổ sung stores `ai_outputs`, `ai_models`, `video_segments`; thay LRU đơn giản bằng **W-TinyLFU + Segmented LRU + Cost-Aware Eviction** + tính hạn mức thích ứng theo `navigator.storage.estimate()`.
  - `app/web/media_optimizer.js` + `media_worker.js`:
    - Không còn ép cứng `1920px` và không âm thầm fallback JPEG/WebP cho ảnh tối ưu: kiểm tra AVIF thật, nếu trình duyệt không xuất được `ftypavif` thì gọi encoder **AVIF thuần Rust (`ravif`) qua WASM** hoặc gửi qua pipeline AVIF chuẩn!
    - Tạo ngay `instantPreviewUrl = URL.createObjectURL(file)` cho người gửi hiển thị tức thì bản gốc, không chờ nén/E2EE/upload; dùng binary transfer (giảm tối đa Base64).
    - Hỗ trợ WebCodecs **AV1** (`av01.0.04M.08`) & **Opus** VBR thích ứng (voice 28 kbps mono / music 72 kbps stereo).
    - Tự động lưu bản gốc của người gửi vào local cache gắn `ref_id` sau khi upload xác nhận thành công.
  - `app/web/sw.js`: Hỗ trợ thêm `CWCH3` chunked streaming decryption song song với `CWCH2`, kèm persistent segment cache vào IndexedDB `video_segments`.
  - `app/src/components/rich.rs` + `app/src/pages/chat.rs`:
    - Optimistic instant-original message bubble cho người gửi với trạng thái gửi rõ ràng (`Đang chuẩn bị`, `Đang nén`, `Đang mã hóa`, `Đang tải lên`, `Gửi thất bại` + nút Thử lại) không nhấp nháy.
    - Progressive display cho người nhận: kiểm tra AI Output Cache -> hiển thị ảnh AVIF giải mã ngay lập tức -> chạy AI phục hồi nền qua `window.__chatwAiRestorer` -> hoán đổi mượt không đổi layout -> lưu AI Output Cache!
  - `app/src/components/storage_ui.rs`: Bổ sung gọn gàng các tùy chọn tối giản (Bật/tắt AI nâng cao ảnh, Chất lượng media, Thống kê bộ nhớ đệm AI & phần cứng tăng tốc, Xóa cache AI) giữ đúng phong cách ChatW.
- **Native (`app/src/storage/native.rs`)**:
  - Tích hợp `chatw_media_core::cache::IntelligentCacheIndex` (W-TinyLFU + Cost-Aware Eviction + atomic writes + OS free-space adaptive quota) và `chatw_media_core::ai` (ONNX `tract-onnx` + SPAB-Lite) cho Desktop/Mobile.

## 5. Kiểm thử & Benchmark thực tế
1. Unit & integration tests trong `crates/chatw-media-core` (ảnh PNG/JPEG/WebP/AVIF/4K/anime/text/alpha/meme, AV1 video encode + fMP4, FLAC lossless bit-exact, ZSTD/Brotli/LZMA2 + decompression bomb, `CWCH3` E2EE + tamper/reorder/truncation, PDQ/vPDQ dedup + E2EE privacy guard, W-TinyLFU cache eviction + AI cache key, real `tract-onnx` ONNX graph inference + SPAB-Lite tiled inference + license gate rejecting CodeFormer/RVRT/APISR/Upscayl).
2. Backend integration & security tests trong `web/backend` (S3 HTTP server loopback thật test PUT/GET/Range/Multipart/Presign/Retry/Delete, ACL/RBAC cross-account rejection, E2EE dedup isolation, concurrent ref_count GC safety).
3. Benchmark runner thực tế (`cargo run --release -p chatw-media-core --example media_benchmark`) đo số liệu thật trên máy này và xuất báo cáo `docs/benchmark-media-v2.md`.
4. Full build (`cargo build`, `./app/build-web.sh release`) + Playwright E2E test thật `app/e2e/spec110-ultra-media-ai-cache.mjs` xác minh thực tế trên Chromium: gửi ảnh thấy gốc ngay lập tức -> nén AVIF thật -> mã hóa E2EE -> upload -> người nhận giải mã -> AI restoration -> AI Output Cache hit lần 2 -> Settings UI -> chụp ảnh màn hình kiểm chứng.
