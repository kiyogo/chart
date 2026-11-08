# Task Checklist: Google OAuth & Port Fix

- [x] Task 1: Update `.env` and `backend/.env` with new Google OAuth Client ID & Secret
- [x] Task 2: Update `backend/src/auth/google.rs` and `backend/src/auth/mod.rs` to track dynamic `redirect_uri` in `OAuthStatePayload`
- [x] Task 3: Add server URL persistence in `crates/chatw-client/src/storage.rs`
- [x] Task 4: Add server switcher and connection health feedback in `crates/chatw-client/src/views/login.rs`
- [x] Task 5: Wire server URL change events in `crates/chatw-client/src/app.rs`
- [x] Task 6: Compile `chatw-client` release build and deploy to `dist-desktop/chatw.exe`
- [x] Task 7: Verify endpoints with curl and launch tests
- [x] Task 8: Git commit and push all changes to `origin main`
