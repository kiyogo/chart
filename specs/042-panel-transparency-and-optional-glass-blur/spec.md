# Feature Specification — Spec 042: Intuitive Panel Transparency Scale, Optional Glass Blur Toggle & Live Blur Responsiveness

## Problem Statement
1. **Confusing Opacity Scale**: The previous slider was titled "Độ mờ đục khung trắng" where 100% was solid/opaque and 0% was transparent. Users intuitively expect "100%" to mean maximum transparency ("100% là càng trong suốt á đổi lại cho tôi").
2. **Glass Blur Was Always Active by Default**: Frosted glass blur should not be enabled by default; users requested a dedicated toggle checkbox to choose whether they want glass blur or clean sharp transparency ("không có làm mờ đầu nhé muốn có là mở thì phải bấm 1 nút tichsc").
3. **Blur Slider Appeared Ineffective When Panels Were Opaque**: When panel transparency was 0% (solid white), adjusting the "Làm mờ kính khung" slider produced no visible change on screen ("và tôi chỉ Làm mờ kính khung có thay đổi gì đâu").

## Acceptance Criteria
1. **Intuitive Panel Transparency Scale (0% - 100%)**:
   - Re-label slider to: `Độ trong suốt khung trắng: {transparency}%`.
   - Scale:
     - `0%` (default): Khung trắng đặc bình thường (100% opaque, 0% transparent).
     - Dragging right (e.g. 40%, 70%, 100%): Khung càng trong suốt, hiện rõ hình nền toàn ứng dụng.
     - `100%`: Trong suốt hoàn toàn.
   - Internal calculation: `panel_opacity = 100 - transparency`.
2. **Dedicated Toggle for Frosted Glass Blur (No Blur by Default)**:
   - Add `panel_glass: bool` (default `false`) to `Prefs`.
   - Add switch row: `Làm mờ kính khung (Frosted glass)` with description `Hiệu ứng kính mờ phủ sương phía sau các khung khi trong suốt`.
   - By default (`panel_glass == false`):
     - `data-panel-glass="false"`.
     - `--panel-blur: 0px`.
     - Zero blur applied across panels (`backdrop-filter: none`). Translucent panels remain pure, crisp, and sharp without smudge.
   - When user clicks/ticks the toggle switch:
     - `panel_glass = true`.
     - If current transparency is 0% (solid white), automatically set transparency to a visible default (`40%`, i.e., `panel_opacity = 60`) so the user immediately sees the frosted glass effect in action.
     - Reveal the blur slider: `Mức độ làm mờ kính: {pr.panel_blur}px` (0px – 32px).
3. **Live Blur Responsiveness**:
   - When `panel_glass` is enabled and transparency > 0%, dragging the blur slider from 0px to 32px live-updates `--panel-blur` so the background behind panels immediately shifts from flat glass to frosted mist in real time.
4. **Verification & Delivery**:
   - Playwright test `app/e2e/spec042-transparency-toggle-blur.mjs` verifying:
     - Default state: 0% transparency (solid white), glass blur switch is OFF, zero blur.
     - Dragging transparency slider to 50% updates `--panel-opacity` to 50%.
     - Ticking the glass blur switch turns on `data-panel-glass="true"`, sets visible transparency, and displays the blur slider.
     - Adjusting the blur slider live-updates `--panel-blur`.
     - Zero runtime page errors.
   - Run all existing regression suites.
   - Rebuild Web WASM release bundle and Linux desktop release binary + `.deb`.
   - Commit with Conventional Commits and push to `origin/main` (Rule #1).
