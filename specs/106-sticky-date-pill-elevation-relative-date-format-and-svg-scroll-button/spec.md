# Spec 106: Sticky Date Pill Elevation, Relative Date Formatting (Hôm nay/Hôm qua/Năm) & Modern SVG Scroll Button

## 1. Problem Statement & User Request
From the user request:
> "nó phải hiện gần lên phía trên ơn chứ
> và hiện đúng Hôm nay ,Hôm qua nhé và nếu năm ngoái hoặc lâu hơi thì hiện thành có năm ở dưới như 12 tháng 05 năm 2025
> và 
> và mũi tên xuống dưới hiện đẹp hơn vẽ bằng svg nhé và hiện gần ở dưới hơn nữa đi"

### User Needs:
1. **Elevate sticky date separator closer to the top ("nó phải hiện gần lên phía trên hơn chứ")**:
   - In Spec 103, `.date-separator` had `top: 8px !important;` inside `.chat-messages` which has `padding-top: calc(var(--header-height) + var(--space-5))` (~76px) plus `margin-top: 16px`. This made the date pill float ~92-100px below the window top edge, leaving an awkward empty gap above it as shown in `media_1795451060572_0dbb2b4f.png` and `media_1795451061630_9e77f446.png`.
   - The date pill should stick elevated close to the top edge (offsetting padding-top so it sits sleekly near the top at ~12px) without excessive empty space above it.

2. **Accurate Date Formats ("Hôm nay", "Hôm qua", and Year for older messages)**:
   - For messages sent today: display `"Hôm nay"` (or `"Today"` in English / localized equivalent).
   - For messages sent yesterday: display `"Hôm qua"` (or `"Yesterday"` in English / localized equivalent).
   - For messages in current year: display `"{day} tháng {mon}"` (e.g., `23 tháng 11`).
   - For messages in previous years or older ("nếu năm ngoái hoặc lâu hơn thì hiện thành có năm ở dưới như 12 tháng 05 năm 2025"): display `"{dd} tháng {mm} năm {yyyy}"` (e.g., `12 tháng 05 năm 2025`).

3. **Modern SVG Scroll-to-Bottom Button ("mũi tên xuống dưới hiện đẹp hơn vẽ bằng svg nhé và hiện gần ở dưới hơn nữa đi")**:
   - Replace the legacy plain-text unicode arrow `"↓"` with a crisp, modern SVG icon (smooth arrow down with rounded stroke linecap and linejoin).
   - Position the button closer down towards the bottom (`bottom` adjusted closer to the composer bar instead of floating 60px high up over messages as shown in `media_1795451134550_ae69c45e.jpg`).

## 2. Acceptance Criteria
- [x] Date separator sticks closer to the top with reduced top margin and offset adjustment.
- [x] Today's messages show "Hôm nay" (in vi) / "Today" (in en).
- [x] Yesterday's messages show "Hôm qua" (in vi) / "Yesterday" (in en).
- [x] Older messages from past years show "{day} tháng {mon} năm {year}" (e.g., "12 tháng 05 năm 2025").
- [x] Scroll-to-bottom button renders a clean SVG arrow icon.
- [x] Scroll-to-bottom button sits closer to the bottom composer bar without floating awkwardly over messages.
- [x] Playwright E2E verification passes all checks with screenshot proof.
