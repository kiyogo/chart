# Tasks: Spec 126 - Omit File Extension When Saving and Displaying Media Filenames

- [x] Task 1: Update `app/web/media_optimizer.js` to strip file extensions from `fileName`
- [x] Task 2: Update `app/src/storage/media_opt.rs` with `strip_file_extension` helper and payload sanitation
- [x] Task 3: Update `app/src/components/rich.rs` and `app/src/components/details_panel.rs` to display and download clean extension-free filenames
- [x] Task 4: Rebuild web bundle via `./app/build-web.sh`
- [x] Task 5: Create and run Playwright verification test `app/e2e/spec126-omit-extension-on-save.mjs`
- [x] Task 6: Run regression tests, commit and git push to `origin main`
