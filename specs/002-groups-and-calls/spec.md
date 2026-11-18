# Spec: Hệ thống Nhóm chat, Gọi thoại (Audio) và Gọi video (Video) thời gian thực

## 1. Mục tiêu
Bổ sung 3 hệ thống lớn vào ứng dụng ChatW:
1. **Nhóm chat (Group Chat) hoàn chỉnh**: Tạo nhóm, hồ sơ nhóm, phân quyền OWNER / ADMIN / MEMBER, quản lý thành viên, link mời, tin nhắn nhóm thời gian thực (hỗ trợ E2EE sender keys / envelope), quản lý tin nhắn (reply, pin, edit, delete).
2. **Gọi thoại (Audio Call)**: Gọi 1:1 và Gọi nhóm, kiểm tra quyền micro, truyền tải âm thanh thực tế qua WebRTC, mute/unmute, phát hiện người đang nói (active speaker), lịch sử cuộc gọi.
3. **Gọi video (Video Call)**: Gọi 1:1 và Gọi nhóm, kiểm tra quyền camera & micro, truyền tải hình ảnh và âm thanh thực tế, lưới video đa điểm (grid layout), bật/tắt camera, chuyển đổi thiết bị, chia sẻ màn hình.

## 2. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. **Database & Migrations**:
   - Migration `0007_groups_and_calls.sql` tạo đầy đủ các bảng `groups`, `group_members`, `group_invites`, `group_messages`, `call_sessions`, `call_participants` có khóa ngoại, ràng buộc và indexes thích hợp.
2. **Group Chat**:
   - Tạo nhóm với tên, avatar, chọn bạn bè tham gia.
   - Phân quyền: OWNER có toàn quyền (giải tán, chuyển quyền, admin), ADMIN (mời, kick, sửa), MEMBER (chat, gọi, xem thành viên).
   - Mời bằng username, chọn từ bạn bè, link mời mã ngẫu nhiên.
   - Tin nhắn nhóm realtime qua WebSocket, lưu trữ PostgreSQL, đồng bộ tức thì.
3. **Audio & Video Calling**:
   - Giao diện cuộc gọi: Incoming call, Outgoing call, Active call (1:1 & Group grid).
   - Truyền tải âm thanh/hình ảnh thật thông qua WebRTC (RtcPeerConnection, MediaStream, SDP offer/answer, ICE candidates).
   - Bật/tắt micro, bật/tắt camera, kết thúc cuộc gọi sạch sẽ giải phóng media tracks.
   - Active speaker detection dựa trên tín hiệu âm thanh.
4. **Kiểm thử Thực tế (Automated E2E)**:
   - Kiểm thử 2 tài khoản (Alice, Bob) cho 1:1 Audio & Video call.
   - Kiểm thử 3 tài khoản (Alice, Bob, Charlie) cho Nhóm chat, Group Audio call, Group Video call.
5. **Đa nền tảng & Build**:
   - Chạy `cargo check`, `cargo clippy`, `cargo test` pass 100%.
   - Build Web WASM và Linux Desktop release thành công.

