# Spec 009: Avatar Motion Studio, richer profiles, synced settings, Demo Gallery

The user's master prompt (40 sections) is far larger than one change. This spec is
**phase 1** of it, chosen from the prompt's "đặc biệt ưu tiên" list after auditing the repo.
Every item below works end to end (UI → API → database) or it is not claimed.

## Audit (what already existed before this spec)
- Profile: display name, bio, custom status (no expiry), presence incl. invisible, banner (static,
  converted to AVIF), theme colors, 6 decorations + custom decoration upload, 7 profile effects +
  custom effect, 5 avatar effects (no parameters, profile card only), 6 name effects, 7 nameplates.
- Emoji / sticker / GIF libraries with upload (spec 006), OpenMoji (spec 008), recent emoji (local only).
- One-icon reaction button that expands on hover (spec 006) — kept as is.
- App preferences live only in each browser (localStorage); not synced.
- Drawing studio (single layer) for stickers / decorations.

## Goals and acceptance criteria
1. **Avatar Motion Studio**: 40 avatar effects (spin, pulse, glow, orbit, hearts, glitch, …) with
   speed, intensity, color, direction (clockwise / counter-clockwise), easing, trigger
   (always / hover / click) and repeat count. Stored server-side as a strictly validated object;
   shown on the profile card, popover, details panel, conversation list and friend list.
   Respects reduced motion and an animation budget setting (lists: always / hover / off).
2. **Animated cover**: GIF / animated WebP covers keep their animation (uploaded as-is, real type
   checked by magic bytes, size-limited). Avatar uploads are now checked to really be AVIF.
3. **More name effects**: 16 name effects (was 6), all keep text readable.
4. **Status expiry**: custom status can clear itself after 30 min / 1 h / 4 h / today / never;
   the server clears it and tells friends in real time.
5. **Richer profile**: pronouns, location, website, occupation, interests, each with its own
   visibility (everyone / friends / nobody) enforced by the server.
6. **Contact nickname and private note** per person, visible only to the owner, used in lists.
7. **Settings sync across devices** (versioned, conflict-safe), excluding the device PIN lock.
   Includes favorite / recent stickers and GIFs and favorite emoji.
8. **Sticker actions** on a sent / received sticker: add to favorites, send again, find similar.
9. **Save a message to "Chính mình"** (end-to-end encrypted, no plaintext on the server).
10. **Settings search** over every settings page.
11. **Demo Gallery**: interactive previews of avatar motions, name effects, nameplates, profile
    effects, chat wallpapers and reactions; "Áp dụng" saves to the real profile / preferences.
12. **Help Center**: searchable guide with tutorials for the new features and "Mở" buttons.

## Out of scope for phase 1 (reported as Not Started)
Multiple accounts, passkeys / TOTP, layer + timeline animation editor, GIF creator from video,
profile layout drag-and-drop editor, guestbook, achievements, per-community profiles, community
emoji / stickers, forum / event channels, voice settings, media library, backup import/export,
command palette, notification center.
