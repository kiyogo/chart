# Technical Plan 096: Fix Startup Skeleton Loading and Seamless Hydration

## 1. File Changes
1. **`app/src/pages/chat.rs`**:
   - Remove early `is_dialogs_loading.set(false)` call from line 614-616 upon reading `cached_dialogs`.
   - Update `load_dialogs_cache` parsing to also restore `privacy` and `bots` if present.
   - In the startup `spawn` block, concurrently fetch essential dialogs: `friends_list()`, `servers_list()`, `groups_list()`, `privacy_get()`, `bots_chats()`, and `friends_get_requests()`.
   - Save `privacy` and `bots` into `cached_dialogs` in `save_dialogs_cache`.
   - Add a brief async delay (350ms) using `gloo_timers::future::TimeoutFuture::new(350).await` or equivalent WASM timer so the skeleton shimmer runs smoothly and allows blob avatars to pre-warm in memory.
   - Set `is_dialogs_loading.set(false)` ONLY after the essential bootstrap bundle is populated and the minimum shimmer window has completed.
   - Pass `loading_initial: dm_loading_initial()`, `loading_more: dm_loading_more()` to `MessageList` in all message views (DMs, groups, channels).
   - In the right-side partner panel render block, render `SkeletonDetails` if `dm_loading_initial()` is active.
2. **`app/src/components/navigation/conversations.rs`**:
   - Ensure `SkeletonConversations` and `SkeletonStories` properly reflect `loading` prop without rendering partial rows.
3. **`app/e2e/spec096-startup-skeleton-loading.mjs`**:
   - Automated verification test checking:
     1. Page load/reload displays `.skeleton-shimmer`, `.skeleton-stories-wrap`, `.skeleton-conv-list`.
     2. Wait for loading to complete, verify conversation list has "Chính mình" at index 0, "Smoke Bot", and friend avatars.
     3. Verify zero CLS / zero flash of missing self-chat.
     4. Verify message skeleton loading when clicking fresh conversation.
     5. Capture screenshots.

## 2. Dependencies & Build
- Compile with `./app/build-web.sh release`.
- Test with Node.js Playwright runner against local daemon `http://127.0.0.1:8085`.
