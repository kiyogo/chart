# Spec 100 Tasks: Eliminate Canvas Strip Blob Request Flooding

- [x] Task 1: Remove `stripCanvas` and `cropPanelStripBlobUrl` from `app/web/adaptive_surface.js`
- [x] Task 2: Remove `.chat-column[data-chat-edge-strip="true"]` rules from `app/assets/main.css`
- [x] Task 3: Sync `app/dist/adaptive_surface.js` and recompile release bundle via `./app/build-web.sh release`
- [x] Task 4: Implement and run automated Playwright verification test `app/e2e/spec100-clean-network-no-strip-blobs.mjs`
- [x] Task 5: Commit and push changes to `origin main` following the mandatory auto-push rule
