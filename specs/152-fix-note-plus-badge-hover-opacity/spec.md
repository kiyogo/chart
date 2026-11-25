# Feature Specification: Opaque Hover Background on `.note-plus-badge` (Spec 152)

**Feature Branch**: `main`  
**Created**: 2026-11-25  
**Status**: In Progress

## Problem Statement

Hovering `.note-plus-badge` (`+` button overlaid on the `"Create story"` / `"Tạo tin"` avatar circle in the notes strip) switches `background` from `var(--bg-elevated)` to `var(--bg-hover)`, which is a semi-transparent RGBA token. Because `.note-plus-badge` overlaps the colored avatar circle and the sidebar background, the purple avatar bleeds half-way through the button on hover.

## Acceptance Criteria

1. `.note-plus-badge:hover` renders an opaque background (`color-mix(in srgb, var(--bg-elevated) 90%, var(--text-primary))`) so the underlying avatar never bleeds through in light or dark themes.
2. Live Chrome hover check confirms computed `backgroundColor` has alpha `1` (opaque) and screenshot shows clean circular `+` hover state.
