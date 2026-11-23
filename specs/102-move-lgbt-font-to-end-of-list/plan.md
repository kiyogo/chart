# Spec 102 Technical Plan: Move LGBT Font to the End of the List

## Architecture & Code Changes
1. **`app/src/components/settings_more.rs`**:
   - Move `("lgbt", "LGBT")` from line 542 (2nd item) to the end of the `opts(&[...])` array (17th item, after `("dyslexic", "Readable")`).
2. **`app/src/prefs.rs`**:
   - Keep `"lgbt"` in the allowed list of fonts (reorder to end for consistency).
3. **Recompile WASM Release Bundle**:
   - Run `cargo check -p chatw-app --target wasm32-unknown-unknown`.
   - Run `./app/build-web.sh release`.
4. **Automated Playwright E2E Verification**:
   - Implement `app/e2e/spec102-move-lgbt-font-to-end.mjs`.
   - Open Settings -> More / Appearance.
   - Click the Font picker dropdown.
   - Verify that the last option in `.cw-select-menu .cw-select-option` is `LGBT` (`data-value="lgbt"` or `data-font-preview="lgbt"`).
   - Take screenshot of the open dropdown.
