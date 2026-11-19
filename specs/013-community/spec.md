# Spec 013: community features

Prompt section XXIV (groups, servers and communities).

## Acceptance criteria
- AC1 Per-community profile: nickname, avatar and short bio used in that community's channels and forum;
  the global profile is never changed.
- AC2 Community emoji and stickers (≤ 100 each, English lowercase names, unique per community) managed with
  the new "Manage Expressions" permission; members see them in the emoji / sticker pickers in that community.
- AC3 Forum channels: topics with replies, pin and lock for moderators; replies stay out of the channel feed.
- AC4 Event channels and events: create / delete (Manage Channels), RSVP going / interested with counts.
- AC5 Moderation audit log (Manage Server): roles, kicks, channels, settings, emoji / stickers, events, welcome,
  locked topics.
- AC6 Welcome screen with message, rules and starter channels; optional "must accept the rules" gate enforced
  by the server for channel messages and forum posts.

## Out of scope (reported)
Per-channel emoji / stickers / themes, temporary channels, community analytics, member notes, onboarding questions.
