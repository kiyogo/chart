# Spec 097 Tasks: Avatar Skeleton Shimmer & Stable Chat Frame

- [x] Task 1: Update `app/src/components/avatar.rs` to render circular skeleton shimmer while avatar image is resolving
- [x] Task 2: Update `app/src/components/skeleton.rs` so `SkeletonDetails` uses `aside.details-panel.skeleton-details-wrap`
- [x] Task 3: Update `app/src/pages/chat.rs` to remove `SkeletonDetails` replacement, maintaining `.chat-column` and `.details-panel` rock-solid
- [x] Task 4: Recompile WebAssembly release bundle via `./app/build-web.sh release`
- [x] Task 5: Implement and execute automated Playwright E2E verification test `app/e2e/spec097-avatar-shimmer-and-stable-frame.mjs`
- [x] Task 6: Commit and push changes to `origin main` following the mandatory auto-push rule
