# Feature Specification: 5-Tier Theme Modes, Vibrant Accent Palette & Inline Picker, Frameless Controls & Chat Header, Symmetrical List Avatar Spacing (Spec 053)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: In Progress

## 1. User Requirements (Images 1–6)

1. **Five Distinct, High-Legibility Theme Modes (`Chủ đề` — Image 1)**:
   - Support 5 selectable theme modes in `Appearance.theme` (`light`, `ash`, `dark`, `oled`, `system`):
     - **Sáng (`light`)**: Crisp pure white panels (`#FFFFFF`), `#E9EDF3` shell.
     - **Xám tro (`ash`)**: Comfortable mid-dark Discord Ash cool slate (`#313338` panels, `#23252A` shell, `#EBEDF0` ink).
     - **Tối (`dark`)**: Soft deep dark slate — noticeably darker than `Xám tro` yet softer than pitch black (`#1C1F26` panels, `#111318` shell, `#EDF0F5` ink).
     - **Đen huyền (`oled`)**: True OLED obsidian black (`#090A0E` panels, `#000000` shell, `#F3F5F8` ink).
     - **Theo hệ thống (`system`)**: Follows OS `prefers-color-scheme` live.
   - All 3 dark variants (`ash`, `dark`, `oled`) set `data-theme="dark"` + `data-theme-variant="ash|dark|oled"` (both in `boot.js` before WASM boots and in `platform::apply_appearance`) so all dark-mode components and `CONTRAST_ENGINE_JS` maintain clean, high-contrast legibility everywhere.
   - Each theme card swatch accurately previews its real surface tones.

2. **Frameless Color Picker Buttons (Image 2)**:
   - Remove all outer borders/frames from `<input type="color">` (`border: none !important; box-shadow: none !important;`) and `.cw-cp-swatch` so color buttons appear pure, flat, and borderless.

3. **Borderless Accent Swatches, Vibrant Fresh Default Palette, Slightly Richer Accent Tint & Inline Custom Color Picker Under `Màu nhấn` (Image 3)**:
   - Remove the thick black ring (`box-shadow`) around `.accent-swatch.active` completely (keep only the crisp centered white checkmark).
   - Upgrade all preset accent colors to vibrant, joyful, bright, high-legibility colors:
     - `blue`: `#2E7CF6` (bright Apple royal-sky blue)
     - `cyan`: `#06B6D4` (fresh crystal turquoise)
     - `violet`: `#8B5CF6` (vibrant radiant violet)
     - `rose`: `#F43F7E` (fresh sakura rose)
     - `orange`: `#F97316` (vibrant sunny tangerine)
     - `green`: `#10B981` (fresh vibrant emerald)
   - Raise `--accent-soft` slightly from `9%` -> **`12%`** (light), `10%` -> **`13%`** (luminous translucent light), and `15%` -> **`18%`** (dark) so active/selected accent pills look clearly colorful and fresh without feeling heavy.
   - Place the custom accent color picker row directly beneath the circular swatches inside the `Màu nhấn` group (removing the duplicated row from lower down in `AppearanceColorsShapes`) so users immediately see preset swatches + custom color customization together.

4. **Frameless Floating Chat Header Across All Chats (Images 4 & 5)**:
   - Remove all outer capsule frames, backgrounds, borders, and drop shadows from `.chat-header-identity`, `.chat-header-actions`, and `.back-btn` across DM, Self-Chat, Group, and Community Channel headers.
   - Keep `.chat-header` floating (`position: absolute; top: 0`) over full-height `.chat-messages` with the smooth, unblurred, wallpaper-matched top edge fade (`::before`, `height: calc(var(--header-height) + 0.5rem)`, `backdrop-filter: none`, `mask-image` smooth dissolve) so messages/images dissolve cleanly behind the frameless floating Avatar, Name, Channel Title, and Action Buttons.

5. **Symmetrical Avatar Spacing Inside `.chat-item` (Image 6)**:
   - Increase left padding of `.chat-item` (and `.create-community-btn`) so the avatar's left clearance equals its top and bottom clearance (`padding: 6px 8px 6px 7px` / balanced symmetrical `6px–7px` gap on top, bottom, and left — never hugged at `3px` against the left edge).
