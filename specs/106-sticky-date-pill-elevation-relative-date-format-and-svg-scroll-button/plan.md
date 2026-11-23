# Spec 106 Plan: Architecture & Implementation Strategy

## 1. Rust Component Architecture (`app/src/components/messages.rs`)
1. **Update `day_label` function**:
   ```rust
   fn day_label(date: NaiveDate) -> String {
       let today = Local::now().date_naive();
       if date == today {
           return "Today".to_string();
       }
       if let Some(yesterday) = today.pred_opt() {
           if date == yesterday {
               return "Yesterday".to_string();
           }
       }
       let month_name = ENGLISH_MONTHS[(date.month().saturating_sub(1) as usize).min(11)];
       if date.year() == today.year() {
           format!("{month_name} {}", date.day())
       } else {
           format!("{month_name} {}, {}", date.day(), date.year())
       }
   }
   ```
2. **Replace unicode `"↓"` with SVG down arrow icon**:
   ```rust
   svg {
       class: "scroll-down-svg",
       width: "18",
       height: "18",
       view_box: "0 0 24 24",
       fill: "none",
       stroke: "currentColor",
       stroke_width: "2.2",
       stroke_linecap: "round",
       stroke_linejoin: "round",
       "aria-hidden": "true",
       path { d: "M12 4v13.5" }
       path { d: "m18 12-6 6-6-6" }
   }
   ```

## 2. i18n Localization Architecture (`app/web/i18n_runtime.js` & `app/dist/i18n_runtime.js`)
1. **In `formatLocalizedDate(str, lang)`**:
   - Handle `"Today"` -> `"Hôm nay"` (or `Intl.RelativeTimeFormat(lang).format(0, 'day')`).
   - Handle `"Yesterday"` -> `"Hôm qua"` (or `Intl.RelativeTimeFormat(lang).format(-1, 'day')`).
   - For `lang === 'vi'` with full date `EN_DATE_FULL_RE` (`{month} {day}, {year}`):
     Format with padded day/month: `${day < 10 ? '0' + day : day} tháng ${mon < 10 ? '0' + mon : mon} năm ${year}` (yielding e.g. `12 tháng 05 năm 2025`).

## 3. CSS Layout Architecture (`app/assets/main.css`)
1. **Elevate `.date-separator`**:
   - In `.date-group > .date-separator, .date-separator`:
     Set `margin: 2px 0 6px !important;` (removing the bulky 16px `var(--space-4)` top margin).
     Set `top: calc(12px - (var(--header-height) + var(--space-5))) !important;` so it sticks elegantly at `top: 12px` near the top of the chat stage.
2. **Lower `.scroll-to-bottom-btn`**:
   - Adjust `bottom: -26px !important;` and `margin-top: -38px; margin-bottom: 0;` so it sits nicely right above the composer input box (~10px above it) instead of floating 60px high up in the middle of messages.

## 4. Verification & Testing
1. Update and run Playwright E2E test `app/e2e/spec105-date-formatting-and-svg-scroll-button.mjs`.
2. Verify all assertions for "Hôm nay", "Hôm qua", "12 tháng 05 năm 2025", sticky top offset, and SVG arrow rendering.
