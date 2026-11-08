# Feature Spec: Google OAuth Credentials Update & Multi-Port Server Configuration

## 1. Problem Statement
The user reported two critical authentication errors:
1. `ERR_CONNECTION_REFUSED http://127.0.0.1:8082/api/auth/google` when attempting to log in via Google from the desktop client.
2. The user provided updated Google OAuth 2.0 Web Client credentials JSON which has `https://api.chat.kiyo.vn/api/auth/google/callback` as the registered redirect URI, and explicitly requested: "dùng đúng port cho tôi nhé".

Currently:
- The desktop app was hardcoded to `http://127.0.0.1:8082`.
- The backend had stale Google OAuth credentials from an older unrelated project (`plvn01.kiyo.vn`).
- Google OAuth rejects requests when `redirect_uri` does not match the Google Cloud Console registration.

## 2. Requirements & Acceptance Criteria
- **AC 1 (OAuth Credentials)**: Backend uses the new credentials from client secret JSON for project `kiyovn` (stored securely in `.env` / `backend/.env` without committing secrets to git).
- **AC 2 (Multi-Origin / Port Compatibility)**:
  - Backend dynamically tracks `redirect_uri` in the HMAC-signed OAuth state cookie to avoid `redirect_uri_mismatch` during token exchange.
  - Local development on port 8082 (`http://127.0.0.1:8082/api/auth/google/callback`) and production (`https://api.chat.kiyo.vn/api/auth/google/callback`) are both supported.
- **AC 3 (Desktop Client Server Selection & Error Handling)**:
  - Desktop client allows selecting between `http://127.0.0.1:8082` (local) and `https://api.chat.kiyo.vn` (production), with persistence in local app storage.
  - If the server is unreachable, a friendly diagnostic message is shown in the UI instead of raw unhandled errors.
- **AC 4 (Packaging & Delivery)**:
  - Build standalone release binary `dist-desktop/chatw.exe`.
  - Automatically commit and push all changes to GitHub repository `origin main`.
