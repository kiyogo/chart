# Plan 132: Theme-Adaptive Message Bubble Colors & High-Contrast Borders

## Architecture & Technical Plan

### 1. Refactor Base Message Bubble Rules (`app/assets/main.css:1738`)
- Remove `!important` from `#FFFFFF` in `.message-row.received .message-bubble`.
- Use dynamic token `var(--msg-recv, #FFFFFF)`:
  ```css
  .message-row.received .message-bubble {
    background: var(--msg-recv, #FFFFFF);
    color: var(--msg-recv-text, #111827);
    border: 1px solid rgba(15, 23, 42, 0.09);
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.05);
  }
  ```

### 2. Refactor Error Bubble Rules (`app/assets/main.css:1763-1782`)
- Replace the monolithic hardcoded `#FFFFFF !important` rules with theme-aware defaults:
  ```css
  .message-bubble.error-bubble,
  .message-row.sent .message-bubble.error-bubble,
  .message-row.received .message-bubble.error-bubble {
    background: var(--msg-recv, #FFFFFF);
    color: var(--msg-recv-text, #111827);
    font-size: var(--text-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    border: 1px solid rgba(15, 23, 42, 0.10);
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.05);
  }
  .message-bubble.error-bubble *,
  .message-row.sent .message-bubble.error-bubble *,
  .message-row.received .message-bubble.error-bubble * {
    color: inherit;
  }
  .message-row.sent .message-bubble.error-bubble .bubble-meta,
  .message-row.received .message-bubble.error-bubble .bubble-meta {
    color: var(--msg-recv-meta, #64748B);
  }
  ```

### 3. Comprehensive Dark Mode & Dark Wallpaper Overrides (`app/assets/main.css:4270`)
- Extend the dark surface rules to cover all dark contexts:
  - `html[data-theme="dark"]`
  - `html[data-surface-tone="dark"]`
  - `.chat-column.wp-night`
  - `.chat-column[data-chat-tone="dark"]`
  - `.settings-preview-chat.wp-night`
  - `html[data-theme="dark"] .settings-preview-chat`
- For received bubbles:
  ```css
  html[data-theme="dark"] .message-row.received .message-bubble,
  html[data-surface-tone="dark"] .message-row.received .message-bubble,
  .chat-column.wp-night .message-row.received .message-bubble,
  .chat-column[data-chat-tone="dark"] .message-row.received .message-bubble,
  .settings-preview-chat.wp-night .message-row.received .message-bubble,
  html[data-theme="dark"] .settings-preview-chat .message-row.received .message-bubble {
    background: var(--msg-recv, rgba(30, 39, 60, 0.90)) !important;
    color: var(--msg-recv-text, #F1F4FA) !important;
    border: 1px solid rgba(255, 255, 255, 0.09) !important;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25) !important;
  }
  ```
- For error bubbles in dark mode:
  ```css
  html[data-theme="dark"] .message-bubble.error-bubble,
  html[data-surface-tone="dark"] .message-bubble.error-bubble,
  .chat-column.wp-night .message-bubble.error-bubble,
  .chat-column[data-chat-tone="dark"] .message-bubble.error-bubble,
  .settings-preview-chat.wp-night .message-bubble.error-bubble,
  html[data-theme="dark"] .settings-preview-chat .message-bubble.error-bubble {
    background: var(--msg-recv, rgba(30, 39, 60, 0.90)) !important;
    color: var(--msg-recv-text, #F1F4FA) !important;
    border: 1px solid rgba(255, 255, 255, 0.12) !important;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25) !important;
  }
  html[data-theme="dark"] .message-bubble.error-bubble *,
  html[data-surface-tone="dark"] .message-bubble.error-bubble *,
  .chat-column.wp-night .message-bubble.error-bubble *,
  .chat-column[data-chat-tone="dark"] .message-bubble.error-bubble *,
  .settings-preview-chat.wp-night .message-bubble.error-bubble *,
  html[data-theme="dark"] .settings-preview-chat .message-bubble.error-bubble * {
    color: inherit !important;
  }
  html[data-theme="dark"] .message-bubble.error-bubble .bubble-meta,
  html[data-surface-tone="dark"] .message-bubble.error-bubble .bubble-meta,
  .chat-column.wp-night .message-bubble.error-bubble .bubble-meta,
  .chat-column[data-chat-tone="dark"] .message-bubble.error-bubble .bubble-meta,
  .settings-preview-chat.wp-night .message-bubble.error-bubble .bubble-meta,
  html[data-theme="dark"] .settings-preview-chat .message-bubble.error-bubble .bubble-meta {
    color: var(--msg-recv-meta, rgba(241, 244, 250, 0.74)) !important;
  }
  ```

### 4. Build, E2E Test & Verification
- Compile web bundle: `./app/build-web.sh`.
- Create Playwright E2E test `app/e2e/spec132-message-bubble-colors.mjs`:
  - Launch app with Dark mode: assert received & error bubbles have dark background and light text.
  - Switch to Light mode on white canvas: assert received & error bubbles have 1px visible border and proper contrast.
  - Save screenshots in artifact directory.
