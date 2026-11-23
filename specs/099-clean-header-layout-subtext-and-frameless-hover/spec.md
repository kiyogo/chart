# Spec 099: Clean Chat Header Layout, Tight Subtitle Spacing, Removed Device Count & Frameless Hover

## 1. Context & User Request
The user reported 4 distinct visual issues across 4 attached images:
1. `"xóa cái tìm như ảnh đi chứ"` (Image 1: `media_1795446407042_08af6917.png`):
   - Shows the "Tìm trong cuộc trò chuyện" / "Tìm tin nhắn đã tải" search input box inside the details panel.
   - Requirement: Delete the in-chat search section from the details panel.
2. `"và lỗi nó bị lỗi hiện ở bên trái"` (Image 2: `media_1795446409706_357123ef.jpg`):
   - Shows the toggle details button `[ | ]` and menu `...` sitting on the left side of the header rather than the far right.
   - Root Cause: In Spec 094 when `.header-inline-devices` (which had `margin-right: auto`) was removed, `.chat-header-identity.self-header-identity` had `margin-right: 0 !important;` and `.chat-header-actions` lacked `margin-left: auto;`, causing action buttons to stick to the left.
   - Requirement: Fix header layout so `.chat-header-actions` always anchors firmly to the far right (`margin-left: auto !important`).
3. `"và xóa cho tôi chấm 20 thiết bị đi và tát cả chữ ở dươi hiện gần tên hơn nữa"` (Image 2 & 3: `media_1795446449879_a1e0f35d.png`):
   - Header subtitle showed `· 20 thiết bị` / `· 1 thiết bị`.
   - Subtitle text had too much vertical distance from the title name.
   - Requirement:
     - Remove `· {count} thiết bị` completely from the "Chính mình" header, rendering clean `🛡 Chỉ mình bạn`.
     - Tighten spacing between title (`.chat-header-title`) and subtitle (`.chat-header-meta`): reset `margin: 0; line-height: 1.15;`, set `gap: 1px;`.
4. `"và xóa cho tôi tất cả khung khi di trỏ chuôt vô đi nhé"` (Image 2 & 4: `media_1795446454183_f29e6afb.png`):
   - When hovering the mouse cursor over the header identity button or action icon buttons, a rounded grey rectangular background box (`var(--bg-hover)`) appeared.
   - Requirement: Completely remove all hover boxes, background scrims, and borders on header hover (`background: transparent !important; box-shadow: none !important; border: none !important;`).

## 2. Acceptance Criteria
- [ ] In-chat search box ("Tìm trong cuộc trò chuyện" / "Tìm tin nhắn đã tải") removed from details panels.
- [ ] Chat header actions (`[ | ]` toggle, `...` more options) stay firmly on the far right on all screens.
- [ ] "Chính mình" header displays only `🛡 Chỉ mình bạn` without `· X thiết bị`.
- [ ] Subtitle text sits snugly directly beneath the contact/conversation name with zero excessive gap.
- [ ] Zero hover boxes or grey background pills when moving the cursor over the header identity and buttons.
- [ ] Release WebAssembly bundle builds cleanly and automated Playwright E2E tests pass.
