# Plan 130: Universal AI Image Enhancement Effects & Sender Enablement

## Architecture & Implementation Steps

### 1. `app/src/components/rich.rs`
- **Sender Original AI Pipeline**:
  - In `run_ai_pipeline` / eval JS: When checking sender local original (`getSenderOriginalFromCache`), also check if the image has already been enhanced in IndexedDB cache (`chatw_ai_restorer.restoreImageFromUrl(..., { cacheOnly: true })`). If cached, emit `"done"` with `cacheHit: true`. If not, emit `"sender_orig"` with the local blob URL.
  - In `"sender_orig"` event handler in Rust:
    Set `needs_on_demand_ai.set(true);` and `raw_src.set(Some(orig_url.to_string()));`.
  - In `can_ai` determination:
    Change `let can_ai = !is_orig && data.m != "image/gif";` to:
    `let can_ai = data.m != "image/gif" && !data.m.starts_with("video/");`.
  - In `show_corner_icon`:
    `let show_corner_icon = needs_on_demand_ai() && !is_ai && !is_loading && !is_revealing && can_ai;`.
  - In Lightbox opening (`*GLOBAL_LIGHTBOX.write()`):
    Pass `can_ai_enhance: can_ai` and `auto_enhance_on_open: show_corner_icon`.
  - In Lightbox auto-enhance logic (`cur_key` change & `use_effect`):
    Remove `!m.is_sender_original` from:
    - line 2409: `auto_enhance_on_open && !m.ai_restored && m.can_ai_enhance`
    - line 2539: `if m.auto_enhance_on_open && !m.ai_restored && m.can_ai_enhance`
  - In Lightbox wand toggle button condition:
    Change line 2589 to:
    `let can_toggle_ai = media.can_ai_enhance && media.original_src.is_some();` (remove `!media.is_sender_original`).
  - In initial loading state (`blur_parts`):
    Add `.ai-gen-frost`, `.ai-gen-aurora`, `.ai-gen-dots-base` to provide a subtle shimmer while downloading/decrypting.

### 2. `app/src/components/details_panel.rs`
- In `onclick` for `self-media-tile` (line 604):
  Set `auto_enhance_on_open: media_can_ai`.

### 3. Build & E2E Verification
- Compile web bundle with `./app/build-web.sh`.
- Create `app/e2e/spec130-universal-image-effects.mjs` verifying:
  - Self-sent image displays corner Focus icon.
  - Clicking image opens Lightbox and displays frosted crystal aurora shimmer overlay (`.lightbox-ai-gen`).
  - Progress advances to 100% and reveals 4K enhanced image.
  - Lightbox toolbar includes wand toggle button (`.lightbox-enhance-toggle`).
- Execute E2E test.
- Git commit & push (Rule 1).
