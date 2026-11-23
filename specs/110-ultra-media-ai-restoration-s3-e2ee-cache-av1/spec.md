# Spec 110: Ultra Media Compression, On-Device AI Restoration, Intelligent Cache, S3, E2EE, Smart Deduplication & AV1 Streaming

## 1. Bối cảnh & Kiểm toán hệ thống hiện tại
Qua kiểm tra trực tiếp mã nguồn kho lưu trữ `https://github.com/kiyogo/chart`, ChatW đã có nền tảng đáng kể:
- Workspace Rust gồm `web/backend` (Axum 0.8 + SQLx + PostgreSQL), `app` (Dioxus 0.7 cho Web WASM / Desktop / Mobile), và `crates/kiyo-kdata`.
- Đã có `web/backend/src/storage/` (`s3.rs`, `local_cas.rs`, `references.rs`, `router.rs`, `permissions.rs`, `migration.rs`) với ký AWS SigV4, presigned URL, RBAC theo scope, và ref-count cơ bản.
- Đã có `app/src/crypto/blob.rs` (`CWCH2` chunked AES-256-GCM) và `app/web/sw.js` (HTTP 206 Range streaming qua Service Worker).
- Đã có `app/web/storage_engine.js` (IndexedDB + OPFS + Cache API) và `app/src/storage/native.rs`.

### Các lỗ hổng & khoảng trống thực tế đã phát hiện:
1. **S3 / MinIO chưa được kết nối hoàn chỉnh vào luồng thực tế**:
   - `router.rs::ingest_media_bytes` chưa ghi vào `s3` dù `S3_ENABLED` sẵn sàng.
   - `router.rs::serve_media_reference` và `migration.rs::release_object_reference` **thiếu nhánh `"s3_ready"`**, dẫn tới lỗi 404 khi tải object đã migrate lên S3 và rò rỉ object khi xóa.
   - `s3.rs` thiếu S3 Multipart Upload thật (`POST ?uploads`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`), thiếu retry exponential backoff, và `get_object_stream` đang nạp toàn bộ file vào RAM thay vì stream.
2. **Nén media còn sơ khai & có fallback sai yêu cầu**:
   - `media_opt/mod.rs`, `media_optimizer.js`, `media_worker.js` ép cứng kích thước ảnh về `1920px` và âm thầm fallback sang WebP/JPEG/PNG (trên Chrome, `canvas.toBlob('image/avif')` trả về PNG/WebP thay vì AVIF thật!).
   - Chưa có bộ phân tích đa lớp ảnh (photo, portrait, text/UI, anime/manga, meme, alpha, noisy, HDR), chưa có vòng chọn rate-distortion đa ứng viên với đo chất lượng SSIM/MS-SSIM.
   - Chưa có mã hóa video **AV1** + đóng gói **Fragmented MP4 (CMAF)**, chưa có **FLAC** lossless & **Opus** thích ứng + khử nhiễu giọng nói nhẹ, chưa có nén tệp không mất dữ liệu đa thuật toán (**ZSTD / Brotli / LZMA2**) kèm chống decompression bomb.
   - Chuyển ảnh giữa JS và Rust đang đi qua Base64 gây tốn RAM/CPU.
3. **Chưa có trải nghiệm Instant Sender Original Preview & Auto-Cache người gửi**:
   - Người gửi phải chờ nén + mã hóa + upload xong mới thấy tin nhắn ảnh xuất hiện.
4. **Chưa có hệ thống AI Phục hồi ảnh/video chạy trên thiết bị (On-Device AI Restoration)**:
   - Chưa có Model Registry có kiểm duyệt giấy phép thương mại (chặn CodeFormer/RVRT/APISR/Upscayl trái phép MIT/thương mại; phê duyệt SPAN/Real-ESRGAN/Real-CUGAN/SwinIR/HAT/GFPGAN/NanoVSR/FlashVSR/BasicVSR++).
   - Chưa có Hardware Detector, Model Router, Tiled Overlapping Inference, Runtime ONNX (`tract-onnx`) + WebGPU WGSL Compute Shader + SPAB-Lite Neural Restoration, Quality Anti-Degradation Gate, và AI Output Cache.
5. **Khử trùng lặp (Deduplication) còn thiếu**:
   - Thiếu BLAKE3, thiếu perceptual hash (PDQ 256-bit cho ảnh/meme và vPDQ cho video) trong phạm vi non-E2EE an toàn; cần khóa chặt chính sách cấm global plaintext dedup trên dữ liệu E2EE riêng tư.
   - Cần chống race condition khi giải phóng `ref_count` và GC object.
6. **Local Cache chưa đủ thông minh**:
   - Đang dùng LRU đơn giản với hạn mức cố định `1024 MB` trên Web và không có eviction trên Native.
   - Thiếu **W-TinyLFU + Segmented LRU + Cost-Aware Eviction**, thiếu tính dung lượng thích ứng theo ổ đĩa trống, thiếu store riêng cho **AI Output Cache**, **Model Weight Cache**, và **Video Segment Cache**.

## 2. Mục tiêu nghiệm thu
Toàn bộ 30 tiêu chí nghiệm thu ở Phần XXXVIII của yêu cầu nâng cấp phải được thiết kế, triển khai thật bằng Rust + WebGPU/JS cầu nối mỏng, kiểm thử tự động (unit, integration, security, E2E Playwright) và đo benchmark thực tế với số liệu trung thực 100%, không giả lập và không phá vỡ dữ liệu hay giao diện cũ.

## 3. Ràng buộc bảo mật, giấy phép và trung thực
- **E2EE tuyệt đối**: Nén -> Mã hóa E2EE trên thiết bị -> Upload ciphertext lên server/S3. Server không bao giờ giải mã hay chạy AI lên dữ liệu E2EE. Không dùng global plaintext hash của nội dung E2EE để dedup xuyên người dùng.
- **Phân biệt rõ 3 loại dữ liệu**: `Original` (byte gốc chính xác), `Optimized` (AVIF/AV1/Opus nén tối ưu), và `AiEnhanced` (kết quả phục hồi cảm nhận trên thiết bị — không bao giờ gọi là ảnh gốc).
- **Giấy phép chặt chẽ**: Model Registry từ chối cứng các model/mã nguồn không tương thích thương mại/MIT (`CodeFormer` NTU S-Lab 1.0 NC, `RVRT` CC BY-NC 4.0, `APISR` GPL-3.0, `Upscayl` AGPL-3.0).
- **Trung thực kiểm thử**: Mọi con số benchmark phải đo từ thuật toán chạy thật trên máy này; mọi nền tảng chưa có phần cứng trực tiếp (Android/iOS vật lý, GPU phần cứng có AV1 encoder, MinIO live cluster) phải báo cáo trung thực mức độ đã xác minh (biên dịch / unit test / mock-S3 HTTP server nội bộ thật / trình duyệt Chromium thật) và giới hạn còn lại.
