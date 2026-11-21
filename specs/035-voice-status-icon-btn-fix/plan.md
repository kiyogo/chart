# Implementation Plan — Spec 035

1. **`app/assets/main.css`**:
   - Add `padding: 0; min-height: 0;` to `.icon-btn, .settings-close-btn, .partner-info-close-btn`.
   - Enforce `padding: 0; min-height: 0; width: var(--icon-btn); height: var(--icon-btn); background: transparent; color: var(--danger);` on `.icon-btn.danger, .icon-btn.btn-danger` and `background: var(--bg-error); color: var(--danger);` on `:hover`.
2. **`app/src/components/navigation/sidebar.rs`**:
   - Switch voice status bar buttons from `"icon-btn btn-danger"` to `"icon-btn danger"`.
3. **Verification**:
   - Build web bundle (`./app/build-web.sh`), run Playwright check joining a community voice channel to assert button & SVG dimensions (`16×16` SVG, equal button width/height, `0px` padding) and visually inspect the screenshot.
   - Rebuild/reinstall Linux Desktop app and push to `origin/main`.
