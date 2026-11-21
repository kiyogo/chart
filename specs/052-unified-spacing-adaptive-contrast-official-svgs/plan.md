# Implementation Plan: Spec 052 — Unified Spacing, Adaptive Contrast, Official SVGs & Chat Header Polish

## 1. Architecture & Files to Modify

1. **`app/src/components/brand_logos.rs`**:
   - Embed official SVG paths from Simple Icons (`simple-icons`) and `flag-icons` for all Cloud providers (`google_drive`, `onedrive`, `dropbox`, `nextcloud`, `webdav`, `s3`, `r2`, `minio`, `nas`, `icloud`), Operating Systems (`linux` Tux multicolor vector, `windows`, `macos`, `android`, `ios`), Browsers (`chrome`, `firefox`, `safari`, `edge`, `brave`, `opera`), Auth badges (`google`, `passkey`, `2fa`, `e2ee`, `zip`), and all 33 language flags (`vi`, `en`, `zh-CN`, `zh-TW`, `ja`, `ko`, `th`, `id`, `ms`, `fil`, `km`, `lo`, `hi`, `bn`, `ar`, `fa`, `he`, `ur`, `tr`, `ru`, `uk`, `pl`, `cs`, `de`, `fr`, `es`, `pt-BR`, `it`, `nl`, `sv`, `ro`, `hu`, `el`).

2. **`app/src/platform/mod.rs`**:
   - Upgrade `CONTRAST_ENGINE_JS` to sample a `48x48` grid for both app wallpapers and chat wallpapers, calculating regional luminance (`meanB`, `railB`, `navB`, `mainB`), standard deviation (`stdB`), Sobel/neighbor edge density (`edgeB`), bimodal dark/bright pixel ratios (`darkRatio`, `brightRatio`), and dominant RGB tint.
   - Compute dynamic CSS custom properties (`--adaptive-text-halo`, `--adaptive-title-halo`, `--adaptive-ctrl-bg`, `--adaptive-ctrl-border`, `--chat-fade-color`) and `data-text-halo="true"` when transparency or wallpaper complexity requires contrast reinforcement, darkening text ink and adding a dark/dual-polarity glow behind text.

3. **`app/src/components/messages.rs`**:
   - Update `day_label(date: NaiveDate)` so current-year dates format as `"{} tháng {}"` (e.g. `"20 tháng 11"`) and other-year dates format as `"{} tháng {} năm {}"` (e.g. `"22 tháng 11 năm 2025"`).

4. **`app/src/pages/chat.rs`**:
   - Move Self-Chat (`self_mode`) device pills inside `header.chat-header` to the right of `.chat-header-identity`, deleting the `"Thiết bị:"` text and removing the separate second bar below `.chat-header`.

5. **`app/src/components/settings_more.rs`, `app/src/components/backup_cloud_ui.rs`, `app/src/components/storage_ui.rs`**:
   - Move `"Mặc định"` buttons to the left of `<input type="color">` in `AppearanceColorsShapes`.
   - Remove the 3-step cloud guide banner (`.cw-cloud-guide`) and verbose explanatory subtitles, and shorten all setting labels.

6. **`app/assets/main.css`**:
   - Lock `--panel-inset: 8px` across `.nav-sidebar`, `.settings-nav`, `.settings-body`, `.feed-column`, `.meet-column`, `.notifications-page`, `.download-page`, `.search-page`, `.plugin-panel`, and `.details-panel` for left, right, and top insets.
   - Style `.chat-item.has-nameplate` so hover shows only a crisp `1px` grey border (`inset 0 0 0 1px`) and active/selected shows only a crisp `1px` accent border (`inset 0 0 0 1px var(--accent)`), and enlarge `.chat-item` avatar (`--avatar-list: 3.45rem`, tighter padding `3px 6px 3px 3px`).
   - Style `.settings-group-title` to match `.sidebar-title` ("Tin nhắn": `font-size: var(--text-xl)`, `font-weight: var(--weight-bold)`, `color: var(--text-primary)`).
   - Synchronize `.app-rail` background, `--panel-opacity`, and `--panel-blur` with `.settings-nav` and `.settings-body`.
   - Style `input[type="color"]` full-bleed with zero inner black border/padding, and style all `.btn-ghost` buttons as framed buttons matching `.btn-secondary`.
   - Remove `position: sticky` from `.date-separator` so date pills never stack/overlap.
   - Float `.chat-header` at the top of `.chat-column` with a top unblurred gradient fade (`backdrop-filter: none`) matching the chat surface.
   - Restore `.meet-column` bottom rounded corners (`border-radius: var(--panel-radius) !important; margin-bottom: 0 !important;`).
