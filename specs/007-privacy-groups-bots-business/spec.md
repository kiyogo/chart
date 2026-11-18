# Spec 007 — Privacy, devices & self chat, labels & pins, group management, bots, business pages, directory, admin

## User request (summary)
More stickers/emoji/GIFs, PIN lock after inactivity, auto-reply, Excel paste options, double-click to reply,
typing/read-receipt visibility, sticker & @mention suggestions, notifications, deep UI customization including
app / per-chat wallpapers (shared with the other person), blocking, privacy controls, many languages,
"Kinh doanh" pages with a public link, labels for chats, pop-out channels, pinned chats & messages, group vs
community distinction, Telegram-like bots with tokens and naming rule, invite **links**, Discord-like roles,
Zalo-like group management (leader/deputy, member permissions, approval, link join, block group), polls/notes/
reminders, Meet-like rooms, a search page with public directory (verified only), a separate admin web,
search-engine indexing of good public community content, and a "Chính mình" self chat with device names.

## Audit findings fixed in this wave
- `GET /api/servers/discover` listed every server and `POST /api/servers/{id}/join` let anyone join any server by id.
- 1:1 calls had no friendship check (anyone could ring anyone by id).
- Group creators/admins could add any user to a group without consent.
- Sessions never stored a device name.

## Acceptance (this wave)
1. Migration 0011 + server-side enforcement: blocks (messages, friend requests, calls, group adds, search),
   privacy settings (friend requests, calls, group invites, discoverable, read receipts, typing).
2. Self chat "Chính mình": messages to yourself (E2EE to own key), device labels "<OS> <n>", rename, delete & recreate.
3. Labels, pinned conversations, pop-out channels, groups shown under Communities — synced per user.
4. DM read receipts (respecting privacy), shared per-conversation wallpaper with realtime update.
5. Group management: member permissions (send, poll, note, reminder, pin, rename, avatar), join approval,
   highlight admins, link join toggle, reusable invite link (copy / share / regenerate), block group, invite preview.
6. Bots: create / edit / delete / rotate token (stored as SHA-256), Bot API getMe / getUpdates / sendMessage,
   private and group chats; bot names may not take another user's username but may reuse the owner's.
7. Business pages `/k/{handle}` (server-rendered, escaped, SEO meta + JSON-LD, sitemap), products, quick replies.
8. Directory search of verified public communities / groups / bots / businesses; join public groups.
9. Admin web `/admin` (separate assets): stats, verification queue, suspend / unsuspend, audit log.
10. Public community pages `/c/{id}` only for verified + public + opted-in communities.
11. Client: UI for all of the above plus notifications, PIN lock, auto-reply, suggestions, paste handling,
    polls / notes / reminders, customization. Remaining items are listed in artifacts/feature_matrix.md.
