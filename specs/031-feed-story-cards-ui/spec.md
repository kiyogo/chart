# Feature Specification: Facebook-Style Feed Composer & Vertical Story Cards in "Mới" (Spec 031)

**Status**: Approved  
**Date**: 2026-11-21

## Overview
Upgrade the "Mới" (`FeedView`) page so its top layout matches a clean, modern social feed with:
1. A compact horizontal composer bar at the top (avatar + rounded pill prompt `"[Tên] ơi, bạn đang nghĩ gì thế?"` + 3 colorful real-action icons on the right) that expands smoothly into the full post composer when clicked.
2. A horizontal carousel of tall vertical Story cards directly underneath the composer bar:
   - First card: `"Tạo tin"` card with the user's avatar filling the top ~72%, pure white bottom footer, circular blue `+` button overlapping the seam, and `"Tạo tin"` label.
   - Subsequent cards: `"Tin của bạn"` (when user has an active story) and friends' vertical story/note cards featuring full-bleed media or tasteful gradient backgrounds, top-left circular avatar with accent ring & online green dot, and bottom-left bold white author name over a smooth dark scrim.
   - Floating circular white `>` / `<` carousel scroll buttons when overflowing.

## Acceptance Criteria
1. **Composer Bar (`.feed-compose-bar`)**:
   - Located at the very top of `.feed-inner` inside `FeedView`.
   - Displays user avatar (40px), rounded pill button (`"{name} ơi, bạn đang nghĩ gì thế?"`), and 3 quick-action icons (Create Story, Upload Photo, Share Thought) with no placeholder/mock behavior and no `...` text.
   - Clicking the pill or photo icon expands the inline post composer with textarea, image preview, visibility select, and `Đăng` button backed by `feed::create_post`.
2. **Vertical Story Cards Carousel (`.feed-story-carousel`)**:
   - Located immediately below `.feed-compose-bar` directly on the tonal canvas (`var(--bg-secondary)`), with no extra gray wrapper box.
   - First card is `"Tạo tin"` (`.feed-story-card.create-card`): top image/avatar section, white bottom footer, circular blue `+` badge (`var(--accent)`) straddling the boundary, and `"Tạo tin"` text. Clicking opens the `"Tạo tin ảnh"` modal.
   - Active user story renders as `"Tin của bạn"` card; friends' stories/notes/cards render with top-left circular avatar (`36px`) + accent ring + online status dot (when online) + bottom-left white name over dark gradient scrim.
   - Clicking a story card opens the Story Viewer modal (supporting reply or delete); clicking a note card opens the Note Viewer modal; clicking a friend without an active story/note opens chat.
   - Horizontal scroll without visible scrollbar, plus circular white `>` / `<` scroll navigation buttons when overflowing.
3. **Design System Compliance**:
   - All `.feed-card` elements remain borderless and shadowless (`border: none`, `box-shadow: none`).
   - Pure CSS, Flat Minimalist Design, full light/dark theme token support.
   - Messages sidebar (`Tin nhắn`) retains its existing circular `StoriesNotesBar` untouched.
