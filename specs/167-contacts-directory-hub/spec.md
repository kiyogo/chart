# Feature Specification: Spec 167 — Contacts Directory Hub ("Danh bạ")

## 1. Problem Statement
Currently, the navigation tab is labeled **Friends** (`Bạn bè`) and renders a single narrow sidebar stack mixing add-friend search, incoming friend requests, and friend rows while leaving the right-hand main stage unchanged. The user requires renaming **Bạn bè** → **Danh bạ** (**Contacts**) and upgrading it into a full two-pane directory matching the supplied reference layout while remaining 100% synchronized with ChatW's existing design system (Light, Ash, Dark, Obsidian themes, custom accent colors, density, glass/scrim mode, and flat minimalist aesthetics).

## 2. User Requirements & Acceptance Criteria
1. **Rename Friends → Contacts (`Danh bạ`)**:
   - Left navigation rail tooltip, mobile bottom bar tab label, and sidebar header display **Danh bạ** (in Vietnamese) / **Contacts** (in English) instead of **Bạn bè** / **Friends**.
2. **Left Contacts Sidebar Menu (matching Reference Image 1)**:
   - Top search bar (`Tìm kiếm` / `Search`) with quick action buttons to add a friend or create a group.
   - Four clean category items with vector SVG icons, active state pill synced with ChatW's `--accent-soft` / `--bg-hover` tokens, and live unread/pending badges:
     1. **Danh sách bạn bè** (`Friends list`)
     2. **Danh sách nhóm và cộng đồng** (`Groups and communities`)
     3. **Lời mời kết bạn** (`Friend requests`)
     4. **Lời mời vào nhóm và cộng đồng** (`Group and community invites`)
3. **Right-Hand Main Stage Directory Panel (matching Reference Image 2)**:
   - Selecting any of the 4 sidebar menu items immediately displays its dedicated full-width workspace in the right-hand `main.main-stage` pane (`< 3ms` instant `onmousedown` switching).
   - **Danh sách bạn bè**: Count header, live search filter, A–Z / Z–A sort, alphabetical letter grouping, friend rows with `DecoratedAvatar`, display name, `@username`, status, and quick actions (Message, Audio call, Video call, View profile, Remove friend, Block).
   - **Danh sách nhóm và cộng đồng**: Count header, live search filter, type filter (`Tất cả`, `Nhóm`, `Cộng đồng`), sort (A–Z, Z–A, Members), quick create group/community actions, click to open immediately.
   - **Lời mời kết bạn**:
     - Inline add-friend search & invite bar (find users by username, send request immediately).
     - **Lời mời đã nhận (N)**: Responsive 3-column card grid with avatar, name, formatted date (`dd/MM`), quick profile/chat icon button, and **Từ chối** / **Chấp nhận** buttons.
     - **Lời mời đã gửi (N)** (exact match to Reference Image 2): Responsive 3-column card grid with circular avatar, bold name, date (`dd/MM`), top-right message/profile icon button, and full-width **Thu hồi lời mời** button backed by real DB deletion.
   - **Lời mời vào nhóm và cộng đồng**:
     - Join-by-invite-link bar + **Lời mời đã nhận (N)** (accept/decline cards) + **Lời mời đã gửi (N)** (withdraw cards), backed by real PostgreSQL storage (`direct_target_invites`) and integrated with `InviteFriendsDialog`.
4. **100% Visual & Theme Synchronization**:
   - Uses exclusively ChatW's existing CSS custom properties (`--bg-app`, `--bg-surface`, `--bg-surface-hover`, `--bg-hover`, `--accent`, `--accent-soft`, `--border`, `--text-primary`, `--text-secondary`, `--text-muted`, `--radius-*`, `--space-*`).
   - Zero hardcoded light-only or dark-only colors, zero mock APIs, zero `"..."` placeholder strings, full mobile responsive navigation (back button from right pane to Contacts menu on small screens).
5. **Security & Performance Invariants**:
   - All invite accept/decline/withdraw/unfriend endpoints enforce strict session authentication and participant authorization (sender-only withdraw, receiver-only accept, member-only invite creation, block checks).
   - Sub-3ms category switching via isolated `ContactsTab` signal and `onmousedown` handlers.
