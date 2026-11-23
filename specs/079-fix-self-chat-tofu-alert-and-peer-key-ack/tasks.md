# Spec 079 — Tasks

- [x] **T1**: Update `get_or_verify_peer_keys_scoped` and `acknowledge_peer_key_change` in `app/src/crypto/e2ee.rs` to never trigger TOFU alerts on self-chat (`my_user_id == Some(peer_id)`), proactively clear stale self alerts + legacy unscoped alerts, and add unit test `test_self_chat_key_rotation_never_triggers_tofu_alert`.
- [x] **T2**: Sync self peer slot & clear self alert inside `reset_device_encryption_key` (`app/src/components/storage_ui.rs`).
- [x] **T3**: Guard `select_friend` & WS handlers with `peer != uid`, add clean 1-click inline `.chat-banner.danger` with `[ Security ]` + `[ New key verified ]` for real peer key rotations, remove redundant overlapping bottom toast, and pass `my_user_id` + `on_key_verified` into `DmDetailsPanel` (`app/src/pages/chat.rs`, `app/src/components/details_panel.rs`) with auto-expanded Security section.
- [x] **T4**: Run Rust unit tests (33/33 pass), rebuild release bundle (`./app/build-web.sh release`), write & run `app/e2e/spec079-no-self-chat-tofu-alert.mjs` (4/4 pass) + regressions (`spec076` 10/10, `spec077` 9/9, `spec078` 8/8), and visually inspect screenshots.
- [x] **T5**: Mark tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
