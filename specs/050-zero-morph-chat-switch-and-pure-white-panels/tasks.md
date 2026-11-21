# Implementation Tasks — Spec 050

- [x] **T001: Atomic Chat Switch & Full DM Cache (`app/src/pages/chat.rs`, `app/src/platform/mod.rs`)**
  - Cache `(messages, has_more, partner_read_seq, chat_wallpaper)` in `dm_cache` so cached DM switches restore everything in 1 frame without wiping `"Đã xem"` or wallpaper.
  - Fetch uncached DM messages, `partner_read`, and `wallpaper_get` concurrently before clearing `dm_loading_initial` so chat bubbles, `"Đã xem"`, and wallpaper paint together with zero secondary jump.
  - Make `scroll_chat_to_bottom(true)` snap synchronously in microtask + rAF to eliminate top-to-bottom jump flash.

- [x] **T002: Eliminate All Morphing Animations, Geometry Transitions & Hover Squish App-Wide (`app/assets/main.css`)**
  - Remove `cw-msg-pop` from `.message-row` and `cw-rise-in` from `.cw-stagger` / `.settings-cols` and `cw-modal-pop` scale jump.
  - Remove `scrollbar-gutter: stable` from `.chat-messages`; lock `.chat-header` exact `height: var(--header-height)`.
  - Fix `.conv-row` hover/focus `padding-right` jump (keep constant padding, fade `.chat-item-time` cleanly behind `.conv-more`).
  - Strip `border-radius` & `transform` from global `button` transition and eliminate `transition: all` geometry morphing.

- [x] **T003: Pure Crisp White Frames on Feed, Meet, Details & All App Panels (`app/assets/main.css`)**
  - Switch `.feed-column`, `.meet-column`, `.details-panel`, `.partner-info-sidebar`, `.right-panel`, `.server-members-panel`, `.notifications-page`, `.download-page`, `.search-page`, `.vico-chat-column` to pure white `var(--bg-primary)` (`#FFFFFF` in Light mode — zero `#F4F6F9` grey, zero colored gradient!).
  - Flatten grey boxes inside Meet & Feed (`.meet-room-item`, `.meet-created-box`, `.meet-mode-chip`, `.feed-compose-pill`).
  - Reinforce translucent/glass panel rules at the end of `main.css` so transparency parity works across ALL panels without ever being overridden.

- [x] **T004: Build Web Release, Run Spec 050 + 049 + 048 E2E Tests, Visual Audit, Build Desktop + `.deb`, Commit & Push**
  - Compile & build Web release, run `spec050`, `spec049`, `spec048`, inspect every screenshot with `read_file`, build Linux Desktop release + `.deb`, commit, and push to `origin/main`.
