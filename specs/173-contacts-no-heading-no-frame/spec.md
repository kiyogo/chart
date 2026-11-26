# Spec 173: Remove Redundant Contacts Sub-Headings & Outer List Frame

## Problem
In Contacts (`Danh bạ`), the Friends List and Groups & Communities views rendered:
1. Redundant sub-headings (`Friends (N)` / `Groups and communities (N)`) right below the header title.
2. An outer rounded box/frame (`.contacts-surface-card.contacts-toolbar-card` and `.contacts-add-box` / `.contacts-empty-inline`) with borders wrapping the toolbar and list rows.

## Requirements
- Remove the `contacts-summary-bar` sub-headings (`Friends (...)`, `Groups and communities (...)`) in `app/src/components/contacts_view.rs`.
- Remove the outer box background, border, and border-radius from `.contacts-toolbar-card`, `.contacts-add-box`, and `.contacts-empty-inline` (including under `data-panel-scrim="true"`), and remove the horizontal divider under `.contacts-toolbar` so list rows sit directly on `.contacts-stage`.
