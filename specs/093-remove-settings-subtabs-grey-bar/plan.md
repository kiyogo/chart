# Plan 093: Remove Horizontal Grey Border Line Under Settings Subtabs

## Architecture & Design
1. **Target CSS Rules**:
   - In `app/assets/main.css`, update `.settings-subtabs`:
     - Change `border-bottom: 1px solid rgba(15, 23, 42, 0.09);` to `border-bottom: none !important;`.
     - In dark / translucent wallpaper selectors (`[data-theme="dark"] .settings-subtabs, html[data-panel-translucent="true"][data-app-wp]:not([data-app-wp="none"])[data-surface-tone="dark"] .settings-subtabs`), set `border-bottom: none !important;`.
2. **Subtab Active Indicator**:
   - `.settings-subtab::after` has `position: absolute; bottom: 0; height: 2px;`.
   - Since `.settings-subtab` has `position: relative` and flex height, removing the container's `border-bottom` preserves the active indicator's exact position without any artifact.
3. **Verification**:
   - Build web bundle: `./app/build-web.sh release`.
   - Run Playwright test/screenshot capturing Settings → Privacy & Security sub-tabs with wallpaper and translucency enabled.
   - Verify visually against the user's attachment `media_1795438166917_904fb5ce.png`.
