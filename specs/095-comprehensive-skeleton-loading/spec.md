# Spec 095: Comprehensive Shimmer Skeleton Loading System Across All Views

## 1. Context & User Request
- **User Request**:
  > "tonaf bộ hiện khung xương load khi đang load nhé làm cho tôi tất cả"
  > (Toàn bộ hiện khung xương load khi đang load nhé làm cho tôi tất cả / Show skeleton loading for everything when loading, do it for all of them for me)
- **User Attachment**:
  - `media_1795439697191_f5a20e87.jpg`: Shows the left navigation sidebar ("Tin nhắn"), stories/notes strip ("Tạo tin", stories), and conversation cards ("Chính mình", contacts, groups, bots).
- **Problem**:
  Currently, before data finishes loading from the server/cache:
  - The conversation list is blank or briefly flashes "No appropriate conversation".
  - The stories strip has a blank gap.
  - Opening a chat or fetching message history renders a small spinning wheel instead of placeholder skeleton bubbles.
  - Channels and friends lists have empty states while loading.
- **Goal**:
  Build a unified, polished, 60fps shimmer skeleton loading system across:
  1. **Conversation List & Sidebar**: Realistic card skeletons with circular avatar, title bar, subtitle preview bar, and unread badge placeholder.
  2. **Stories & Notes Strip**: Circular story avatar skeletons + subtitle name bars.
  3. **Chat Messages View**: Alternating staggered received & sent message bubble skeletons with shimmer animation (replacing the generic spinner).
  4. **Channels & Community List**: Channel row skeletons with hash icon + title bar.
  5. **Right Details Panel / Profile**: Hero banner, circular avatar, name, and quick-action button skeletons.
  6. **Friends List**: Friend card skeletons with action button placeholders.

## 2. Acceptance Criteria
- [x] **Shimmer CSS Engine**: `@keyframes skeleton-shimmer` with smooth linear gradient supporting light, dark, and translucent wallpaper themes with zero layout shifts.
- [x] **Conversation List Skeletons**: Displayed during initial boot / dialog loading and when filtered/switching, seamlessly replaced when data arrives.
- [x] **Stories Bar Skeletons**: Horizontal scroll strip of 5 story avatar skeletons while stories are loading.
- [x] **Message Stream Skeletons**: Staggered alternating left/right chat bubble skeletons when `loading_initial` is true in `MessageList`.
- [x] **Community & Channels Skeletons**: Channel list skeletons while channels are loading.
- [x] **Right Details Skeletons**: Profile hero + action skeletons while loading conversation/user details.
- [x] **Build & Verification**: Web WASM compiles cleanly via `./app/build-web.sh release`. Playwright automated test validates skeleton elements and captures visual screenshots.
- [x] **Auto-Push**: Code committed and pushed to `origin main` with commit hash reported.
