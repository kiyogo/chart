# Plan 011
- Migration 0018: users.totp_* columns, backup_codes, twofa_challenges, security_events.
- Backend `security`: TOTP (HMAC-SHA1, ±1 step, replay guard), AES-GCM secret storage, backup codes,
  `complete_login` used by Google callback and dev-login, `/api/auth/2fa/*`, `/api/auth/accounts*`,
  `/api/auth/logout-all`, `/api/security/*`, login alerts over WS (`security.alert`).
- Client: `services/security.rs`, `components/accounts.rs` (TwoFactorPrompt, SignedInAccounts,
  AccountSwitcher, SecuritySettings with QR via the `qrcode` crate), login page, rail menu, devices dialog.
