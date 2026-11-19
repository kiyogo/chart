# Test Results — 2026-11-18

Môi trường: Linux, Chrome hệ thống (headless, Playwright 1.64.1), backend Axum debug trên `127.0.0.1:8085`,
PostgreSQL cục bộ, bundle WASM **release**. Tài khoản thật (alice_/bob_/charlie_ + mã chạy), không mock.

## Lệnh đã chạy
| Lệnh | Kết quả |
|---|---|
| `cargo build -p chatw-app --target wasm32-unknown-unknown --features web --release` | PASS |
| `cargo check -p chatw-app --features desktop` (bản native) | PASS |
| `cargo test --workspace` | PASS — 3 unit + 1 integration (`test_full_e2ee_chat_flow`) |
| `node app/e2e/ui-audit.mjs artifacts/screenshots/after` | **442/442 PASS** |

### Ghi chú về integration test
`test_full_e2ee_chat_flow` **đã lỗi từ trước** phiên này (kỳ vọng 1 kênh khi tạo server, nhưng commit 46e9e06 thêm
kênh thoại mặc định đúng theo spec 003). Test được cập nhật để kiểm tra đúng hành vi đã đặc tả: có kênh text
`general` **và** một kênh voice. Không nới điều kiện nào khác.

## UI audit — chi tiết theo nhóm
Mỗi viewport kiểm tra: không cuộn ngang; header & composer nằm trong viewport và không bị phần tử khác che
(`elementFromPoint`); danh sách tin ≥ 320px (hoặc panel là drawer có backdrop); panel thông tin không đè lên chat khi
inline; bubble ≤ 720px; không bubble nào tràn nội dung; font tin nhắn ≥ 14px.

Viewport: 320×568, 360×740, 375×667, 390×844, 430×932, 768×1024, 900×800, 1024×768, 1280×800, 1366×768,
1440×900, 1920×1080, 2560×1440, 3840×2160 — cả light và dark, có và không có panel thông tin.

| Nhóm | Pass | Fail |
|---|---|---|
| login light | 8 | 0 |
| login dark | 8 | 0 |
| friendship alice_636id | 2 | 0 |
| friendship bob_636id | 1 | 0 |
| create community | 1 | 0 |
| create group | 1 | 0 |
| E2EE DM | 1 | 0 |
| no decrypt | 1 | 0 |
| dm light | 188 | 0 |
| dm dark | 188 | 0 |
| settings light | 8 | 0 |
| settings dark | 8 | 0 |
| community opens | 1 | 0 |
| group light | 10 | 0 |
| emoji search | 2 | 0 |
| emoji picker | 2 | 0 |
| mobile list | 2 | 0 |
| group dark | 10 | 0 |

## Chức năng kiểm chứng end-to-end (qua UI thật)
| Luồng | Kết quả |
|---|---|
| Dev-login 3 tài khoản, kết bạn qua API thật | PASS |
| DM E2EE Alice ↔ Bob: tin ngắn, đoạn dài, URL dài, nhiều dòng (Shift+Enter), emoji | PASS, 0 lỗi giải mã |
| Tin nhắn tự gửi không bị nhân đôi | PASS (kiểm bằng ảnh) |
| Tạo cộng đồng, mở từ rail, hiển thị danh mục + kênh | PASS |
| Tạo nhóm 3 người, gửi tin nhóm qua composer mới | PASS |
| Emoji picker: mở, tìm "tim" có kết quả, nằm trong viewport | PASS |
| Settings: mở, chuyển mục Bảo mật / Giao diện, mobile list→detail→đóng | PASS |

## NOT TESTED
Truyền âm thanh/video WebRTC thật, kênh thoại có media, Google OAuth thật, upload avatar/banner/sticker qua UI mới,
Windows/macOS, Android/iOS, hiệu năng với hàng nghìn tin nhắn.

## Spec 007 (phiên 19/11/2026)
| Bộ kiểm thử | Kết quả |
|---|---|
| `cargo test --workspace` (backend + app: E2EE, blob AES-GCM, gợi ý composer, i18n 33 ngôn ngữ, nhãn thiết bị, kiểm tra giá trị hình nền) | PASS (16 unit + 1 integration) |
| WASM release build (`app/build-web.sh release`) | PASS, 7,9 MB |
| `cargo check -p chatw-app --features desktop` | PASS (liên kết bản desktop vẫn cần `libxdo3` trên máy) |
| API smoke spec 007 (quyền riêng tư, chặn, thiết bị, nhãn, ghim, hình nền, Chính mình, bot, Kinh doanh, nhóm, quản trị) | PASS |
| API smoke 2: vai trò cộng đồng (tạo, sửa, gán, gỡ, mời ra, @everyone, không cấp quyền mình không có), 9 bộ sticker | PASS 17/17 |
| UI audit (`node app/e2e/ui-audit.mjs --quick`, Chrome thật, backend thật) | PASS 276/276 |

Kiểm tra giao diện mới trong UI audit: Chính mình (thiết bị, ghi chú E2EE), gợi ý @ và Tab, gợi ý sticker theo nội dung,
dán ô Excel (hỏi cách gửi, bảng hiển thị ở người nhận qua E2EE), bình chọn và bỏ phiếu hai phía, ghim tin nhắn, "Đã xem",
ghim cuộc trò chuyện, chặn / bỏ chặn, hộp quản lý nhóm, liên kết mời nhóm, tìm kiếm toàn trang, đổi ngôn ngữ (Anh, Ả Rập
RTL), đặt mã PIN / khóa / sai PIN / mở khóa, 6 trang cài đặt mới.

Lỗi tìm thấy và đã sửa trong phiên: tin gửi cho "Chính mình" bị ràng buộc DB cũ chặn (migration 0014); trang Tìm kiếm
treo do hiệu ứng đọc và ghi cùng một tín hiệu; menu tài khoản ở cột trái bị danh sách hội thoại che (lớp xếp chồng CSS);
Esc không đóng hộp thoại mới (đưa tiêu điểm vào hộp thoại khi mở); trái tim trong sticker động bị CSS ghi đè vị trí;
xem trước "Chính mình" lộ mã thiết bị; ảnh đại diện trong hộp quản lý nhóm bị kéo dãn.

## Spec 008: OpenMoji stickers (2026-11-19)

- `cargo test --workspace`: all pass (new: `services::openmoji` parse, search and suggest tests).
- Importer: 4,495 files read from the verified archive; 2,015 kept (0 rejected by the SVG allow-list);
  2,035 skin-tone variants and 367 OpenMoji-only drawings skipped; 78 unnamed combinations skipped.
- Server: `catalog.json` served gzip-precompressed (226,775 → 67,363 bytes); SVGs served as `image/svg+xml`.
- UI audit (real Chrome, real server): 285/285 pass, including 9 new OpenMoji checks (tab, categories, credit,
  image load, Vietnamese search "mèo", send + receive in an E2EE DM, "pizza" suggestion).
- Found and fixed while testing: sticker pack tabs squashed into each other once there were 10+ packs;
  backend failed to start (`VersionMissing(14)`) because embedded migrations were stale (added `build.rs`).
