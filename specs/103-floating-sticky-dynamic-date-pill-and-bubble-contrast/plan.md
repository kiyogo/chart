# Spec 103 Plan: Architecture & Implementation Strategy

## 1. Component Architecture (`app/src/components/messages.rs`)
- Compute `day_groups: Vec<(Option<NaiveDate>, Vec<usize>)>` from parsed timestamps.
- Wrap each day's messages in a `.date-group` container:
  ```rust
  div {
      key: "dg-{g_idx}",
      class: "date-group",
      if let Some(d) = day_opt {
          div { class: "date-separator", span { "{day_label(*d)}" } }
      }
      for &i in msg_indices.iter() {
          // message rows
      }
  }
  ```

## 2. CSS Architecture (`app/assets/main.css`)
- Style `.date-group`:
  ```css
  .date-group {
    display: flex;
    flex-direction: column;
    gap: var(--message-gap);
    position: relative;
    width: 100%;
  }
  ```
- Restore `position: sticky !important; top: 8px !important; z-index: 5;` on `.date-separator`.
- Replace legacy override at line 10409 (`position: relative !important;`).
- Enforce contrast rules on `.error-bubble`:
  ```css
  .message-bubble.error-bubble *,
  .message-row.sent .message-bubble.error-bubble *,
  .message-row.received .message-bubble.error-bubble * {
    color: #111827 !important;
  }
  .message-row.sent .message-bubble.error-bubble .bubble-meta,
  .message-row.received .message-bubble.error-bubble .bubble-meta {
    color: #64748B !important;
  }
  ```
- Adjust `.scroll-to-bottom-btn`:
  ```css
  .scroll-to-bottom-btn {
    position: sticky;
    bottom: 20px;
    margin-top: -38px;
    margin-bottom: 4px;
    margin-right: 14px;
  }
  ```

## 3. Verification & Quality Assurance
- Run automated Playwright script `app/e2e/spec103-floating-sticky-date-and-bubble-contrast.mjs`.
- Check all 4 audits:
  1. Sticky styles and position on `.date-separator`.
  2. Dynamic sticking and replacement when scrolling across multiple days.
  3. Text and metadata contrast on `.error-bubble`.
  4. Scroll button placement and layout.
