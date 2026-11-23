# Spec 096: Fix Startup Skeleton Loading and Seamless Hydration

## 1. Problem Statement & User Feedback
The user reported: `"lỗi nó vẫn chưa hiện load được"` (Error: it still doesn't show loading) with two comparison screenshots:
1. `media_1795441270168_593a62c9.png` (Image 1): An incomplete, unhydrated conversation list captured during initial page load:
   - "Chính mình" (Note to self) is missing.
   - "Smoke Bot" is missing.
   - User avatars are displayed as raw fallback initial letters ("1", "1", "N") instead of photos.
   - Anime background wallpaper has not applied.
   - **Crucially: No skeleton shimmer was shown at all**, exposing this raw, jarring, broken intermediate state to the user.
2. `media_1795441274091_71f7f020.jpg` (Image 2): The final loaded state where "Chính mình", "Smoke Bot", and avatar photos suddenly popped into view.

### Root Cause Analysis
1. In `app/src/pages/chat.rs` (lines 614-616), `is_dialogs_loading.set(false)` was fired prematurely upon reading `platform::load_dialogs_cache(&uid).await` (~5ms into page load).
2. At 5ms, `load_dialogs_cache` had only restored raw friends without blob avatars; `social::privacy_get().await` (which controls `self_chat`) and `social::bots_chats().await` had not been fetched yet.
3. Because `is_dialogs_loading` became `false` instantly, the skeleton loading disappeared immediately, exposing the half-loaded Image 1 state to the user before morphing jarringly into Image 2.
4. Furthermore, `loading_initial: dm_loading_initial()` was never passed into `MessageList` in `chat.rs`, and `SkeletonDetails` was not connected to the right sidebar.

## 2. Proposed Architecture & Solution
1. **Seamless Startup Skeleton Hydration**:
   - In `app/src/pages/chat.rs`, keep `is_dialogs_loading` `true` throughout the startup bootstrap.
   - Bundle the initial network fetch of `friends_list()`, `privacy_get()` (ensuring `self_chat` is resolved), `bots_chats()`, `servers_list()`, `groups_list()`, and `pending_requests()` before dismissing `is_dialogs_loading`.
   - Include a minimum display duration (350ms) during initial boot/refresh so users experience a silky smooth, intentional shimmer animation instead of a momentary flicker or raw initial state.
2. **Persistent Cache Expansion**:
   - Cache `privacy` (including `self_chat`) and `bots` in `platform::save_dialogs_cache` so that offline/subsequent reloads know `self_chat` and `bots` immediately.
3. **Wire Skeleton Loading in All Views**:
   - `MessageList`: Pass `loading_initial: dm_loading_initial()` and `loading_more: dm_loading_more()` for DMs, groups, and channels.
   - Details Panel: Render `SkeletonDetails` when `dm_loading_initial()` is active.
   - Left Sidebar: Keep `SkeletonStories` and `SkeletonConversations` active until bootstrap completes.

## 3. Acceptance Criteria
- [x] Initial page load and hard refresh (`Ctrl + Shift + R`) shows `SkeletonStories` and `SkeletonConversations` with 60fps shimmer for at least 350ms.
- [x] The broken/incomplete intermediate state (Image 1: missing self-chat, missing bot, raw letter initials) NEVER appears.
- [x] When the skeleton loading concludes, the list transitions directly and seamlessly to the complete, fully-populated conversation list (Image 2) with "Chính mình", "Smoke Bot", and resolved avatars.
- [x] Opening a chat that is loading messages shows `SkeletonMessages`.
- [x] Opening right details while loading shows `SkeletonDetails`.
- [x] WebAssembly release build compiles cleanly.
- [x] Automated Playwright E2E tests verify the skeleton loading behavior and capture screenshots.
