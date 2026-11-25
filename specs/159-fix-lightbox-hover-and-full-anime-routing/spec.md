# Spec 159: Remove Lightbox Button Hover White Circle & Fix Full Anime Routing + Cache Invalidation

## Problem Statement
1. Hovering or toggling `.lightbox-tool-btn` / `.lightbox-enhance-toggle` displays a milky white circle (`background: rgba(255, 255, 255, 0.16)`) and scale/background effects (`media_1795615093373_7060febd.png`), unlike clean borderless icon buttons.
2. Full-frame anime/cartoon illustrations with comic outlines & text (`luffy_full_banner.png`: `outline_r = 0.0228`, `sat = 0.359`) were still classified as `"portrait"` (`NomosUniSpanX4`) instead of `"anime"` (`AnimeVideoV3`), AND `PIPELINE_VERSION = 'hybrid-span-gfpgan-v25'` caused stale blurry/haloing IndexedDB entries to persist across upgrades (`media_1795615430749_390fa526.jpg`).

## Acceptance Criteria
1. `.lightbox-tool-btn` (including `.lightbox-enhance-toggle`, `.lightbox-zoom-btn`, `.lightbox-rotate`), `.lightbox-download`, `.lightbox-close`, and `.lightbox-zoom-readout` have zero background circle or scale effect on `:hover`, `:active`, `.is-active`, or `:focus-visible`.
2. Both zoomed anime crops (`crop_orig.png`) and full anime illustrations with outlines (`luffy_full_banner.png`) route to `AnimeVideoV3` (`"anime"`), while real portraits (`e1cddb86.jpg`, `outline_r = 0.0037`) continue routing to `NomosUniSpanX2`.
3. Bump `PIPELINE_VERSION` to `'hybrid-span-gfpgan-v28'` so stale `v25` IndexedDB AI outputs automatically recompute cleanly.
