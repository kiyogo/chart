# Plan 111: Thiết kế kỹ thuật Hyper-Deduplication Engine

## 1. Tham khảo Open Source (AGENTS.md Rule 3)
- **FastCDC v2020 + BLAKE3**: Tham chiếu kiến trúc `BlackPool25/packt` và `nlfiedler/fastcdc-rs` (Gear-Hash 256-entry bảng băm trượt + chuẩn hóa hai ngưỡng mask trước/sau trung bình giúp phân khối bất biến khi chèn/sửa byte đầu tệp).
- **Meta ThreatExchange PDQ / vPDQ + Multi-Index Hashing (MIH)**: Tham chiếu `facebook/ThreatExchange` (256-bit 2D-DCT + phân hoạch 16 băng 16-bit theo nguyên lý Dirichlet/Pigeonhole trên B-tree PostgreSQL giúp truy vấn Hamming O(1) thay vì quét tuyến tính O(N)).
- **Message-Locked Convergent Encryption + Proof-of-Ownership (MLE + PoW)**: Tham chiếu Tahoe-LAFS & Bellare–Keelveedhi–Ristenpart (EUROCRYPT 2013): khóa AEAD suy dẫn tất định từ `BLAKE3_derive_key(plaintext)` kết hợp bằng chứng sở hữu `pow_digest = BLAKE3(head || mid || tail || cipher_blake3)` để máy chủ hoàn toàn mù nội dung nhưng chống tấn công dò tồn tại bằng hash.

## 2. Thay đổi theo từng tầng

### 2.1. Core Library (`crates/chatw-media-core`)
- `src/dedup.rs`:
  - Mở rộng `PdqHash256` thêm `color_moment_hash: u64`, `aspect_ratio_milli: u16`, `mih_bands() -> [u16; 16]`.
  - Cập nhật `compare_perceptual_images` với Triple Guard (texture + caption band + color moment + aspect ratio) và hỗ trợ `effective_existing_pixels` để bản AVIF `ai_recovery` đã có `orig_width × orig_height` khớp cả bản gốc lẫn bản copy mờ hay bản 4× đã làm nét.
  - Thêm `compute_pow_digest(bytes) -> [u8; 32]`.
  - Thêm `fastcdc_chunk_bytes(bytes, min, avg, max) -> Vec<FastCdcChunk>` (Gear-Hash 2020 thuần Rust).
  - Thêm `AudioFingerprint` (`bits: [u8; 16]`, `duration_ms: u64`) + `compute_audio_fingerprint_pcm` + `audio_fingerprints_equivalent`.
  - Unit tests đầy đủ cho FastCDC shift-invariance, MIH 16-band pigeonhole recall, meme caption guard, blurry-AVIF-vs-original PDQ matching, audio fingerprint, và PoW digest.
- `src/e2ee.rs`:
  - Thêm `seal_cwch3_convergent(bytes, media_id, variant_id, chunk_size)` (suy dẫn tất định master key & salt bằng `blake3::derive_key`, 100% tương thích bộ giải mã `open_cwch3` hiện có).

### 2.2. Database Migration (`web/backend/migrations/0035_hyper_dedup_fastcdc_mih.sql`)
- Mở rộng `media_objects`: `orig_profile_json`, `orig_width`, `orig_height`, `color_moment_hash`, `aspect_ratio_milli`, `pow_digest`, `is_cdc_assembled`.
- Mở rộng `media_dedup_indexes`: `orig_blake3`, `color_moment_hash`, `aspect_ratio_milli`, `audio_fingerprint`, `orig_profile_json`, `orig_width`, `orig_height`, `ai_recommended`.
- Tạo bảng `media_pdq_mih_buckets (band_idx, band_val, scope_class, object_id)` + chỉ mục B-tree.
- Tạo bảng `media_cdc_chunks` và `media_cdc_recipes` hỗ trợ lưu trữ & lắp ghép khối FastCDC.

### 2.3. Backend Rust (`web/backend/src/storage/` & `attachments/mod.rs`)
- Tạo `storage/dedup_engine.rs`:
  - `insert_pdq_mih_buckets`, `find_pdq_mih_candidates` (truy vấn O(1) bằng `ANY` trên 16 băng MIH!).
  - `handle_dedup_claim` (`POST /api/storage/dedup/claim`): hỗ trợ 3 chế độ an toàn `from_ref_id`, `convergent_cipher_pow`, `shared_cas_or_perceptual` (và `user_authorized_perceptual`). Tạo `media_references` + `attachments`, tăng `ref_count`, trả về `download_url`, `ref_id`, `object_id`, `reused_object: true`, `dedup_match_kind`, `orig_profile_json`, `orig_width`, `orig_height`, `ai_recommended`.
  - `handle_cdc_plan` (`POST /api/storage/dedup/cdc/plan`) + `handle_cdc_chunk_put` (`PUT /api/storage/dedup/cdc/chunks/{blake3}`) + `handle_cdc_commit` (`POST /api/storage/dedup/cdc/commit`) cho khử trùng lặp từng khối FastCDC của tệp lớn.
  - `assemble_cdc_object` tích hợp vào `serve_media_reference` (hỗ trợ cả tải nguyên tệp lẫn HTTP `Range`!).
- Cập nhật `storage/router.rs` (`ingest_media_bytes`, `handle_forward_reference`) và `attachments/mod.rs` để đọc metadata dedup từ header, lưu `pow_digest` + `orig_profile_json` + MIH buckets, và luôn tạo bản ghi `attachments` khi forward/claim.
- Cập nhật `storage/references.rs` (`handle_dedup_check` dùng MIH O(1) + audio FP; `handle_storage_metrics` trả về thêm `cdc_unique_chunks`, `cdc_saved_bytes`, `mih_indexed_images`).

### 2.4. Client WASM & Dioxus UI (`app/`)
- `app/src/crypto/blob.rs`: thêm `seal_convergent` (tất định MLE cho DM E2EE, giải mã y hệt bằng `open` sẵn có).
- `app/src/storage/wasm_bridge.rs`: xuất `computeImagePdq`, `computePowProof`, `computeFastCdc`, `computeAudioFingerprint`, `comparePdqLocal` lên `window.__chatwWasmMedia`.
- `app/web/media_optimizer.js`:
  - Thêm `window.__chatwDedupLocal` (bộ nhớ + IndexedDB lưu dấu vân tay PDQ/BLAKE3 của mọi ảnh/video/tệp đã tải trong các đoạn chat để nhận diện tức thì ảnh copy/paste/tải-lại kể cả khi ảnh đang mờ chưa làm nét!).
  - Hook sự kiện `copy`/`contextmenu`/`dragstart` trên `.rich-img` / `.lightbox-img`.
  - Thêm trích xuất PDQ 256-bit (cho ảnh), vPDQ 4-keyframe (cho video), AudioFingerprint (cho âm thanh), và FastCDC/BLAKE3 (cho mọi tệp).
- `app/web/ai_restorer.js`:
  - Khóa cache AI theo `contentHash` (`ai:hash:${contentHash}:...`) khi có hash 32–64 ký tự để dùng chung kết quả làm nét 4K xuyên suốt mọi cuộc trò chuyện!
- `app/src/services/storage_sync.rs`: thêm `claim_dedup`, `cdc_plan`, `cdc_put_chunk`, `cdc_commit`, và gửi header dedup trong `upload_attachment_scoped`.
- `app/src/pages/chat.rs`, `app/src/platform/mod.rs`, `app/src/components/rich.rs`, `app/src/components/messages.rs`:
  - Hợp nhất luồng dán clipboard (`paste_watch` / `send_dm_image` / `send_group_image`) vào cùng pipeline tối ưu + khử trùng lặp.
  - Pre-flight claim trước khi tải lên ở cả DM, Group và Channel (`0 byte` upload khi trùng!).
  - Khi tái sử dụng ảnh mờ/nén từ server (`dedup_reused_ai`), không ghi đè ảnh mờ vào `sender_original` và bật làm nét Real-ESRGAN 4× cho **cả người gửi lẫn người nhận**!
