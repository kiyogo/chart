# Plan 013
- Migration 0020: server_members (avatar_url, bio, rules_accepted_at), servers (welcome_message, rules,
  require_rules), server_expressions, forum_threads + channel_messages.thread_id, server_events + RSVPs,
  server_audit_log; permission bit 1024 Manage Expressions.
- Backend `community` module + hooks: channel messages use community nickname / avatar, skip thread replies,
  rules gate; audit entries from roles, kicks, channels, settings.
- Client: `services/community.rs`, `components/community_ui.rs` (WelcomeDialog, CommunityProfileDialog,
  EventsPanel, ForumView, WelcomeEditor, ExpressionsManager, AuditLogView), server settings tabs, channel views,
  pickers, create-channel types.
