# Technical Plan — Spec 050

## Targeted Changes
1. **`app/src/pages/chat.rs`**:
   - Extend `dm_cache` entry from `(Vec<DisplayMessage>, bool)` to `(Vec<DisplayMessage>, bool, Option<i64>, Option<String>)` storing `(messages, has_more, partner_read_seq, chat_wallpaper)`.
   - On cached DM switch: restore messages, `dm_has_more`, `partner_read_seq`, and `chat_wallpaper` synchronously in one batch (never wipe to `None` before restoring!).
   - On uncached DM switch: fetch `api::messages_get`, `social::partner_read`, and `social::wallpaper_get` concurrently (`futures_util::join!` / parallel async) BEFORE clearing `dm_loading_initial`, so messages + `"Đã xem"` + wallpaper commit atomically in 1 single frame with zero secondary layout jump!
2. **`app/src/platform/mod.rs`**:
   - In `scroll_chat_to_bottom(instant)`: execute scroll immediately on microtask + first rAF (and skip unnecessary double-frame jumps when already at bottom).
3. **`app/assets/main.css`**:
   - **Zero-Morph Stability App-Wide**:
     - Remove `.message-row { animation: cw-msg-pop ... }` and `.cw-stagger, .settings-cols { animation: cw-rise-in ... }` and `.modal-dialog { animation: cw-modal-pop ... }` (keep pure smooth opacity-only fades or instant crisp geometry so ZERO bubbles, cards, or frames shrink/stretch/shift!).
     - Remove `scrollbar-gutter: stable;` from `.chat-messages`.
     - Lock `.chat-header` exact `height: var(--header-height); min-height: var(--header-height); max-height: var(--header-height); box-sizing: border-box;`.
     - Eliminate `.conv-row:hover / :focus-within .chat-item { padding-right: 2.25rem; }` text-squish bug: keep `.chat-item` `padding-right` constant and smoothly fade `.chat-item-time` / `.chat-item-badge` to `opacity: 0` when `.conv-more` reveals on hover/focus!
     - Remove `border-radius` and `transform` from global `button { transition: ... }` and eliminate `transition: all` on lines 2944, 4173, 4237.
     - Remove `transform: translateY(-2px)` hover jump on `.download-os-card`, `.guide-card`, `.cw-quickstart-card` so hovered cards never bounce/jiggle layout visually (keep crisp smooth hover highlight + spotlight!).
   - **Pure Crisp White Panels Across Feed, Meet & Entire Web**:
     - Change `.feed-column` (line 6059) and `.meet-column` (line 7100) from `background: var(--bg-secondary)` -> `background: var(--bg-primary)` (`#FFFFFF` in Light mode!).
     - Change `.details-panel, .partner-info-sidebar, .right-panel` (line 2082) from `linear-gradient(...)` -> `background: var(--bg-primary);` (pure `#FFFFFF` in Light mode, zero colored gradient wash!).
     - Make `.meet-room-item`, `.meet-created-box`, `.meet-hero-card`, `.meet-list-card`, `.feed-card`, `.feed-compose-bar`, `.feed-stories-card` flat `background: transparent` on top of the pure white outer frame; give `.feed-compose-pill` crisp pure white `var(--bg-primary)` + clean 1px `var(--input-border)` matching our Spec 046 inputs; align `.meet-mode-chip` with Spec 048 translucent hover/active (no grey fill, no text color change, no bolding).
     - Move/reinforce authoritative `html[data-panel-translucent="true"]` and `html[data-panel-glass="true"]` rules at the end of `app/assets/main.css` covering EVERY panel (`.feed-column`, `.meet-column`, `.vico-chat-column`, `.chat-column`, `.plugin-panel`, `.nav-sidebar`, `.settings-nav`, `.settings-body`, `.details-panel`, `.partner-info-sidebar`, `.server-members-panel`, `.notifications-page`, `.download-page`, `.search-page`, `.main-stage > .empty-state`) so transparency/blur never gets overridden by earlier page styles, while default (0% transparency) is strictly **pure white `#FFFFFF`** everywhere!
4. **Live Chromium E2E Test (`app/e2e/spec050-zero-morph-and-pure-white.mjs`)**:
   - Record frame-by-frame (every 16ms across 400ms!) bounding boxes + `transform` + `animationName` of `.chat-column`, `.chat-header`, `.message-row`, `.message-bubble`, `.chat-item` while hovering & clicking into a new chat -> assert **ZERO** `transform` scale/translate morph, **ZERO** `.chat-column` / `.chat-header` shift (`0px`), **ZERO** `.chat-item` `padding-right` jump on hover/click!
   - Open Feed (Screenshot 2), Meet (Screenshot 3), Messages + Details panel (Screenshot 1), Notifications, Download, Search, Settings -> assert EVERY outer panel has exact pure white `rgb(255, 255, 255)` background in default Light mode (zero `#F4F6F9` grey, zero gradient tint!) AND respects translucent opacity cleanly when transparency is turned on!
   - Inspect all screenshots visually with `read_file`.
