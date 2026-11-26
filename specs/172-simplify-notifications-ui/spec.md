# Spec 172: Simplify Notifications UI (Remove Filter, Empty Frame & Description)

## Problem
The Notifications page (`notifications_view.rs`) displayed:
1. A filter tab bar (`All`, `Message (0)`, `Make friends (0)`) in `.notifications-tabs`.
2. An unnecessary frosted box frame around `.notifications-empty` and a redundant description paragraph (`All your unread messages and friend requests will be displayed here.`).

## Requirements
- Remove the `.notifications-tabs` filter bar completely from `NotificationsView`.
- Remove the outer box frame (`background: transparent; border-radius: 0;`) and description `<p>` from `.notifications-empty`, centering the empty state cleanly inside `.notifications-page`.
