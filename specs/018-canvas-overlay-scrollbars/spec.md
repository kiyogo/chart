# Spec 018: Canvas Overlay Scrollbars with Proximity & Scroll-Triggered Visibility

## 1. Context & User Problem
Currently, scrollbars across the application are using standard CSS selectors (`*:hover::-webkit-scrollbar-thumb`) which cause scrollbars to appear anytime the mouse cursor is anywhere inside a scrollable container. The user requested:
> "làm tất cả giao diện thanh kéo bằng canvas cho tôi đi di trỏ chuột gần vô nó sẽ hiện thanh kéo hoặc kéo mới hiện thanh kéo"

Key objectives:
1. Make all scrollbars use a smooth, floating Canvas overlay indicator that does not occupy layout space or disrupt content.
2. **Proximity-triggered visibility ("di trỏ chuột gần vô mới hiện")**: The scrollbar handle is completely invisible by default. It only reveals itself when the mouse cursor moves close to the scrollable edge (within ~24px).
3. **Scroll-triggered visibility ("kéo mới hiện")**: When scrolling (mouse wheel, touch gesture, keyboard navigation, or dragging), the canvas scrollbar instantly appears at full visibility, and smoothly fades away ~800ms after scrolling stops.
4. **Interactive Dragging**: Clicking or dragging the canvas scrollbar thumb scrolls the target container seamlessly with 1:1 precision.
5. **Universal Application**: Works seamlessly across all scrollable containers:
   - Message streams (`.chat-messages`, `.vico-chat-scroll`)
   - Navigation & sidebars (`.sidebar-scrollable`, `.friends-scroll`, `.server-members-scroll`, `.conversation-list`)
   - Enterprise workspaces (`.ent-content`, `.overview-scroll`)
   - Modals and drawers (`.details-scroll`, `.settings-body`)

## 2. Acceptance Criteria
- [ ] No visible scrollbar track or thumb while idling with cursor away from the scroll edge.
- [ ] Moving mouse within ~24px of the scroll edge causes the canvas scrollbar thumb to smoothly fade in.
- [ ] Moving mouse away causes the canvas scrollbar thumb to smoothly fade out.
- [ ] Any scroll event (wheel, touch, arrow keys, programmatic) causes the canvas scrollbar thumb to appear immediately and stay visible during active scrolling, then fade out after ~800ms of inactivity.
- [ ] Dragging the canvas thumb scrolls the content synchronously and keeps the thumb visible while dragging.
- [ ] Native CSS scrollbars are configured to be transparent/hidden by default to avoid double-scrollbars while retaining native keyboard & touchpad accessibility.
- [ ] Passes all unit and integration tests, compiles cleanly in release mode, and is pushed to `main`.
