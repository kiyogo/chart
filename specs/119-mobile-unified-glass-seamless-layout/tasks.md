# Tasks: Spec 119 - Mobile Unified Frosted Glass & Seamless Bottom Bar Layout

- [x] Task 1: Update `app/assets/main.css` mobile styles
  - [x] 1.1: Align `.nav-sidebar` and `.app-rail` height to `calc(100dvh - 58px - env(safe-area-inset-bottom))` with 0 bottom padding bleeding.
  - [x] 1.2: Disable `.app-rail::before` on mobile (`display: none !important;`).
  - [x] 1.3: Unify `.app-rail` and `.nav-sidebar` frosted glass (`color-mix` with `var(--bg-surface)` and `blur(16px)`).
  - [x] 1.4: Normalize all left rail icons to `24×24px` in `40×40px` tap targets.
  - [x] 1.5: Set `.mobile-bottom-bar` to an elevated full-width frosted surface (`blur(20px) saturate(1.4)`) with no seam underneath.
- [x] Task 2: Update `app/src/components/navigation/rail.rs`
  - [x] 2.1: In Tab 5 ("Tài khoản"), render `UserIcon { size: 22 }` when no custom avatar URL is provided, eliminating the heavy purple letter dot.
- [x] Task 3: Build WebAssembly Release bundle
  - [x] 3.1: Run `./app/build-web.sh release`.
- [x] Task 4: Automated E2E verification
  - [x] 4.1: Run mobile layout audit on `390×844` and `360×800` checking bottom bar uniformity and icon sizes.
  - [x] 4.2: Capture screenshot to visually verify zero vertical seam cutting through "Tin nhắn".
- [x] Task 5: Auto-commit and git push to `origin main`
  - [x] 5.1: `git add .`, `git commit -m "fix(spec-119): ..."`, `git push origin main`.
