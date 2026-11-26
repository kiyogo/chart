# Plan 173: Clean Contacts List Layout

1. Remove `contacts-summary-bar` in `FriendsList` and `GroupsCommunities` and drop `.contacts-surface-card` from toolbar/add/empty wrappers in `app/src/components/contacts_view.rs`.
2. Update `.contacts-toolbar-card`, `.contacts-toolbar`, `.contacts-alpha-letter`, `.contacts-list-row`, `.contacts-add-box`, and `.contacts-empty-inline` in `app/assets/main.css` to render flat without an outer frame.
3. Rebuild `app/dist` release bundle, deploy to `chat.kiyo.vn`, verify, and push to GitHub.
