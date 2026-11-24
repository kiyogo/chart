# Spec 111: Hyper-Deduplication Engine (4-Tier Pure Rust: BLAKE3 + FastCDC v2020 + Meta PDQ/MIH O(1) + vPDQ/Audio + Convergent MLE PoW + Cross-Chat Blurry-Copy AI Recovery)

## 1. Bối cảnh & Bài toán
Khi nhiều người dùng cùng gửi/tải một tệp, meme, ảnh, video hoặc âm thanh giống nhau, hoặc khi một người dùng copy ảnh/meme từ cuộc trò chuyện này (kể cả ảnh còn đang mờ chưa qua AI làm nét) sang cuộc trò chuyện khác:
- Hệ thống cũ chỉ khử trùng lặp hạn chế trong một số nhóm công khai bằng quét tuyến tính `LIMIT 120` sau khi đã tải trọn gói tệp lên máy chủ.
- Copy/paste ảnh từ khung chat (`Ctrl+V` hoặc tải về gửi lại) bỏ qua bước kiểm tra trùng lặp, tạo ra tệp vật lý mới hoàn toàn trên máy chủ và không tự làm nét lại cho người gửi nếu ảnh nguồn đang mờ.
- Chưa hỗ trợ khử trùng lặp ở mức khối nội dung (Content-Defined Chunking) cho tệp lớn/tài liệu/video chỉnh sửa một phần, chưa có dấu vân tay âm thanh, và chưa khử trùng lặp được các tệp giống hệt nhau giữa nhiều tài khoản DM E2EE một cách mù nội dung (zero-knowledge).

## 2. Mục tiêu & Tiêu chí nghiệm thu (Acceptance Criteria)
1. **Lưu trữ 1 bản vật lý duy nhất & cấp quyền theo ngữ cảnh**:
   - Nhiều tài khoản cùng gửi/tải một tệp/ảnh/meme/video/âm thanh giống nhau → máy chủ chỉ lưu **1 đối tượng vật lý duy nhất (`media_objects`)**, tăng `ref_count`, cấp `media_references` + `attachments` riêng với kiểm soát quyền đọc (`authorize_read`) chặt chẽ cho từng cuộc trò chuyện/tài khoản.
2. **4 tầng thuật toán khử trùng lặp thuần Rust (`crates/chatw-media-core/src/dedup.rs`)**:
   - **Tầng 1 (Exact CAS + Proof-of-Ownership)**: `BLAKE3` + `SHA-256` + bằng chứng sở hữu nội dung (`pow_digest`) trước khi tải lên (`0 byte` upload khi trùng).
   - **Tầng 2 (Sub-file FastCDC v2020)**: Thuật toán Gear-Hash Content-Defined Chunking (`min 16 KiB / avg 64 KiB / max 256 KiB`) lưu từng khối duy nhất tại `media_cdc_chunks` và ghép công thức tại `media_cdc_recipes` cho tệp lớn/PDF/ZIP/video chỉnh sửa nhẹ, phục vụ tải/Range streaming trong suốt.
   - **Tầng 3 (Ảnh & Meme — Meta PDQ 256-bit DCT + Multi-Index Hashing O(1) + Triple Guard)**:
     - Chia 256 bit PDQ thành 16 băng 16-bit (`media_pdq_mih_buckets`) để truy vấn ứng viên khoảng cách Hamming `O(1)` trên PostgreSQL.
     - Bảo vệ 3 lớp chống gộp nhầm meme: `quality >= 18` + `caption_band_hash` (64-bit dHash dải chữ trên/dưới, `dist <= 10`) + `color_moment_hash` (64-bit sắc độ không gian 4×4) + tỷ lệ khung hình (±8%).
     - Khớp chính xác cùng một ảnh/meme dù đổi định dạng (`PNG ↔ JPG ↔ WebP ↔ AVIF`), thay đổi độ phân giải, nén lại, copy qua clipboard, hay là bản AVIF mờ chưa làm nét vs bản 4× đã làm nét.
   - **Tầng 4 (Video vPDQ + Audio Spectral Fingerprint)**:
     - Video: đa khung hình chính (tối đa 8 keyframe PDQ) + ngưỡng thời lượng ±3% + chỉ mục MIH.
     - Âm thanh: dấu vân tay phổ năng lượng 128-bit (`AudioFingerprint`) + ngưỡng thời lượng ±3%.
3. **Copy ảnh giữa các đoạn chat (kể cả ảnh mờ chưa làm nét) → KHÔNG tạo tệp mới & tự làm nét cả người gửi lẫn người nhận**:
   - Copy/paste (Ctrl+C/V, Right-Click Copy Image, kéo-thả, hoặc lưu xuống rồi gửi lại) một ảnh từ Chat 1 sang Chat 2 (DM↔DM, DM↔Group, Group↔Group) sẽ nhận diện tức thì qua chỉ mục cục bộ/máy chủ và tái sử dụng đúng `object_id` hiện có trên máy chủ (`0` tệp vật lý mới, `0 byte` upload).
   - Nếu nguồn copy là ảnh mờ chưa làm nét (bản nén `ai_recovery` AVIF trên server), máy chủ trả về `orig_profile_json` gốc và kích hoạt Real-ESRGAN 4× (`588 ms` hoặc `< 5 ms` nếu trúng AI cache theo `contentHash`) làm nét cho **cả người gửi lẫn người nhận**.
4. **Bảo mật E2EE & Zero-Knowledge**:
   - DM E2EE sử dụng **Convergent Message-Locked AEAD (`BLAKE3` derive key/nonce) + Proof-of-Ownership (`pow_digest`)**: máy chủ hoàn toàn mù nội dung và không bao giờ biết khóa giải mã, vẫn lưu 1 bản mã hóa duy nhất giữa các tài khoản có cùng tệp; quyền truy cập `attachments/{ref_id}` cô lập tuyệt đối (`403 Forbidden` khi tài khoản ngoài gọi chéo).
   - Giữ nguyên tuyệt đối chuẩn bảo mật Spec 110 trên `POST /api/storage/dedup/check` (`is_e2ee: true` trả về `dedup_allowed: false`).
5. **Tương thích ngược & hiệu năng**:
   - Migration `0035_hyper_dedup_fastcdc_mih.sql` 100% additive.
   - Không làm giảm tốc độ gửi ảnh/tệp; trường hợp trùng lặp đạt tốc độ gửi gần như tức thời (`< 80 ms`).
