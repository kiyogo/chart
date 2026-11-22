# Spec 062: Full 33-Language i18n Runtime & Unified Accent-Synchronized Controls Polish

## Problem Statement

User feedback with three live screenshots identified four critical defects:

1. **Non-functional multi-language switching (`toàn bộ ngôn ngữ đều phải hoạt động được`)**:
   - The Language settings grid exposes 33 languages, yet `app/src/i18n.rs` only defines 31 core keys and is consumed by only 4 files. Selecting `English` (or any of the other 31 non-Vietnamese languages) leaves almost the entire UI (Settings navigation, Settings pages, Appearance labels, Chat controls, Sidebar, Modals, Auth, Secondary views) stuck in Vietnamese.
   - In the language grid, `Bahasa Indonesia` is truncated to `Bahasa Indone...` (violating AGENTS.md Rule #6: no `...` truncation).
2. **Inconsistent random white frosted boxes on buttons (`lỗi nó cứ bị hiện cái mờ trắng ở mấy cái nút, có bị có nút không — xóa đi chứ`)**:
   - `app/web/adaptive_surface.js` sets `data-local-scrim="true"` + `--local-scrim-bg` per row when local luminance variance exceeds a threshold, causing ugly faint white rectangles to appear unpredictably on some `.settings-nav-item` rows (`Đồng bộ`, `Sao lưu`, `Đám mây`, `Trò chuyện`, etc.) while adjacent rows remain transparent.
3. **Active states visually desynchronized from the vibrant `--accent` color (`giao diện nó bị hiện quá xấu, nó vẫn không đồng bộ với ở dưới mấy cái màu xanh á`)**:
   - Under busy/translucent wallpapers, active controls (`Giao diện` nav item, `Sáng` theme card, `Gọn` / `Tròn` / `Vừa` active segmented buttons, `Không` wallpaper tile) render with a washed-out milky pale tint (`makeCleanTint(..., 0.24, 0.76)`) that clashes with the vivid `--accent` swatches, checkboxes, range sliders, outgoing bubbles, and brand logo.
4. **Broken/ugly buttons & input fields (`nút, ... khung nhập, ...`)**:
   - `adaptive_surface.js` tags `.btn-secondary`, `.btn-ghost`, and `.segmented button` with `data-local-tone="dark"` whenever dark wallpaper pixels sit underneath, flipping their text & icons to `#FFFFFF` on top of bright light button backgrounds (making `"Tải ảnh của bạn"`, `"Tròn"`, `"Gọn"`, `"Vừa"`, `"Rộng"` unreadable white-on-white/pale!).
   - Individual idle segmented buttons get isolated floating white rectangles (`data-framed-controls="true"`), looking fragmented and mismatched.
   - Search and text inputs (`Tìm cài đặt`, `Tìm cuộc trò chuyện`, text/select/textarea, composer) lack cohesive rounded styling, crisp contrast over transparent wallpapers, and accent-matched focus states.

## Goals & Acceptance Criteria

### A. Complete 33-Language Internationalization
1. **All 33 Languages Work End-to-End**: `vi`, `en`, `zh-CN`, `zh-TW`, `ja`, `ko`, `th`, `id`, `ms`, `fil`, `km`, `lo`, `hi`, `bn`, `ar`, `fa`, `he`, `ur`, `tr`, `ru`, `uk`, `pl`, `cs`, `de`, `fr`, `es`, `pt-BR`, `it`, `nl`, `sv`, `ro`, `hu`, `el`.
2. **Instant Reactive Switching & Persistence**: Clicking any language immediately translates the entire visible UI (left rail tooltips/labels, Settings nav groups & all 22 tabs, Settings search, Appearance page & all controls/wallpapers/sliders, Messages/Friends/Communities sidebar, Chat header/composer/actions/date separators, Secondary views, Login screen), sets `html[lang]` and `html[dir]` (`rtl` for `ar`, `fa`, `he`, `ur`; `ltr` otherwise), and persists across reload via `localStorage` + user `Prefs`.
3. **Zero Bloat & Zero User-Content Corruption**: External lazy-loaded JSON packs (`/locales/{code}.json`) keep WASM slim while `i18n.rs` drives native reactive core keys; user-generated content (chat messages, usernames, server/group names, fingerprints, code blocks, input values) is NEVER altered.
4. **Zero Truncation in Language Selector**: All 33 language tiles display their full native name with zero `...` ellipsis (`Bahasa Indonesia`, `Português Brasil`, etc.).
5. **Multilingual Settings Search**: Typing either localized tab names or Vietnamese/English keywords in Settings Search finds matching tabs.

### B. Unified Accent-Synchronized Controls, Inputs & Zero Patchy White Scrims
1. **Zero Patchy Per-Row White Scrims**: Completely delete `data-local-scrim` and `--local-scrim-bg` from `adaptive_surface.js` and `main.css`. Every idle `.settings-nav-item`, `.chat-item`, `.channel-row`, `.rail-item` stays clean & uniform (zero random white boxes), while preserving real-time per-position `data-local-tone` text polarity on transparent idle text/rows!
2. **Zero White-on-White / Unreadable Button Text**: Remove filled controls (`.btn-secondary`, `.btn-ghost`, `.segmented button`) from `LOCAL_ROW_SELECTOR` and guard `[data-local-tone]` CSS so button text & SVG icons ALWAYS contrast strongly (`>= 4.5:1` WCAG AA) against the button's own background (e.g., `"Tải ảnh của bạn"`, `"Khôi phục giao diện mặc định"`, all segmented options).
3. **100% Accent Synchronization (`--accent` & `--on-accent`)**:
   - All primary active pills/tabs/buttons (`.settings-nav-item.active`, `.segmented button.active`, `.lang-tile.active`, `.filter-chip.active`, `.section-tab.active`, `.media-tab.active`, `.picker-tab.active`, `.rail-item.active:not(.has-logo)`, `.chat-item.selected:not(.has-nameplate)`, `.channel-row.selected`) render in vivid `var(--accent)` with `var(--on-accent)` text & icons — matching `Màu nhấn` swatches, sliders, checkboxes, outgoing bubbles, and brand logo across all 6 presets and custom picker colors.
   - `.theme-card.active` and `.wallpaper-tile.active` display a crisp `2px` `var(--accent)` inset ring with clean matched surface backing and high-contrast caption text.
   - Spec 060 guarantee strictly preserved: changing wallpaper NEVER overrides `--accent`.
4. **Unified Segmented Controls, Refined Buttons & Clean Inputs (`khung nhập`)**:
   - Delete fragmented `data-framed-controls="true"` per-button boxes; every `.segmented` group is ONE cohesive rounded pill track with clean transparent idle buttons (`0.00px` layout shift) and vivid `var(--accent)` active button.
   - Standardize `.btn-secondary`, `.btn-ghost`, `.btn-avatar-upload`, `.btn-small-secondary` with refined `10px` radius, hairline inset ring, and matched ink across opaque & translucent modes.
   - Upgrade all inputs (`input`, `textarea`, `select`, `.search-field input`, `.sidebar-search input`, `.composer-box`) with unified clean geometry, crisp light/dark translucent contrast over wallpapers, aligned search icons, and `var(--accent)` focus borders/rings.
