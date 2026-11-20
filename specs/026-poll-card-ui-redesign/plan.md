# Technical Plan (026)

1. **`app/src/components/rich.rs`**:
   - Import `BarChartIcon` from `super::icons`.
   - Render `.poll-card` header with `BarChartIcon { size: 14 }` and sentence-case `"Bình chọn"`.
   - Inside each `.poll-option`:
     - Keep `.poll-bar` (`width: {pct}%`).
     - Render `<span class="poll-check" aria-hidden="true">` containing `CheckIcon { size: 11 }` when `mine == Some(i)`.
     - Render `<span class="poll-label">{opt}</span>` cleanly inline.
     - Render `<span class="poll-stats">`: when `votes.len() > 0`, show `<span class="poll-pct">{pct}%</span><span class="poll-count">{count}</span>`; when `0` votes, keep subtle `<span class="poll-count">0</span>`.
     - Footer shows `"Chưa có lượt bình chọn"` when `0`, or `"{votes.len()} lượt bình chọn"` when `> 0`.
2. **`app/assets/main.css`**:
   - Style `.poll-card`, `.poll-card .rich-kind` (sentence-case, no uppercase), `.poll-options`, `.poll-option` (clean flat background, subtle hover, NO thick 2px border on `.chosen`), `.poll-check` (`18px × 18px` circle), `.poll-label` (inline flex), `.poll-stats`, `.poll-pct`, `.poll-count`.
3. **Verification**:
   - Rebuild Web release (`./app/build-web.sh release`).
   - Run Playwright poll visual + functional check (`app/e2e/poll-card-check.mjs`), verify single-line layout, zero vertical icon stacking, no thick border, and take screenshots.
   - Run full workspace cargo tests.
