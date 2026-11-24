# Tasks 128: Butter-Smooth Kinetic Dragging for Stories & Notes Strip

- [x] 1. Update `.notes-strip` and `.note-item` CSS in `app/assets/main.css` (remove scroll snap, add overscroll contain and hardware acceleration)
- [x] 2. Rewrite drag controller in `app/src/components/navigation/conversations.rs` with rAF, pointer capture, zero-jerk deadzone, and kinetic momentum
- [x] 3. Update `feed_view.rs` with the unified kinetic drag controller
- [x] 4. Rebuild Web frontend bundle with `./app/build-web.sh`
- [x] 5. Create and run Playwright E2E verification test `app/e2e/spec128-butter-smooth-stories-drag.mjs`
- [x] 6. Run regression tests (`spec123`, `spec126`, `spec127`)
- [x] 7. Mandatory auto-commit and push to `origin main` (Rule 1)
