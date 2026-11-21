# Feature Specification: Edge-to-Edge Horizontal Scroll for Sidebar Avatar/Notes Strip

**Feature Branch**: `034-notes-strip-edge-to-edge-scroll`
**Created**: 2026-11-21
**Status**: In Progress

## Problem Statement
When horizontally dragging or scrolling the `.notes-strip` avatar bar inside `.sidebar-scrollable`, `.sidebar-scrollable`'s horizontal padding (`0.45rem` / `6.75px`) clips the avatars `6.75px` inside the sidebar edge, producing an unintended white vertical band along the left/right border of the white `.nav-sidebar` frame.

## Acceptance Criteria
- **AC1**: `.notes-strip-wrap` and `.notes-strip` extend full-bleed edge-to-edge across `.nav-sidebar` (`stripRect.left === sidebarRect.left` and `stripRect.right === sidebarRect.right`, 0px horizontal clip inset).
- **AC2**: At rest (`scrollLeft === 0`), the first avatar maintains balanced horizontal breathing room (`padding-inline: 0.9rem`, aligned with `.chat-item` avatar below it).
- **AC3**: While dragging/scrolling horizontally, avatars glide all the way to touch the `.nav-sidebar` frame edge with **0px** white clipping strip on both left and right edges, and `.sidebar-scrollable` never produces a horizontal scrollbar.
