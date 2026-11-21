# Feature Specification — Spec 050: Zero-Morph Layout Stability App-Wide & Pure-White Frame Parity (Feed, Meet & All Panels)

## Context & Problem
1. **Frame & Bubble Morphing ("biến đổi một tí") When Clicking a New Chat & Across the Web (Screenshot 1)**:
   - Opening or switching to a chat triggers visible morphing/shifting of message bubbles and frames caused by:
     - `.message-row { animation: cw-msg-pop 190ms ... }` scaling every message row from `0.985` + `translateY(6px)` on mount.
     - `.cw-stagger` / `.settings-cols { animation: cw-rise-in ... }` shifting Settings/Cloud/Download grids vertically on every tab switch.
     - `.chat-messages { scrollbar-gutter: stable; }` reserving an inconsistent scrollbar gutter despite hidden native scrollbars.
     - `.chat-header { height: auto; }` fluctuating height when metadata loads/changes between chats.
     - `.conv-row:hover / :focus-within .chat-item { padding-right: 2.25rem; }` abruptly squishing sidebar chat text on hover/click and expanding back on blur.
     - Global `button { transition: ... border-radius ... transform ... }` and `transition: all` rules morphing element geometry across the web.
     - Multi-stage async updates in `select_friend` wiping `partner_read_seq` and `chat_wallpaper` on cached switches and inserting `"Đã xem"` / wallpaper after messages already painted.
2. **Grey / Colored Panel Backgrounds Instead of Pure Crisp White on Feed, Meet & App Panels (Screenshots 2 & 3)**:
   - `.feed-column` (Screenshot 2) and `.meet-column` (Screenshot 3) hardcode `background: var(--bg-secondary)` (`#F4F6F9` dull grey) AFTER the translucent panel rules, overriding both pure white `#FFFFFF` and panel transparency.
   - `.details-panel` / `.partner-info-sidebar` applies a colored vertical `linear-gradient(...)` tint instead of pure white.
   - `.meet-room-item`, `.meet-created-box`, `.feed-compose-pill` add dull grey fills on top of Meet/Feed.
   - `.search-page`, `.download-page`, `.notifications-page`, `.vico-chat-column` define `background:` after `data-panel-translucent`, breaking uniform panel rendering.

## Acceptance Criteria
1. **Zero Morph / Zero Layout Shift on Chat Switch & App-Wide**:
   - Clicking a new DM, Group, or Channel renders every `.message-row` and `.message-bubble` at exact 1:1 scale (`transform: none`, `animation: none`) with zero scale/translate morphing.
   - `.chat-header` stays at exact locked height (`var(--header-height)`), `.chat-messages` has no `scrollbar-gutter: stable`, and `.conv-row .chat-item` keeps constant `padding-right` on hover/focus/click (`.conv-more` cleanly replaces `.chat-item-time` in-place with zero text squish).
   - `select_friend` caches `(messages, has_more, partner_read_seq, wallpaper)` together and fetches uncached DM messages + read receipt + wallpaper in parallel before committing state so messages, `"Đã xem"`, and wallpaper appear atomically in a single frame.
   - Zero `cw-rise-in` vertical jump on Settings/Download/Cloud tabs; zero global `border-radius`/`all` geometry transitions on buttons/panels.
2. **100% Pure Crisp White Panels Across Feed, Meet & Entire Web (Light Mode Default) + Full Transparency Parity**:
   - `.feed-column`, `.meet-column`, `.chat-column`, `.nav-sidebar`, `.details-panel`, `.partner-info-sidebar`, `.server-members-panel`, `.settings-nav`, `.settings-body`, `.notifications-page`, `.download-page`, `.search-page`, `.vico-chat-column`, `.plugin-panel`, `.main-stage > .empty-state` ALL render pure white `rgb(255, 255, 255)` (`var(--bg-primary)`) in default Light mode (ZERO `#F4F6F9` grey tint, ZERO gradient color wash!).
   - Inner Feed/Meet containers (`.feed-card`, `.feed-compose-bar`, `.meet-card`, `.meet-hero-card`, `.meet-list-card`, `.meet-room-item`, `.meet-created-box`) are flat transparent on top of the pure white frame (zero grey box clutter).
   - Translucent / glass panel rules sit AFTER all page background declarations so `--panel-opacity` and `--panel-blur` work uniformly on Feed, Meet, Messages, Settings, Search, Download, Notifications, and Details.
3. **Zero Regressions**:
   - Spec 048 (14/14), Spec 049 (13/13), and new Spec 050 live Chromium E2E test ALL pass 100% with visual screenshot verification.
