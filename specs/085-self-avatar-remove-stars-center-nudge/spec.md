# Spec 085 — Refine "Note to self" Pride Mascot SVG: Remove Sparkle Stars & Shift Mascot +1px Right

## Problem Statement
User requested refining the Spec 082 "Chính mình" ("Note to self") Pride mascot avatar:
1. Remove the two floating 4-point sparkle stars flanking the mascot.
2. Shift the center mascot figure slightly right by `+1px` (`+2.5` SVG units in the `128×128` viewBox ≈ `1.0px` at `51px` sidebar size) to balance the bottom-left speech-bubble tail optically at dead center.

## Acceptance Criteria
1. Both `app/public/self-chat-avatar.svg` and inline `SelfChatPrideAvatar` in `app/src/components/icons.rs` remove both sparkle stars and wrap the mascot figure in `transform="translate(2.5, 0)"`.
2. Updated Playwright E2E (`app/e2e/spec081-self-chat-pride-svg-avatar.mjs`) passes 5/5 checks and visual inspection confirms a crisp, star-free, optically centered mascot in both sidebar and header.
3. Rust unit tests pass and changes are committed & pushed to `origin main`.
