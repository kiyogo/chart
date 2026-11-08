# Implementation Plan: Google OAuth & Server Port Configuration

## 1. Architectural Changes
1. **Environment Configuration**:
   - Update `backend/.env` and `.env` with new `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`.
   - Set default `GOOGLE_REDIRECT_URL=https://api.chat.kiyo.vn/api/auth/google/callback`.
2. **Backend Dynamic OAuth Handling**:
   - File: `backend/src/auth/google.rs` & `backend/src/auth/mod.rs`
   - Store `redirect_uri: String` inside `OAuthStatePayload`.
   - Pass the verified `redirect_uri` to `exchange_code_for_token`.
   - Accept optional `redirect_uri` parameter or inspect `Host` header on `GET /api/auth/google` (allowing local 127.0.0.1:8082 if requested).
3. **Desktop Client Server URL Storage & UI**:
   - File: `crates/chatw-client/src/storage.rs`: Add `load_server_url` and `save_server_url`.
   - File: `crates/chatw-client/src/views/login.rs`: Add sleek server toggle/selector (`127.0.0.1:8082` vs `api.chat.kiyo.vn`).
   - File: `crates/chatw-client/src/app.rs`: React to server changes, recreate `HttpClient`, show connection error if server is offline.
4. **Compilation & Packaging**:
   - Stop any existing backend/client processes.
   - Build `chatw-client` in release mode.
   - Copy to `dist-desktop/chatw.exe`.
5. **Git Push**:
   - Run `git add .`, commit with `fix(auth): update google credentials and support multi-server port configuration`, and `git push origin main`.
