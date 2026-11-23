# Spec 099 Implementation Plan: Clean Header Layout, Tight Subtitle Spacing & Frameless Hover

## 1. Details Panel Clean-Up (`app/src/components/details_panel.rs`)
- Remove the "Search in chat" (`div.details-section`) section from `DmDetailsPanel` (around line 220).
- Remove the "Instant In-Chat Search" (`div.details-section`) section from `SelfDetailsPanel` (around line 1106).
- Clean up any unused signals if `search` was only used there.

## 2. Header Layout & Device Count Removal (`app/src/pages/chat.rs`)
- At line 4367: Change `ShieldIcon { size: 12 } "Only you · {sessions_now.len()} devices"` to:
  `ShieldIcon { size: 12 } "Only you"` (translates to `🛡 Chỉ mình bạn`).

## 3. CSS Refinements (`app/assets/main.css`)
- **Header Actions Positioning**:
  - Add `margin-left: auto !important;` to `.chat-header-actions` so actions are always anchored to the far right.
  - Remove/override `.chat-header-identity.self-header-identity { margin-right: 0 !important; }`.
- **Tight Subtitle Spacing**:
  - Update `.chat-header-info`:
    ```css
    .chat-header-info {
      display: flex;
      flex-direction: column;
      min-width: 0;
      line-height: 1.15;
      gap: 1px;
    }
    .chat-header-title {
      margin: 0 !important;
      line-height: 1.15 !important;
    }
    .chat-header-meta {
      margin: 0 !important;
      line-height: 1.15 !important;
    }
    ```
- **Frameless Hover**:
  - Remove hover background scrim on header identity and buttons:
    ```css
    button.chat-header-identity:hover,
    .chat-column > .chat-header button.chat-header-identity:hover,
    .chat-column:has(> .chat-messages) > .chat-header button.chat-header-identity:hover,
    .chat-header .chat-header-actions .icon-btn:hover,
    .chat-column:has(> .chat-messages) > .chat-header .chat-header-actions .icon-btn:hover {
      background: transparent !important;
      box-shadow: none !important;
      border: none !important;
      outline: none !important;
    }
    .chat-header .chat-header-actions .icon-btn:hover {
      opacity: 0.75;
    }
    ```

## 4. Verification & Testing
- Compile WebAssembly release bundle: `./app/build-web.sh release`.
- Write Playwright E2E test `app/e2e/spec099-clean-header-and-hover.mjs` verifying:
  1. No search input exists in details panel.
  2. `.chat-header-actions` is positioned on the far right (x-position near right edge).
  3. Header text displays `Chỉ mình bạn` without `thiết bị`.
  4. Header identity and actions have `background-color: rgba(0, 0, 0, 0)` / `transparent` on hover.
  5. Distance between title and subtitle is <= 2px.
  6. Capture verification screenshots into `verify_099/`.
- Git commit and push to `origin main`.
