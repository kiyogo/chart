# Technical Plan: Spec 031 — Feed Composer Bar & Vertical Story Cards

## Architecture & Files
1. **`app/src/components/feed_view.rs`**:
   - Replace the top `.feed-stories-card` + bulky `.feed-compose-card` with:
     - Compact `.feed-card.feed-compose-bar`:
       - Row 1: `Avatar (40px)` + `.feed-compose-pill` button (`"{display_name} ơi, bạn đang nghĩ gì thế?"`) + 3 quick icon buttons:
         - Story / Video-style coral icon (`#F3425F`) -> opens `Tạo tin ảnh` modal.
         - Photo gallery green icon (`#45BD62`) -> triggers `#feed-media-upload` & expands composer.
         - Thought / Moment rose-pink icon (`#E1306C`) -> opens `Chia sẻ suy nghĩ` modal.
       - Collapsible inline composer body (shown when `composer_open()` is true or `draft` / `media_url` is non-empty): textarea, media preview, visibility select, `Hủy`, `Đăng`.
     - `.feed-story-carousel` directly beneath `.feed-compose-bar`:
       - Scrollable `.feed-story-rail` (`id="feed-story-rail"`) with mouse-wheel & drag scrolling + automatic `>` / `<` button visibility via a small DOM helper.
       - Card 1: `.feed-story-card.create-card` (`Tạo tin`).
       - Card 2 (conditional): `.feed-story-card.story-card` (`Tin của bạn`) if user has a latest story in `posts()`.
       - Cards 3..N: Ordered friends (stories first, then notes, then remaining friends) rendered as `.feed-story-card.story-card` with media photo or gradient card, top-left `.feed-story-avatar-badge` (36px avatar + ring + green online dot), and bottom-left `.feed-story-name`.
       - Floating `.feed-story-nav-btn.prev` (`<`) and `.feed-story-nav-btn.next` (`>`) circular white buttons.
     - Modals inside `FeedView`:
       - Story Creator modal (`Tạo tin ảnh`) using `library::upload_media` + `feed::create_post`.
       - Thought Note modal (`Chia sẻ suy nghĩ`) using `api::update_my_profile` + `profile_ext::update_me`.
       - Story Viewer modal (image, caption, delete for own story, reply for friend's story).
       - Friend Note Viewer modal (full note + reply / open chat).
2. **`app/assets/main.css`**:
   - Add pure-CSS styling for `.feed-compose-bar`, `.feed-compose-row`, `.feed-compose-pill`, `.feed-compose-quick-btn` (with subtle colored icon tints), `.feed-story-carousel`, `.feed-story-rail`, `.feed-story-card`, `.feed-story-create-media`, `.feed-story-create-footer`, `.feed-story-create-plus`, `.feed-story-avatar-badge`, `.feed-story-online-dot`, `.feed-story-scrim`, `.feed-story-name`, `.feed-story-nav-btn`, and responsive mobile sizing.
3. **`app/e2e/spec030-ui-check.mjs`** (and verification):
   - Update FeedView checks to verify `.feed-compose-bar`, `.feed-story-carousel`, `.feed-story-card.create-card` (`Tạo tin`), clicking `"Tạo tin"` opens `"Tạo tin ảnh"` modal, and verify borderless/shadowless compliance.
