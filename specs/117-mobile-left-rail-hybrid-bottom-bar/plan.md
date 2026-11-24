# Spec 117 Technical Plan

## Files to Modify
1. **`app/src/components/navigation/rail.rs`**:
   - Add class `rail-mobile-in-bottom` to the 5 items that live in `.mobile-bottom-bar` on mobile (`Messages`, `Friends`, `Search`, `Notifications`, `rail-account`), so they stay in `.app-rail` on desktop (`>= 768px`) and move cleanly into `.mobile-bottom-bar` on mobile (`<= 767px`) with zero duplication.
   - Render a sibling `<nav class="mobile-bottom-bar ...">` inside `AppRail` (hidden on desktop via CSS, hidden on mobile when `hidden_mobile == true`) containing the 5 core tabs (`Tin nhắn`, `Bạn bè`, `Tìm kiếm`, `Thông báo`, `Tôi` avatar with unread/pending badges) and mobile account bottom-sheet menu.
2. **`app/assets/main.css`**:
   - On desktop (`>= 768px`): `.mobile-bottom-bar { display: none; }`.
   - On mobile (`<= 767px`):
     - Restore `.workspace-shell { flex-direction: row; }`.
     - Style `.app-rail` as a crisp `56px`-wide **LEFT** vertical rail (`padding: max(10px, env(safe-area-inset-top)) 0 calc(66px + env(safe-area-inset-bottom))`), border-right hairline, hiding `.rail-mobile-in-bottom` and `.rail-download-wrap`, showing `.rail-pins` + feature icons + Settings cleanly!
     - Pad `.nav-sidebar` bottom by `calc(60px + env(safe-area-inset-bottom))` so conversation rows scroll cleanly above `.mobile-bottom-bar`.
     - Style `.mobile-bottom-bar` as a sleek fixed bottom bar (`left: 0; right: 0; bottom: 0; height: calc(58px + env(safe-area-inset-bottom)); display: grid; grid-template-columns: repeat(5, 1fr);`) with crisp icons, micro-labels (`10.5px`), active accent pill, and unread badges.
     - Polish `.details-panel .profile-close` (`display: inline-flex; align-items: center; justify-content: center; padding: 0;`) so the close `X` is dead-center.
3. **`app/e2e/spec116-mobile-ui-optimization.mjs`**:
   - Update assertions to verify the hybrid layout on `390×844` and `360×800`:
     - `.app-rail` is positioned on the **LEFT** (`left === 0`, `width <= 60`, `height > 400`, vertical column) with `Cộng đồng`, `Họp nhóm`, `Mới`, `Vico AI`, `Trình cắm`, `Cài đặt`.
     - `.mobile-bottom-bar` is positioned at the **BOTTOM** (`bottom === vh`, `width === vw`, 5 core buttons `Tin nhắn`, `Bạn bè`, `Tìm kiếm`, `Thông báo`, `Tôi`).
     - Zero duplication between left rail & bottom bar; both hide cleanly inside active DM chat.
