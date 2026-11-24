# Tasks 130: Universal AI Image Enhancement Effects & Sender Enablement

- [x] Task 1: Update `app/src/components/rich.rs` to enable AI enhancement and effects on sender original images <!-- id: 0 -->
  - Enable `needs_on_demand_ai` on `"sender_orig"`
  - Support `can_ai` and `show_corner_icon` regardless of `is_orig`
  - Remove `!m.is_sender_original` from Lightbox auto-enhance checks and wand button toggle
  - Add frosted aurora & dots to `blur_parts` initial image loading container
- [x] Task 2: Update `app/src/components/details_panel.rs` to enable `auto_enhance_on_open: media_can_ai` <!-- id: 1 -->
- [x] Task 3: Build Web Bundle and verify compilation with `./app/build-web.sh` <!-- id: 2 -->
- [x] Task 4: Create and run Playwright E2E verification test `app/e2e/spec130-universal-image-effects.mjs` <!-- id: 3 -->
- [x] Task 5: Auto-commit and git push to `origin main` (Rule 1) <!-- id: 4 -->
