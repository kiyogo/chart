# Spec 100 Technical Plan: Eliminate Canvas Strip Blob Request Flooding

## Architectural Strategy
Replace the reactive canvas-to-blob pipeline with pure CSS gradient masking and color-mix for translucent chat headers and composers.

## Detailed Steps

1. **Refactor `app/web/adaptive_surface.js`**:
   - Remove `stripCanvas` and `stripCtx` (160x18 canvas).
   - Remove `cropPanelStripBlobUrl`, `lastTopStripBlobUrl`, `lastBotStripBlobUrl`, `stripRenderSeq`, and `revokeStripBlobs`.
   - In `syncChatEdgeFades()`:
     - Remove the `Promise.all([cropPanelStripBlobUrl(...), cropPanelStripBlobUrl(...)])` branch.
     - Clean up `--chat-top-fade-strip`, `--chat-bot-fade-strip`, and `data-chat-edge-strip`.
     - Keep the dimension variables (`--chat-col-w`, `--chat-col-h`, `--chat-bg-w`, `--chat-bg-h`, etc.) which are pure numbers used by CSS geometry.

2. **Clean up `app/assets/main.css`**:
   - Remove the `.chat-column[data-chat-edge-strip="true"]` rules that set `background-image: var(--chat-top-fade-strip)` and `var(--chat-bot-fade-strip)`.
   - Ensure the existing `.chat-header::before` and `.composer::before` CSS gradient masks with `color-mix(in srgb, var(--bg-chat) var(--panel-opacity, 100%), transparent)` handle the fade seamlessly with hardware acceleration.

3. **Synchronize Build Files**:
   - Copy `app/web/adaptive_surface.js` to `app/dist/adaptive_surface.js`.
   - Run `./app/build-web.sh release`.

4. **Automated Playwright E2E Verification**:
   - Create `app/e2e/spec100-clean-network-no-strip-blobs.mjs`.
   - Track all `blob:` URLs and verify 0 canvas strip blobs are created.
   - Capture verification screenshots of chat scrolling with wallpaper and translucent panels.
