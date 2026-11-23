# Plan 095: Comprehensive Shimmer Skeleton Loading System Across All Views

## 1. CSS Design (`app/assets/main.css`)
- Implement `@keyframes skeleton-shimmer` with background gradient sweep.
- Style `.skeleton-conversations`, `.skeleton-conv-row`, `.skeleton-conv-avatar`, `.skeleton-conv-title`, `.skeleton-conv-sub`.
- Style `.skeleton-notes-strip`, `.skeleton-story-item`, `.skeleton-story-avatar`, `.skeleton-story-name`.
- Style `.skeleton-messages-wrap`, `.skeleton-msg-row`, `.skeleton-msg-avatar`, `.skeleton-msg-bubble`.
- Style `.skeleton-channels`, `.skeleton-channel-row`.
- Style `.skeleton-details-wrap`, `.skeleton-details-banner`, `.skeleton-details-avatar`, `.skeleton-details-actions`.

## 2. Rust Component Architecture
- Create reusable component module `app/src/components/skeleton.rs`:
  - `SkeletonConversations`: Renders 6 rows of conversation card placeholders.
  - `SkeletonStories`: Renders 5 circular story placeholders.
  - `SkeletonMessages`: Renders 5 staggered alternating left/right chat bubble placeholders.
  - `SkeletonChannels`: Renders 5 channel list placeholders.
  - `SkeletonDetails`: Renders hero banner, avatar, title, and action placeholders.
- Register `skeleton` module in `app/src/components/mod.rs`.

## 3. Integration into Views
- **`app/src/components/navigation/conversations.rs`**:
  - Add `loading: bool` prop to `ConversationList` (default: false).
  - When `loading || (friends.is_empty() && groups.is_empty() && initial_loading)`:
    - Render `SkeletonStories` in the stories strip.
    - Render `SkeletonConversations` (6 rows) instead of the empty message.
- **`app/src/components/messages.rs`**:
  - In `MessageList`, when `loading_initial && messages.is_empty()`:
    - Replace `div { class: "chat-loading-initial", span { class: "spinner" } }` with `SkeletonMessages {}`.
- **`app/src/components/navigation/sidebar.rs`**:
  - When channels or servers are loading: render `SkeletonChannels`.
  - When friends are loading: render skeleton friends list.
- **`app/src/pages/chat.rs`**:
  - Add `is_dialogs_loading` signal (`true` initially, set to `false` once cached dialogs or fresh API data is hydrated).
  - Pass `loading: is_dialogs_loading()` to `ConversationList`.

## 4. Verification
- Compile release web bundle with `./app/build-web.sh release`.
- Write Playwright E2E test `app/e2e/spec095-comprehensive-skeleton-loading.mjs`:
  - Verify skeleton elements exist during loading states.
  - Assert shimmer CSS animation is active and computed.
  - Capture visual screenshots across all views: Conversation list, Stories strip, Message list, Details panel.
- Auto-push to `origin main`.
