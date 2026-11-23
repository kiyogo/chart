# Spec 103: Floating Sticky Dynamic Date Pill & White Bubble Contrast Fix

## 1. Problem Statement & User Request
- **Issue 1**: The date separator ("ngày, tháng, năm") in the chat window was no longer floating/sticking dynamically at the top when scrolling through messages ("lỗi ngày ,tháng ,năm hiên tùy theo nó không còn hiện ở trên nữa khi mình kéo xuongs").
- **Issue 2**: Error message bubbles (`.error-bubble` displaying "Không đọc được trên máy này") rendered with pure white backgrounds (`#FFFFFF`) and white text (`var(--text-primary)` in dark mode = `#F4F6FA`), making the text invisible and unreadable.
- **Issue 3**: The scroll-to-bottom button (`.scroll-to-bottom-btn`) overlapped the latest message (`k 22:29`) due to an excessive negative top margin (`margin-top: -46px`).

## 2. Acceptance Criteria
- [x] Date separators stick dynamically at `top: 8px` within each day's group container (`.date-group`), smoothly scrolling off when scrolling into the next day.
- [x] Error message bubbles with white backgrounds have high-contrast solid dark text (`#111827`) and slate metadata (`#64748B`), with >= 4.5:1 WCAG contrast ratio.
- [x] The scroll-to-bottom button floats cleanly above the bottom composer bar without obscuring the latest message text or timestamps.
- [x] Automated E2E verification passes all checks with visual screenshot verification.
