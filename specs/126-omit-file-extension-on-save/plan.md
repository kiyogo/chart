# Technical Plan: Spec 126 - Omit File Extension When Saving and Displaying Media Filenames

## Architecture & Code Changes
1. **Frontend Media Optimizer (`app/web/media_optimizer.js`)**:
   - Add utility function `stripFileExtension(fileName)`:
     ```javascript
     function stripFileExtension(fileName) {
       if (!fileName || typeof fileName !== 'string') return '';
       const idx = fileName.lastIndexOf('.');
       return idx > 0 ? fileName.slice(0, idx) : fileName;
     }
     ```
   - Use `stripFileExtension` for all output `fileName` assignments (e.g. lines 976, 1006, 1033, 1069, 1124, 1216, 1253).
2. **Rust Media Optimizer (`app/src/storage/media_opt.rs`)**:
   - Add `pub fn strip_file_extension(name: &str) -> String`.
   - Apply `strip_file_extension` in `OptimizedMediaPayload` parsing.
3. **Rust Rich Media Components (`app/src/components/rich.rs` & `app/src/components/details_panel.rs`)**:
   - In `build_rich_media_wire`: sanitize `name` with `strip_file_extension`.
   - In Lightbox component:
     - Render `.lightbox-filename` with `strip_file_extension(&media.name)`.
     - Set `dl_name = strip_file_extension(&media.name)`.
   - In `details_panel.rs`: sanitize `display_name` with `strip_file_extension`.
4. **Verification**:
   - Write Playwright E2E test `app/e2e/spec126-omit-extension-on-save.mjs`.
   - Run test, capture screenshot in `verify_spec126/01_clean_title_no_extension.png`.
   - Run regression tests.
