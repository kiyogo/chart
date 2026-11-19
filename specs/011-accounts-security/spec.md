# Spec 011: several accounts and account security

Prompt sections III (3.3 multiple accounts) and XXVI (account security). ChatW signs in with Google
(plus a loopback-only test login), so two-step verification is a second step after that sign-in.

## Acceptance criteria
- AC1 Up to 5 accounts signed in on one browser; switch from the avatar menu without signing in again;
  per-account keys, preferences, theme, notifications and PIN stay separate (all keyed by user id).
- AC2 Add account, sign out one account, sign out all accounts; sign-in page lists parked accounts;
  per-device order and "hide from quick switch"; unread count per account.
- AC3 TOTP (RFC 6238) with QR code; enabling needs a valid code; 10 one-time backup codes (hashed);
  regenerate / disable need a code; codes cannot be replayed; 10 wrong codes in 15 min → 429;
  challenges expire after 5 minutes and allow 5 attempts.
- AC4 Every sign-in (Google, test) asks for the second step when it is on.
- AC5 Sensitive action: signing out every other device needs the second step when it is on.
- AC6 Login alerts: a sign-in from a device type never used before is logged and pushed to open apps.
- AC7 Security log (100 latest events) with device and masked IP.

## Security notes
Other accounts' session tokens live only in an HttpOnly cookie scoped to /api/auth (never readable by
scripts). TOTP secrets are encrypted at rest with AES-256-GCM (key derived from the server secret).

## Out of scope (reported)
Passkeys / WebAuthn (needs a WebAuthn verifier and a real domain; not on 127.0.0.1), biometric unlock,
password sign-in (ChatW has no passwords), SMS / email codes.
