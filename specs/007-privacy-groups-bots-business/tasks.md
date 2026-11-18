# Tasks 007
- [x] T1 Migration 0011 (devices, privacy, blocks, labels, prefs, reads, wallpapers, group mgmt, bots, business, admin)
- [x] T2 privacy module + enforcement in messages / friends / calls / groups / user search
- [x] T3 convo module (labels, prefs, reads, wallpapers, self chat) + device labels & rename
- [x] T4 group management module + permission checks + join approval + invite links
- [x] T5 bots module (management + Bot API)
- [x] T6 business module + public pages + dynamic sitemap
- [x] T7 directory + admin API + admin web + server discover/join fix
- [x] T8 Client: Chính mình (device labels, rename, delete / recreate), pinned conversations and messages, labels, read receipts ("Đã xem"), block / unblock, DM reply, privacy settings
- [x] T9 Client: notifications + sounds, PIN lock (PBKDF2, idle + on start), auto-reply + Kinh doanh greeting, @ / emoji / quick-reply / sticker suggestions, spreadsheet paste (ask / image / table / text), polls, notes, reminders, E2EE images
- [x] T10 Client: deep customisation (accent, bubbles, shapes, fonts, width, app wallpaper) + shared per-chat wallpapers; Zalo-style group management; invite links + invite cards; bots (chat, create, token, docs); Kinh doanh settings; full-page search + verified directory; 33-language core UI with RTL
- [x] T12 Server roles: edit / delete roles, assign / unassign, kick, my-permissions; new bits (kick, @everyone, voice) enforced server-side; server settings UI
- [x] T13 Emoji: full RGI set available to this build (1,906 + skin tones, 259 flags); 126 original stickers in 7 packs; 22 animated reactions (tools/gen_stickers.py)
- [x] T14 SEO: public community page lists pinned ("good") messages first
- [x] T11 Tests (unit + API smoke + UI audit), screenshots, reports, push

## Deferred (reported to the user)
- Meet-style rooms in the rail with guest join (name only) and lobby approval: needs a rooms service with guest sessions.
- Live streaming ("Go Live") in voice channels.
- Licensed third-party sticker / emoji art packs: need the user's permission to download.
- Translating every screen: only the core navigation, list and composer strings are translated.
