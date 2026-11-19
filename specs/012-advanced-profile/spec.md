# Spec 012: advanced profile

Prompt sections IV (4.2–4.4), XVII, XVIII, XIX. Builds on spec 009 details and motion.

## Acceptance criteria
- AC1 Profile sections (about, details, badges, showcase, achievements, guestbook, shared communities,
  mutual friends, links) can be reordered by drag and drop or ↑↓, shown / hidden, normal / wide; saved
  on the server; hidden sections are never sent to other people.
- AC2 21 profile templates (Minimal … Community Profile) that style only the profile card.
- AC3 Up to 10 saved layout presets; apply / delete; export / import as JSON; reset to default.
- AC4 Preview as owner / friend / stranger and desktop / mobile, applying the same visibility rules as the server.
- AC5 Showcase (≤ 12): links, uploaded images / GIFs, favourite stickers; pin and order; only our own media URLs.
- AC6 Guestbook: off / friends / everyone; blocked users excluded; owner or author can delete; 300 chars;
  5 entries per hour per author.
- AC7 15 achievements computed from real activity (messages, friends, groups, communities, sticker packs,
  drawings, animations, 2FA, style, profile completeness, bots, business, one hidden), with progress for the
  owner, unlock history, and only unlocked ones visible to others.
- AC8 Inventory page: equipped items, sticker packs, favourites, custom emoji, GIFs, drawings (open in
  Studio), layout presets (apply), achievements; search and categories.

## Out of scope (reported)
Free-form resizing of sections beyond normal / wide, profile visitors, item trading, collections of items.
