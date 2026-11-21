# Implementation Plan — Spec 038

1. **Bottom-Flush Chat Column (`app/assets/main.css`)**:
   - Update `.chat-column` to extend flush to the bottom edge of `.workspace-shell` (`margin-bottom: calc(-1 * var(--shell-gap)); border-bottom-left-radius: 0; border-bottom-right-radius: 0;`), reverting `margin-bottom: 0;` in `@media (max-width: 767px)`.
   - Ensure `.composer` remains `background: transparent !important` with zero extra bottom bar, while `.chat-messages` reaches `bottom: 0` cleanly.
2. **Icon System Redesign (`app/src/components/icons.rs`, `app/src/components/navigation/rail.rs`, `app/src/components/composer.rs`, `app/src/components/rich.rs`, `app/src/components/messages.rs`, `app/src/pages/chat.rs`, `app/assets/main.css`)**:
   - Upgrade `Duo` SVG wrapper stroke to `2.05` and modernize the entire icon set in `icons.rs` ("ChatW Duo v2") with full-bleed `2..22` optical geometry, clean single-stroked paths, `0.22` duotone fill depth, and a mathematically centered 8-tooth `SettingsIcon` with a true transparent `fill-rule="evenodd"` center cutout.
   - Increase `.app-rail` icon sizes in `rail.rs`: `LogoIcon` `30`, top nav icons `28`, bottom rail icons `26`, with `.rail-brand` / `.rail-item` (`2.625rem`) and `.rail-item.compact` (`2.5rem`) in `main.css` so icons feel bold, crisp, and balanced beside `40px` app/plugin tiles.
   - Increase Composer icon sizes (`PlusIcon`, `MicIcon`, `EmojiIcon`, `GifIcon`, `StickerIcon`, `SendIcon` to `24`), Chat header action icons (`PhoneIcon`, `VideoIcon`, `PanelRightIcon` to `22`, `MoreIcon` to `20`), and uniform Message hover action icons (`19`) while preserving explicit `size: 16` call sites (such as `.voice-status-panel`).
3. **Verification & Delivery**:
   - Add Playwright verification (`app/e2e/spec038-chat-bottom-icons.mjs`) confirming `chatColumnBottomGap === 0`, `chatMessagesBottomGap === 0`, rail icons `30/28/26px`, composer icons `24px`, `SettingsIcon` evenodd cutout, and visually inspect light & dark mode screenshots.
   - Re-run all regression suites, rebuild Web & Linux Desktop release, install on machine, commit, and push to `origin/main`.
