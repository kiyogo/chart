# Tasks 011
- [x] T1 Migration 0018 and backend `security` module (TOTP, backup codes, challenges, log, vault)
- [x] T2 Google callback and test login go through `complete_login` (second step + login alerts)
- [x] T3 Sign out other devices requires the second step when it is on
- [x] T4 Client: second-step screen, signed-in accounts on the sign-in page, avatar-menu switcher
- [x] T5 Settings › Bảo mật: accounts on this device (order, hide, sign out), TOTP setup with QR, backup codes, security log
- [x] T6 Realtime new-device alert (toast + system notification)
- [x] T7 Tests: RFC 6238 vectors, base32, masking; `app/e2e/security-check.mjs` 21/21; UI audit

## Found and fixed while testing
- "Thêm tài khoản" did nothing: the request was cancelled when the menu closed (now runs independently).
