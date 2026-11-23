# Báo cáo Benchmark Thực Tế — ChatW Ultra Media v2 (Spec 110)

> Mọi số liệu trong tài liệu này đều được đo trực tiếp từ lệnh chạy thật (`--release`) trên phần cứng thực tế — không nội suy, không làm tròn giả tạo.

## 1. Môi trường đo thực tế

- **Lệnh thực thi**: `cargo run --release -p chatw-media-core --features "native-fast,onnx-runtime" --example media_benchmark`
- **Hệ điều hành / Kiến trúc**: `linux_x86_64`
- **CPU**: Intel Xeon (`72` luồng logic, SIMD `x86_64_avx2_fma`)
- **RAM**: `94,196 MB` tổng (`17,325 MB` khả dụng tại thời điểm đo)
- **GPU**: `amdgpu (1002:6FDF)` (AMD Radeon RX 580 Polaris 20 XL — Vulkan / VA-API)
- **Kiểm tra bộ mã hóa AV1 phần cứng (`probe_av1_hardware`)**: `hw_enc=false` (RX 580 không có mạch AV1 hardware encode; hệ thống trung thực tự động chọn `rav1e_software_simd` thay vì báo giả phần cứng).

---

## 2. Kết quả Benchmark Ảnh — Adaptive Ultra AVIF + On-Device SPAB-Lite AI + PDQ

| Bộ dữ liệu kiểm thử | Phân lớp tự động (`ImageClass`) | PNG gốc | AVIF đầu ra (`ftypavif`) | Tỉ lệ nén | Cấu hình chọn | Thời gian nén AVIF | AI SPAB-Lite (Tiled CPU) | Băm cảm quan PDQ 256-bit |
| :--- | :--- | ---: | ---: | ---: | :--- | ---: | ---: | ---: |
| `Photo_1080p` (`1920×1080`) | `Photo` | `2,898,139 B` | `10,023 B` | **289.15×** | `Q=62`, `10-bit` | `1,836.0 ms` | `649.4 ms` (`applied=true`) | `6.7 ms` |
| `Photo_4K` (`3840×2160` — **không ép xuống 1920px**) | `Photo` | `11,570,717 B` | `30,159 B` | **383.66×** | `Q=62`, `10-bit` | `3,032.2 ms` | `2,292.9 ms` (`applied=true`) | `26.7 ms` |
| `CrispUI_1280x720` (`1280×720`) | `CrispTextUi` | `22,982 B` | `16,640 B` | **1.38×** | `Q=74`, `8-bit` | `554.1 ms` | `200.9 ms` (`applied=true`) | `3.1 ms` (`q=100`) |
| `Anime_1024x1024` (`1024×1024`) | `AnimeManga` | `32,755 B` | `11,238 B` | **2.91×** | `Q=68`, `10-bit` | `710.8 ms` | `226.0 ms` (`applied=true`) | `3.5 ms` |
| `Alpha_512x512` (`512×512` trong suốt) | `AlphaGraphic` | `9,188 B` | `8,568 B` | **1.07×** | `Q=68`, `8-bit` | `266.4 ms` | `94.5 ms` (`applied=true`) | `1.0 ms` |

> **Ghi chú kỹ thuật trung thực**: Trong bản build thuần Rust không liên kết thư viện C `libdav1d` (`avif-native`), crate `image 0.25` chỉ bật bộ mã hóa AVIF (`ravif`) mà không có bộ giải mã AVIF native trên CPU máy chủ; do đó `estimated_ssim` trong vòng chọn RD của `ravif` trả về `None` (chọn theo bảng chất lượng chuẩn hoá theo từng lớp ảnh), còn trên trình duyệt Web/Client thì Chromium/Firefox/Safari giải mã AVIF gốc bằng phần cứng/dav1d tích hợp sẵn và chạy **Anti-Degradation Quality Gate (SSIM + Color Drift)** trực tiếp trước/sau bước AI!

---

## 3. Kết quả Benchmark Video AV1 + CMAF fMP4 + E2EE `CWCH3`

| Bộ dữ liệu | Raw RGBA | fMP4 AV1 (`rav1e`) | Tỉ lệ nén | Số segment CMAF | Thời gian encode (`rav1e` SW) | E2EE `CWCH3` Seal | E2EE `CWCH3` Open |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `AV1_360p_24f` (`640×360`, 24fps, 24 khung) | `22,118,400 B` | `9,360 B` | **2,363.1×** | `2` | `8,279.9 ms` (`2.9 fps`) | `0.08 ms` | `0.02 ms` |

- `CWCH3` xác minh toàn vẹn từng chunk với nonce dẫn xuất HKDF-SHA-256 riêng biệt và AAD 27 byte; byte-for-byte đầu ra giải mã khớp 100% tệp fMP4 gốc.

---

## 4. Kết quả Benchmark Âm thanh FLAC Lossless (Bit-Exact)

| Bộ dữ liệu | Raw PCM 16-bit | FLAC đầu ra (`flacenc`) | Tỉ lệ nén | Encode | Decode (`claxon`) | Toàn vẹn bit-exact |
| :--- | ---: | ---: | ---: | ---: | ---: | :--- |
| `FLAC_1s_48kHz_Stereo` | `192,000 B` | `32,446 B` | **5.92×** | `10.36 ms` | `1.87 ms` | **100% (`true`)** |

---

## 5. Kết quả Benchmark Nén Tệp Không Mất Dữ Liệu (`CWCMP1` — ZSTD / Brotli / LZMA2)

| Kịch bản kiểm thử | Thuật toán tự chọn | Dung lượng gốc | Dung lượng sau nén | Tỉ lệ nén | Thời gian nén | Thời gian giải nén | Xác minh BLAKE3 |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | :--- |
| `JSON_Balanced` (1,000 bản ghi) | `zstd` | `110,794 B` | `2,228 B` | **49.73×** | `6.57 ms` | `0.20 ms` | **Khớp 100%** |
| `JSON_MaxRatio` (1,000 bản ghi) | `lzma2` | `110,794 B` | `1,367 B` | **81.05×** | `63.87 ms` | `0.87 ms` | **Khớp 100%** |
| `ZIP_AlreadyComp` (tệp đã nén) | `none` (bỏ qua thông minh) | `65,540 B` | `65,540 B` | **1.00×** | `0.11 ms` | `0.00 ms` | **Khớp 100%** |

- Tệp đã nén / entropy cao được phát hiện tức thì trong `0.11 ms` và giữ nguyên không làm phình dung lượng hay tốn CPU.

---

## 6. Kết quả Benchmark Bộ Nhớ Đệm Thông Minh (W-TinyLFU + Cost-Aware SLRU)

- **Kịch bản**: `500` thao tác hỗn hợp Zipf (20 mục nóng truy cập lặp lại + 200 mục quét 1 lần) dưới ngân sách cứng `32.0 MB`.
- **Kết quả đo**:
  - `hits = 307`, `misses = 193` -> **Tỉ lệ hit đạt `61.4%`** (chống nhiễu quét một lần tốt hơn rõ rệt so với LRU thuần).
  - `evicted = 161` mục lạnh, dung lượng cuối cùng tuân thủ chính xác **`32.0 MB`**.
  - Tổng thời gian xử lý toàn bộ 500 thao tác lập kế hoạch + cập nhật Count-Min Sketch: **`1.55 ms`**.

---

## 7. Tổng hợp kết quả kiểm thử tự động toàn Workspace

- `chatw-media-core` (with `native-fast,onnx-runtime`): **30 / 30 tests PASS** (gồm tải & chạy đồ thị ONNX thật qua `tract-onnx`, chặn giấy phép CodeFormer/RVRT/APISR/Upscayl, chống decompression bomb, chống gộp nhầm meme PDQ, `CWCH3` random-access seek).
- `chatw` (Backend Axum + PostgreSQL + S3): **27 / 27 tests PASS** (gồm real loopback HTTP S3 server kiểm thử PUT + transient 503 retry + Range 206 + Streaming + Multipart + SigV4 Presign + DELETE).
- `chatw-app` (Client Rust + Crypto + Native Cache): **41 / 41 tests PASS** (gồm `CWCH3` + `CWCH2` + v1 tương thích ngược).
- `kiyo-kdata`: **3 / 3 tests PASS**.
- **Tổng cộng**: **101 / 101 Rust unit tests PASS**, 0 lỗi.

---

## 8. Kết quả Kiểm thử E2E Thực Tế trên Trình Duyệt Chromium (Playwright)

- **Lệnh thực thi**: `./app/build-web.sh release && node app/e2e/spec110-ultra-media-ai-cache.mjs`
- **Bản đóng gói WASM Release (`CWRT1 core.bin`)**: `22,931,886 B` -> `5,106,242 B` (**4.49×**), biên dịch **0 cảnh báo**.
- **Nén ảnh AVIF thật trong trình duyệt (Pure-Rust WASM `ravif`)**: PNG `7,105 B` -> AVIF `5,887 B`, xác minh magic bytes `ftypavif`, `mime: "image/avif"`, `codec: "avif_wasm_ravif"` (tuyệt đối không fallback sang JPEG/WebP).
- **Nén tệp không mất dữ liệu trong trình duyệt (`CWCMP1`)**: JSON `15,765 B` -> `461 B` (**34.2×**, `brotli`), giải nén khớp byte-for-byte qua **BLAKE3**.
- **Phục hồi ảnh AI trên thiết bị & AI Output Cache (IndexedDB v5)**:
  - Lần chạy 1 (`compute`): `enhanced = true`, `cacheHit = false`, `backend = "cpu_tiled_spab_lite"`, `ssim = 1.0`, `elapsedMs = 144 ms`, `provenance = "ai_restored"`.
  - Lần chạy 2 (cùng phiên): `cacheHit = true`, `cacheTier = "memory"`.
  - Sau khi tải lại trang hoàn toàn (`page.reload()`): `cacheHit = true`, `cacheTier = "indexeddb"` (0 ms chạy lại AI).
  - Tự động phục hồi ngay trong khung chat `RichImage` sau reload: `complete = true`, `256×192`, `data-ai-restored = "true"`.
- **Kiểm duyệt giấy phép AI trong trình duyệt**: `codeformer`, `rvrt`, `apisr`, `upscayl` đều trả `false` (chặn cứng); `spab_lite_builtin`, `span_x2_s`, `realesrgan_x2plus`, `realcugan_2x`, `nanovsr_lite` đều trả `true`.
- **Kiểm thử bảo mật Backend trực tiếp (Alice & Bob)**:
  - `POST /api/storage/dedup/check` với `is_e2ee = true` -> trả `dedup_allowed = false` (`e2ee_strict_privacy_isolation_no_global_dedup`).
  - `POST /api/storage/dedup/check` trên nhóm non-E2EE -> trả `matched = true` (`exact_blake3`).
  - Upload ảnh nhóm -> tạo biến thể tối ưu `has_optimized = true` (AVIF).
  - Lưu trữ bền vững Server-First cho DM (`storage_tier = "server_durable"`, `e2ee_chunk_protocol = "CWCH3"`).
  - Kiểm tra ACL chéo tài khoản (Bob truy cập `/api/attachments/{id}` và `/api/storage/references/{id}/manifest` của Alice) -> chặn cứng **`403 Forbidden`** cả 2 endpoint.
