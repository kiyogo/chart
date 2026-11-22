# Spec 058: Tái Cấu Trúc Toàn Diện Kiến Trúc E2EE, Đa Thiết Bị, Streaming Media & Hợp Nhất Một Nền Tảng Plugin Duy Nhất

## 1. Mục tiêu
Thực hiện nâng cấp toàn diện kiến trúc bảo mật đầu cuối (E2EE), quản lý khóa nhiều thiết bị, khóa bảo mật kho lưu trữ tệp (`/uploads`), truyền phát media (streaming video/audio) mã hóa theo phân đoạn (chunked AEAD) và hợp nhất tuyệt đối hệ thống Plugin của ChatW về **một nền tảng duy nhất**, loại bỏ hoàn toàn hệ thống 6 plugin tích hợp cũ (`PluginMod`) mà không làm mất bất kỳ dữ liệu người dùng nào.

## 2. Các vấn đề cốt lõi được giải quyết

### 2.1. Kiến trúc E2EE & Quản lý khóa (Sections III, IV, VI, VII)
1. **Loại bỏ hoàn toàn fallback ghi khóa plaintext**:
   - Xóa bỏ `.unwrap_or(json)` trong `app/src/crypto/e2ee.rs` và `.unwrap_or_else(|| value.to_string())` trong `app/src/platform/native.rs`.
   - Mọi thao tác bọc khóa thất bại phải trả lỗi tường minh (`Result::Err`), tuyệt đối không ghi khóa riêng ở dạng chưa mã hóa.
2. **Phân cấp khóa chuẩn & Argon2id Key Wrapping**:
   - Thiết lập hệ thống phân cấp: **Account Root Key** (256-bit CSPRNG) -> **Account Recovery / Backup Encryption Key** (bọc bằng **Argon2id** với random salt 16-byte, tham số KDF có phiên bản `argon2id-v1`, AEAD AES-256-GCM + AAD; thay thế vòng lặp 50.000 SHA-256 thủ công nhưng vẫn giữ nhánh giải mã tương thích ngược cho bản sao lưu cũ) -> **Device Identity Keys** (P-256 ECDH + ECDSA) -> **Per-Conversation Ratchet & Message Keys (`v2`)** -> **Attachment Keys**.
   - Bổ sung `Zeroize` cho các cấu trúc giữ khóa nhạy cảm (`UserKeys`, bộ đệm khóa tạm) và ẩn khóa riêng khỏi `Debug`.
   - Ràng buộc cả `ecdh_public` VÀ `ecdsa_public` vào hàm tính vân tay khóa (`compute_fingerprint`), kèm kiểm tra tương thích ngược cho vân tay `v1`.
3. **Chống ghi đè khóa định danh trái phép**:
   - Thiết bị mới hoặc phiên trình duyệt chưa có khóa cục bộ khi `me.has_keys == true` trên máy chủ **không được phép** tự sinh cặp khóa mới và gọi `POST /api/keys` ghi đè khóa định danh của tài khoản.
   - Máy chủ kiểm tra trạng thái tin cậy của thiết bị (`trust_state == 'trusted'`) trước khi cho phép xoay vòng/cập nhật khóa hoặc ghi đè bản sao lưu khóa, đồng thời lưu lịch sử vào `identity_key_history` và `security_events`.
4. **Xác minh khóa đối tác (TOFU & Phát hiện thay đổi khóa bất thường)**:
   - Lưu hồ sơ khóa công khai của đối tác theo phạm vi tài khoản `chatw_e2ee_peer_{my_uid}_{peer_id}`.
   - Đối chiếu vân tay và khóa ECDSA mới với khóa đã lưu; nếu thay đổi, phát cảnh báo bảo mật rõ ràng trên giao diện hội thoại, ghi nhận lịch sử thay đổi khóa và hỗ trợ đối chiếu mã an toàn / QR.
5. **Nâng cấp giao thức tin nhắn E2EE `v2` (Authenticated Per-Message Ratchet Chain + Replay/Out-of-Order Window)**:
   - Nâng cấp mã hóa tin nhắn 1-1 lên `PROTOCOL_VERSION = 2` sử dụng chuỗi KDF Ratchet tiến (Forward-Secure Chain Step) kết hợp ECDH, cửa sổ khóa bỏ qua (skipped-key window) cho tin nhắn đến lệch thứ tự và chống phát lại (replay detection), đồng thời **giữ nguyên bộ giải mã `PROTOCOL_VERSION = 1`** để đọc 100% tin nhắn lịch sử.
   - Vá lỗi `PUT /api/messages/{id}` (kiểm tra `recipient_id` và `recipient_key_fingerprint`) và `GET /api/messages/{id}/reactions` (chặn IDOR, kiểm tra quyền tham gia hội thoại).

### 2.2. Đa thiết bị, Phê duyệt & Đồng bộ lịch sử (Section V)
1. **Luồng phê duyệt thiết bị mới có truyền khóa E2EE (`wrapped_sync_key`)**:
   - Không tiết lộ `verification_code` cho chính thiết bị đang ở trạng thái `pending` qua `GET /api/devices`; yêu cầu mã xác minh hợp lệ khi thiết bị `trusted` phê duyệt.
   - Thiết bị `trusted` bọc gói khóa E2EE bằng ECDH tới `ecdh_public` của thiết bị mới thông qua `wrapped_sync_key`.
   - Chặn thiết bị `pending` gọi `DELETE /api/devices/{id}`, `PUT /api/keys/backup`, và vá lỗ hổng tự nâng quyền tại `POST /api/devices/recover-lost`.
   - Ngắt kết nối WebSocket tức thì trong `WsHub` khi thiết bị hoặc phiên bị thu hồi.
2. **Đồng bộ Delta & Khôi phục `.kdata`**:
   - Hoàn thiện `run_incremental_delta_sync` để giải mã và nạp tin nhắn mới vào bộ đệm cục bộ.
   - Vá lỗi khôi phục tin nhắn DM trong `web/backend/src/backup/engine.rs` (khôi phục cả chiều gửi và chiều nhận cùng phong bì khóa tương ứng).

### 2.3. Bảo mật Lưu trữ Cục bộ, Khóa `/uploads` & Streaming Media (Sections VIII, IX, X)
1. **Khóa truy cập tĩnh `/uploads`**:
   - Chỉ phục vụ công khai `/uploads/avatars` cho ảnh đại diện `.avif` (`AGENTS.md` Mục 4.1).
   - Toàn bộ `uploads/cas`, `uploads/relay`, `uploads/encrypted_backup` và `uploads/media` không còn mở công khai; truy cập media đi qua tuyến xác thực có kiểm tra quyền hội thoại.
2. **Vá lỗi rò rỉ plaintext trên `encrypted_relay`**:
   - Trong `app/src/storage/recovery.rs`, mọi tệp phản hồi qua `encrypted_relay` bắt buộc phải được mã hóa AEAD (`blob::seal` / `is_e2ee = true`) trước khi tải lên máy chủ.
3. **Mã hóa bộ nhớ đệm cục bộ & Phân tách tài khoản**:
   - Mã hóa dữ liệu nhạy cảm lưu cục bộ (`chatw_previews_{uid}`, `chatw_reminders_{uid}`, media cache trong OPFS/IndexedDB/Native CAS) và phân vùng theo `(account_id, device_id)`.
   - Native vault (`client_storage.json`) ghi nguyên tử với quyền tệp `0o600` ngay từ lúc tạo tệp.
4. **Truyền phát Video/Audio (Streaming) & Tải xuống sạch**:
   - Hỗ trợ HTTP Range (`206 Partial Content`) cho media không E2EE và phân đoạn AEAD (`blob::seal_chunked` / `open_chunked`) cho media E2EE.
   - Nút tải xuống tệp/video giải mã và trả về đúng tệp gốc, **tuyệt đối không chèn watermark** và không gắn metadata theo dõi.

### 2.4. Xóa Bỏ Hệ Thống Plugin Cũ & Hợp Nhất Một Nền Tảng Plugin Duy Nhất (Sections XII–XVII)
1. **Di trú an toàn dữ liệu người dùng từ tiện ích cũ**:
   - Tự động gom dữ liệu người dùng từng tạo trong `chatw_plugins_v2*`, `chatw_plugin_notes_v1*`, `chatw_plugin_todos_v1*`, `chatw_plugin_voxel_v1*` vào bản lưu trữ an toàn `chatw_legacy_plugin_archive_v1_{uid}` kèm nút **"Xuất dữ liệu tiện ích cũ (.json)"** trong trang Plugin hợp nhất.
2. **Xóa hoàn toàn hệ thống Plugin tích hợp thứ hai (`PluginMod`)**:
   - Xóa `default_builtin_plugins()`, `PLUGINS`, `PluginMod`, mục `"Tiện ích tích hợp"` và toàn bộ 6 plugin mẫu cứng (`Sổ tay & Việc làm`, `Soundboard 8-Bit`, `Công cụ Lệnh nhanh`, `Bảng vẽ Voxel`, `Sakura Kính mờ`, `Cyber Neon`).
   - Bỏ tự động seed danh mục plugin mẫu khi khởi động máy chủ production (`seed_official_catalog`).
3. **Một nền tảng Plugin duy nhất (`PluginHubPanel` + `ChatwPluginHost`)**:
   - Mọi điểm gắn kết trên giao diện (Rail trái, Thanh soạn thảo, Lệnh `/`, Hành động tin nhắn, Trang tùy chỉnh, Tìm kiếm, Cài đặt -> `Tiện ích`) đều vận hành thống nhất qua `PLUGIN_INSTALLATIONS` và sandbox `plugin_runtime.js` với kiểm soát 4 quyền E2EE (`e2ee.read_decrypted`, `e2ee.process_local`, `e2ee.egress_external`, `e2ee.store_external`).
