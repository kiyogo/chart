# Engineering Plan — Spec 041: Global Borderless Cleanliness & Sleek Slim Chat Composer Overhaul

## Technical Architecture & Files Affected

### 1. [app/assets/theme.css](file:///run/media/ngson/DATA1/DATA/source/chatw/app/assets/theme.css)
- Update `--focus-ring: none;` on `:root` to eliminate default 3px outline rings globally.

### 2. [app/assets/main.css](file:///run/media/ngson/DATA1/DATA/source/chatw/app/assets/main.css)
- **Global Focus Reset**:
  - Update `button:focus-visible, a:focus-visible, [tabindex]:focus-visible, input:focus-visible, textarea:focus-visible, select:focus-visible` to `outline: none;`.
  - Update `input:not(...):focus, textarea:focus, select:focus` to `box-shadow: none; outline: none;`.
  - Update `input[type="checkbox"].switch:focus-visible` to `box-shadow: none; outline: none;`.
- **Composer Overhaul**:
  - Replace `.composer-box`, `.composer-box::before`, `.composer-box:focus-within`, `.composer-box .icon-btn`, `.composer-input`, and `.send-btn` with the modern, slim capsule pill styling (`border-radius: 9999px;`, `min-height: 2.125rem`, compact padding `3px 4px 3px 6px`, zero focus ring/border, circular icon buttons).
  - Add dark theme overrides for `.composer-box::before`.
  - Eliminate divider borders on `.plugin-task-footer` and `.feed-compose-expanded`.

### 3. [app/e2e/spec041-borderless-sleek-composer.mjs](file:///run/media/ngson/DATA1/DATA/source/chatw/app/e2e/spec041-borderless-sleek-composer.mjs)
- Playwright verification checking:
  1. Composer box has zero border width, zero outline, zero focus ring on `:focus-within`.
  2. Composer input is focused and has zero border and zero focus ring.
  3. Composer box has `border-radius: 9999px` (capsule pill).
  4. Composer box total height is slim (`<= 44px`, down from ~52px).
  5. Search input and global controls have zero focus ring on focus.
  6. Zero page errors.
  7. Visual screenshots of the new sleek chat composer saved to artifact directory.

### 4. Build & Delivery
- Run `./app/build-web.sh release` to compile WASM bundle with inlined CSS into `app/dist/`.
- Run Playwright test suite and all regression tests.
- Rebuild Linux desktop release binary and `.deb` package.
- Commit with Conventional Commits and push to `origin/main`.
