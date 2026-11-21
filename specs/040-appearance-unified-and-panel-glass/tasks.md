# Tasks — Spec 040: Fix Sliders, Unify Appearance Tab, Add White Panel Opacity/Glass Control & Chat Header/Composer Polish

- [x] T001 Add `panel_opacity` (0..=100, default 100) and `panel_blur` (0..=32, default 12) to `Prefs` (`app/src/prefs.rs`) and emit `--panel-opacity`, `--panel-blur`, `data-panel-glass` in `app/src/platform/mod.rs`.
- [x] T002 Fix broken global `input[type="range"]` height/thumb visibility, fix wallpaper dim on presets & blur edge halo, and add modal-safe `html[data-panel-glass="true"]` translucency + frosted glass rules in `app/assets/main.css`.
- [x] T003 Move visual appearance & wallpaper settings into `SettingsTab::Appearance` ("Giao diện"), add `Độ mờ đục khung trắng` & `Làm mờ kính khung` sliders with live progress tracks, unify reset button, and keep `Tùy chỉnh` clean (`settings_view.rs`, `settings_more.rs`, `settings_extra.rs`).
- [x] T003b Polish chat header & composer: remove blurred/opaque border on top header (`.chat-header` transparent, no `::before` blur strip) and tune visible white translucent gradient (`.composer::before` with 68% -> 45% -> 15% -> 0% pure white, zero blur) under and behind the floating chat input bar (`app/assets/main.css`).
- [x] T004 Rebuild Web WASM release, run unit tests + new Playwright test `app/e2e/spec040-appearance-panel-glass.mjs` + visually inspect screenshots + run all existing Playwright regression suites.
- [x] T005 Rebuild & install Linux Desktop release + `.deb`, mark tasks complete, commit, and push to `origin/main`.
