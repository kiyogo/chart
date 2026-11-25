# Feature Specification: Sub-3ms Ultra-Fast Runtime & Zero-Latency UI Architecture (Spec 166)

**Feature Branch**: `main`  
**Created**: 2026-11-26  
**Status**: In Progress

## 1. Goal
Eliminate every measurable CPU, DOM, crypto, IPC, and network bottleneck across ChatW so that:
1. **Opening & Closing Settings** executes in **< 3 ms** (DOM commit + layout/paint) with zero full-app teardown/remount.
2. **Switching Settings tabs & sub-tabs** executes in **< 3 ms** via `onpointerdown` dispatch and non-blocking deferred background refreshes.
3. **Opening / switching Chat conversations (DM, Group, Channel)** executes in **< 3 ms** when warm in L0 RAM (including the **very first click** after boot via idle top-N pre-warming + hover speculative prefetch!), with **zero** skeleton flash, **zero** redundant E2EE re-decryption of already-decrypted messages, **zero** IndexedDB draft blocking, and direct `web_sys` scrolling without `dioxus::document::eval` overhead.
4. **Boot, Render & Transport** achieve hardware-bound efficiency: deferred non-critical scripts, zero-recompression warm WASM cache in IndexedDB, 32-bit XOR CWRT1 unpack, CSS `contain` + `content-visibility` without `:has()` invalidation storms, memoized `AppRail` QR generation, backend `TCP_NODELAY`, and immutable caching for `?v=` versioned static bundles.

## 2. Non-Negotiable Constraints
- Preserve 100% of E2EE security invariants (decrypted plaintext L0 cache stays strictly in volatile RAM and is immediately wiped on lock/logout/key-rotation).
- Preserve HTTP Request for sending & WebSocket for receiving (Spec 165).
- Preserve all existing visual design, UI controls, keyboard accessibility, and CWRT1 / CWBF2 protections.
- Measure REAL click-to-DOM and click-to-paint latency honestly in a live browser — never fabricate timings.
