# Spec 021: Floating Composer, Invisible Resizer Hover, Feed Icon Redesign & Simplified Settings Navigation

## 1. Context & User Requests
Based on user visual review of the latest screenshots:
1. **Floating Composer without Bottom Bar Frame**: Remove the lower frame/hint box below the chat input bar (`.composer-hint`) and make the composer float cleanly above the message stream with zero opaque bottom panel (`position: absolute; bottom: 0; background: transparent`), adding scroll bottom padding so messages scroll smoothly behind and above the pill input.
2. **Invisible Panel Resizer Guides**: Remove the bright blue hover/drag vertical highlight line (`.panel-resizer:hover::after`, `.panel-resizer.is-dragging::after`), leaving only the `col-resize` cursor.
3. **Redesigned "Mới" (Social Feed) Icon**: Replace the boxy `@` icon (`FeedThreadsIcon` in `icons.rs`) with a clean, balanced, symmetrical duotone Threads/Feed vector SVG.
4. **Community Settings Placement & Header Cleanup**:
   - Keep the community channel/workspace sidebar (`.nav-sidebar`, Image 5) visible on the left when opening Community Settings.
   - Replace only the right chat stage (`.main-stage`) with Settings (`ServerSettingsView`).
   - Remove the top bar showing `[Logo] Cài đặt cộng đồng · <ServerName>` and extra top buttons.
   - Keep `⚙` gear button toggling settings cleanly; clicking any channel/workspace item or "Quay lại" exits settings back to chat.
5. **Unified Left Settings Navigation (No Nested Enterprise Wrapper) & Clean Copy Rules**:
   - Do NOT isolate Enterprise settings into one separate tab containing nested horizontal tabs (`Chung & Giao diện`, etc.). When a community has Enterprise mode enabled, display all Enterprise settings sections directly inside the left `.settings-nav` column alongside Community settings.
   - Zero `&` characters, zero `-` / `—` dashes in channel/category/group/setting names, concise short labels, and strictly sentence-case capitalization (only first letter uppercase, preserving `AI` / `Vico AI`).
   - Migrate existing seeded enterprise categories, channels, departments, and projects in the database and apply clean display normalization for categories/channels (`Thông báo nội bộ`, `Ban điều hành`, `Chủ sở hữu 1`, etc.).

## 2. Acceptance Criteria
- [ ] Chat composer pill floats over a transparent area with zero hint bar or opaque bottom block underneath.
- [ ] Hovering or dragging `.panel-resizer` shows `col-resize` cursor with NO blue bar.
- [ ] `FeedThreadsIcon` displays a refined, sleek vector mark without boxy borders.
- [ ] Clicking Community Settings (`⚙`) keeps the left server sidebar (`.nav-sidebar`) visible, swaps the chat area into the Settings view with NO top logo header bar, and clicking a channel / `⚙` / `Quay lại` exits cleanly.
- [ ] Enterprise settings appear directly as short flat items in `.settings-nav` when Enterprise is active, with zero nested horizontal pill bars.
- [ ] No `-`, `—`, or `&` characters and strictly sentence-case formatting on channel lists, member roster groups (`Chủ sở hữu 1`), and Community/Enterprise settings labels.
- [ ] Unit tests, integration tests, release bundle, live browser verification, and auto-push to `main` succeed.
