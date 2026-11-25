# Technical Plan: Spec 137 — Remove AI Scan Sweep Beam

## 1. Technical Changes
1. **DOM Structure (`app/src/components/rich.rs`)**:
   - In `RichImage` loaded stage (around line 971):
     Remove `div { class: "ai-gen-dots-wave" }`.
   - In `RichImage` placeholder stage (around line 1045):
     Remove `div { class: "ai-gen-dots-wave", aria_hidden: "true" }`.
   - In `Lightbox` enhancement overlay (around line 2902):
     Remove `div { class: "ai-gen-dots-wave" }`.

2. **CSS Cleanup (`app/assets/main.css`)**:
   - Remove or neutralize `.ai-gen-dots-wave` and `.ai-gen-dots-wave::after` rules.
   - Remove `@keyframes chatwAiLaserScan`.
   - Remove `.ai-gen-dots-wave` mentions in `@media (prefers-reduced-motion)` and `data-reduce-motion`.

3. **E2E Verification (`app/e2e/spec137-remove-ai-scan-sweep.mjs`)**:
   - Test chat and Lightbox image enhancement overlays.
   - Assert `.ai-gen-dots-wave` is completely removed from the DOM.
   - Verify remaining elements (`.ai-gen-frost`, `.ai-gen-aurora`, `.ai-gen-pct`) render cleanly.
   - Take screenshot of Lightbox loading state to prove no sweep beam is present.
