# Spec 132: Theme-Adaptive Message Bubble Colors & High-Contrast Borders

## 1. Context & User Problem
- **User Request**:
  > "lỗi giao diện tin nhắn
  > hiện không đúng màu"
- **User Attachments**:
  - `media_1795540722178_5812ba77.png` (Screenshot 1): Dark theme chat where undecryptable / error message bubbles ("Không đọc được trên máy này") appear as stark, blinding pure white (`#FFFFFF !important`) pills with pitch-black (`#111827 !important`) text inside a dark container, clashing jarringly with the dark aesthetic.
  - `media_1795543322147_ca5e916e.jpg` (Screenshot 2): Light theme chat on a white container where message bubbles ("Không đọc được trên máy này") have pure white surfaces without borders, causing the bubble outlines to blend 100% into the white canvas and disappear completely, leaving floating text with zero visual boundary.
- **Root Cause Analysis**:
  1. `app/assets/main.css:1738`:
     `.message-row.received .message-bubble { background: #FFFFFF !important; color: var(--msg-recv-text, #111827); box-shadow: 0 1px 2px rgba(15, 23, 42, 0.05); }`
     The `!important` on `#FFFFFF` overrides all dark theme and dark wallpaper rules (`html[data-theme="dark"]`, `.chat-column.wp-night`, `.chat-column[data-chat-tone="dark"]`).
  2. `app/assets/main.css:1763-1782`:
     `.message-bubble.error-bubble, .message-row.sent .message-bubble.error-bubble, .message-row.received .message-bubble.error-bubble` hardcoded `background: #FFFFFF !important; color: #111827 !important;` and `* { color: #111827 !important; }`. This completely broke dark mode by forcing all error bubbles to blinding white with black text.
  3. Lack of border on light mode bubbles:
     On a white chat canvas (`--bg-surface: #FFFFFF` / `--bg-chat: #FFFFFF`), a white bubble with only `rgba(15, 23, 42, 0.05)` shadow lacks structural definition. It requires a clean `1px solid rgba(15, 23, 42, 0.09)` border and soft elevation so it is crisp and clearly demarcated.

---

## 2. Goals & Acceptance Criteria

### Goal 1: Dark Mode Adaptive Bubble Styling
- [ ] In Dark Mode (`html[data-theme="dark"]`, `html[data-surface-tone="dark"]`, `.chat-column.wp-night`, `.chat-column[data-chat-tone="dark"]`), received message bubbles render with dark surface `var(--msg-recv)` (e.g. `rgba(30, 39, 60, 0.90)` or `#282D38`), light text `var(--msg-recv-text)` (`#F1F4FA` / `#EDF0F5`), and subtle border `1px solid rgba(255, 255, 255, 0.08)`. Never stark white `#FFFFFF`.
- [ ] In Dark Mode, undecryptable/error bubbles (`.message-bubble.error-bubble`) render with dark surface `rgba(30, 39, 60, 0.90)` (or `var(--msg-recv)`), light text `var(--msg-recv-text, #F1F4FA)`, light neutral timestamp `var(--msg-recv-meta, rgba(241, 244, 250, 0.74))`, and subtle border `1px solid rgba(255, 255, 255, 0.12)`.
- [ ] Remove harsh `* { color: #111827 !important; }` so child text inherits proper theme colors.

### Goal 2: Light Mode High-Contrast Boundary & Crisp Definition
- [ ] In Light Mode on white canvas (`--bg-surface: #FFFFFF`), received message bubbles and error bubbles render with crisp `border: 1px solid rgba(15, 23, 42, 0.09)` and soft shadow `box-shadow: 0 1px 2px rgba(15, 23, 42, 0.05)`.
- [ ] Bubble outlines are clearly visible against pure white chat backgrounds, eliminating the "washed-out/floating text" bug in `media_1795543322147_ca5e916e.jpg`.

### Goal 3: Zero Red in Timestamps Preserved
- [ ] Timestamps on error bubbles remain neutral muted (`#64748B` in light, `rgba(241, 244, 250, 0.74)` / `#949EB0` in dark), never danger/red.

### Goal 4: Regression Prevention & E2E Validation
- [ ] Playwright E2E test verifies:
  1. Dark mode chat: received bubbles and error bubbles have dark background, light text, never white background.
  2. Light mode chat: received bubbles and error bubbles have visible 1px borders and distinct contrast against white backgrounds.
