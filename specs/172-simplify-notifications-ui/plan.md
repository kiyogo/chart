# Plan 172: Simplify Notifications UI

1. Remove `filter` state and `.notifications-tabs` + `.notifications-empty p` in `app/src/components/notifications_view.rs`.
2. Update `.notifications-empty` and `.notifications-inner` in `app/assets/main.css` to remove the card box frame and center cleanly.
3. Rebuild `app/dist` release bundle, deploy to `chat.kiyo.vn`, verify, and push to GitHub.
