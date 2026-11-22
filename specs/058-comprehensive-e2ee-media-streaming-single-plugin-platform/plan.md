# Kế hoạch Kỹ thuật (Spec 058)

## 1. Cơ sở dữ liệu & Migrations (`web/backend/migrations/0032_e2ee_v2_device_keys_and_media_acl.sql`)
- Tạo bảng `identity_key_history`: lưu vết mọi lần đăng ký và xoay vòng khóa định danh (`user_id`, `device_id`, `ecdh_public`, `ecdsa_public`, `fingerprint`, `action`, `created_at`).
- Tạo bảng `device_prekeys`: lưu trữ `signed_prekey_pub`, `signed_prekey_sig`, `one_time_prekeys JSONB` theo từng thiết bị để hỗ trợ thiết lập phiên E2EE `v2`.
- Mở rộng `user_key_backups`: thêm các cột metadata KDF (`kdf_alg`, `kdf_mem_kib`, `kdf_time_cost`, `kdf_parallelism`, `root_key_wrapped_b64`).
- Mở rộng `messages`: thêm cột `device_envelopes JSONB` (nullable) cho phép đính kèm phong bì khóa theo thiết bị bên cạnh `recipient_envelope` và `sender_envelope`.

## 2. Backend (`web/backend/src/`)
- **`main.rs`**:
  - Thay `.nest_service("/uploads", ServeDir::new(&config.uploads_dir))` bằng `.nest_service("/uploads/avatars", ServeDir::new(config.uploads_dir.join("avatars")))` và tuyến xác thực `/uploads/media/{*file_path}` có kiểm tra quyền truy cập tệp.
  - Gỡ bỏ lời gọi tự động `plugins::marketplace::seed_official_catalog(&state.db).await` khi khởi động máy chủ (chỉ chạy khi đặt biến môi trường `CHATW_SEED_DEMO_PLUGINS=1` hoặc trong bài kiểm thử tích hợp).
- **`crypto/keys.rs`**:
  - Cập nhật `compute_fingerprint_v2(ecdh_pub, ecdsa_pub)` ràng buộc cả hai khóa công khai (giữ `compute_fingerprint_v1(ecdh_pub)` để đối chiếu tương thích ngược).
  - Ràng buộc `handle_register_keys` (`POST /api/keys`): nếu tài khoản đã có khóa (`device_keys`), yêu cầu phiên hiện tại phải thuộc thiết bị `trusted` (hoặc có cờ khôi phục hợp lệ), ghi nhật ký vào `identity_key_history` và `security_events`.
  - Hỗ trợ lưu và trả về metadata Argon2id (`kdf_alg`, `kdf_mem_kib`, `kdf_time_cost`, `kdf_parallelism`, `root_key_wrapped_b64`) tại `PUT /api/keys/backup` và `GET /api/keys/backup`.
  - Bổ sung endpoints `/api/keys/prekeys` và `/api/keys/{user_id}/bundle` cho phiên E2EE `v2`.
- **`devices_sync/mod.rs`**:
  - Yêu cầu `require_trusted_device` cho `DELETE /api/devices/{id}` và `PUT /api/keys/backup`.
  - Ẩn `verification_code` trong `GET /api/devices` đối với các phiên chưa được tin cậy (`trust_state != "trusted"`), trả kèm `ecdh_public` của thiết bị `pending` để thiết bị `trusted` bọc khóa `wrapped_sync_key`.
  - Bắt buộc đối chiếu `verification_code` trong `POST /api/devices/requests/{id}/approve`.
  - Vá `POST /api/devices/recover-lost`: chặn thiết bị `pending` tự nâng quyền khi tài khoản không bật 2FA trừ khi cung cấp bằng chứng xác thực/khôi phục hợp lệ.
  - Khi thu hồi thiết bị (`handle_revoke_device`), đóng ngay các kết nối WebSocket của phiên/thiết bị đó qua `WsHub`.
- **`messages/mod.rs`**:
  - Vá `PUT /api/messages/{id}`: kiểm tra đầy đủ `recipient_id` và `recipient_key_fingerprint`.
  - Vá `GET /api/messages/{id}/reactions`: kiểm tra người gọi là thành viên của cuộc trò chuyện (`sender_id` hoặc `receiver_id`).
- **`storage/router.rs` & `attachments/mod.rs`**:
  - Hỗ trợ `Range: bytes=start-end` (`206 Partial Content`, `Accept-Ranges: bytes`, `Content-Range`) cho truyền phát video/audio mượt mà.
- **`backup/engine.rs` & `backup/reauth.rs`**:
  - Sửa truy vấn khôi phục tin nhắn DM trong `engine.rs` để khôi phục cả tin nhắn gửi đi và nhận về với đúng phong bì khóa.

## 3. Frontend & Core Crypto (`app/src/`)
- **`app/Cargo.toml`**:
  - Thêm `argon2 = "0.5"`, `zeroize = { version = "1.8", features = ["derive"] }`, `hmac = "0.12"`, `subtle = "2.6"`.
- **`app/src/crypto/e2ee.rs` & `app/src/platform/native.rs`**:
  - Xóa hoàn toàn fallback ghi khóa plaintext khi mã hóa lỗi.
  - Cài đặt `Zeroize` và `Debug` an toàn cho `UserKeys`.
  - Cài đặt **Account Root Key** và hàm dẫn xuất khóa sao lưu bằng **Argon2id** (`derive_backup_key_argon2id`) kèm tương thích ngược với bản sao lưu SHA-256 cũ.
  - Cài đặt xác minh TOFU nghiêm ngặt theo tài khoản (`chatw_e2ee_peer_{my_uid}_{peer_id}`), phát hiện thay đổi khóa (`PeerKeyTrustStatus::Changed`), lưu lịch sử thay đổi khóa và hiển thị cảnh báo + mã an toàn.
  - Cài đặt giao thức mã hóa tin nhắn `PROTOCOL_VERSION = 2` (Chuỗi Ratchet KDF theo từng tin nhắn + Skipped-Key Window cho tin nhắn lệch thứ tự + Replay Protection) song song với bộ giải mã `PROTOCOL_VERSION = 1`.
- **`app/src/lib.rs` & `app/src/storage/sync_manager.rs`**:
  - Chặn tự động sinh khóa mới ghi đè lên máy chủ khi `me.has_keys == true` nhưng thiếu khóa cục bộ; chuyển sang trạng thái chờ khôi phục bằng mật khẩu E2EE hoặc phê duyệt từ thiết bị tin cậy (`wrapped_sync_key`).
  - Hoàn thiện `run_incremental_delta_sync` để xử lý và lưu tin nhắn đồng bộ delta.
- **`app/src/crypto/blob.rs` & `app/src/storage/recovery.rs`**:
  - Bổ sung định dạng mã hóa phân đoạn `seal_chunked` / `open_chunked` (hỗ trợ giải mã từng chunk cho streaming video/audio E2EE) tương thích hoàn toàn với `seal` / `open` truyền thống.
  - Bắt buộc mã hóa AEAD cho toàn bộ gói dữ liệu gửi qua `encrypted_relay` trong `recovery.rs`.
  - Mã hóa dữ liệu nhạy cảm lưu cục bộ (`chatw_previews_{uid}`, `chatw_reminders_{uid}`, media cache) và đảm bảo tải xuống tệp/video gốc sạch 100% không watermark.

## 4. Hợp nhất Nền tảng Plugin Duy Nhất (`app/src/components/plugin_*.rs`, `app/web/plugin_runtime.js`)
- Di trú dữ liệu người dùng từ `chatw_plugins_v2*`, `chatw_plugin_notes_v1*`, `chatw_plugin_todos_v1*`, `chatw_plugin_voxel_v1*` sang `chatw_legacy_plugin_archive_v1_{uid}` và cung cấp nút xuất JSON trên giao diện.
- Xóa toàn bộ 6 plugin tích hợp cứng và mô hình `PluginMod` / `PLUGINS` cũ.
- Hợp nhất `PluginsSettings` (`Cài đặt -> Tiện ích`), Rail trái, Composer, Slash Commands, Message Actions và Search về duy nhất `PluginHubPanel` + `PLUGIN_INSTALLATIONS` + `ChatwPluginHost`.
