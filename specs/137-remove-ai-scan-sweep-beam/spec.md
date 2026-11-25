# Feature Specification: Spec 137 — Remove AI Scan Sweep Beam

## 1. Problem Statement
The user requested:
- `"xóa cho tôi cái quét đi nhé"` (Remove/delete the scanning sweep beam for me!)
- Attached screenshot: `media_1795584839291_15845a55.png` showing the bright optical laser sweep bar across the bottom-right near the progress pill (`79%`).
- While the scan beam was refined in Spec 135, the user prefers a cleaner, calmer, non-distracting visual experience during media loading and AI enhancement without any laser beam or sweeping light bar slicing across images.

## 2. Goals & Acceptance Criteria
- [ ] **Completely Remove the Scan Sweep Beam**:
  - Remove all DOM instances of `div { class: "ai-gen-dots-wave" }` from `app/src/components/rich.rs` (in Chat message loading, rich-image blur placeholder, and Lightbox enhancement overlay).
  - Remove `.ai-gen-dots-wave`, `.ai-gen-dots-wave::after`, and `@keyframes chatwAiLaserScan` from `app/assets/main.css`.
- [ ] **Preserve Calming Backdrop Elements**:
  - Keep the soft frost blur (`.ai-gen-frost`), ambient aurora glow (`.ai-gen-aurora`), subtle dot matrix (`.ai-gen-dots-base`), and percentage readout pill (`.ai-gen-pct`).
  - Zero visual clutter, zero laser beam sweep slicing across photos.
- [ ] **Build & Verification**:
  - Web bundle compiles cleanly with `./app/build-web.sh`.
  - Playwright E2E verification test confirms that `.ai-gen-dots-wave` is absent from all image stages and overlays.
- [ ] **Mandatory Auto-Push (Rule 1)**:
  - Commit and push to GitHub `origin main` immediately upon completion.
