# Implementation Plan — Spec 014

## Constitution & Standards Check
- Pure Rust / Dioxus `0.7` in `app/`, Pure CSS minimalist design (`main.css`, `theme.css`).
- No `...` UI placeholders, no mock APIs, full `AGENTS.md` compliance.
- References Open Source patterns:
  - **Obsidian / Vencord CSS Variable Architecture**: derive complete semantic dark/light token set (`--bg-input`, `--bg-active`, `--bg-warning`, `--border`, `--on-accent`, `color-scheme`) from base theme colors so dark plugin themes never inherit light-mode white input/tab/composer backgrounds.
  - **Obsidian Community Plugin Lifecycle**: explicit `installed` state (`Tải` / `Gỡ`) separated cleanly from active toggle (`enabled`).

## Technical Changes

### 1. [app/assets/theme.css](file:///run/media/ngson/DATA1/DATA/source/chatw/app/assets/theme.css)
- Add `--avatar-header: 2.375rem;` (`2.125rem` for `compact`, `2.5rem` for `spacious`) to synchronize all header avatars across DM, Group, SelfChat, Bot/Plugin, and Custom Pages.

### 2. [app/assets/main.css](file:///run/media/ngson/DATA1/DATA/source/chatw/app/assets/main.css)
- **Switch (`.switch`)**: Exclude `checkbox`, `radio`, `range`, `color` from generic `input:focus`; explicitly reset `border`, `outline`, `box-shadow` on `input[type="checkbox"].switch`, center the knob (`1.5rem` height, `1.125rem` thumb, `0.1875rem` offset, `translateX(1.25rem)`), and use a clean `focus-visible` glow without concentric rings.
- **Chat Header & Avatars**:
  - Give `.chat-header` balanced `min-height: var(--header-height); height: auto; padding: var(--space-2) var(--space-4);` and clean `.chat-header-identity` padding (`var(--space-1) var(--space-2)`).
  - Enforce uniform `--avatar-header` sizing on `.chat-header .user-avatar`, `.chat-header .self-chat-avatar`, and `.chat-header .bot-header-icon`.
  - Keep `.chat-header .avatar-plugin-badge` compact inside the header slot.
- **Message Row & Avatar Slot**:
  - Increase `.message-row` gap to `var(--space-3)` and `.message-avatar-slot` width to `2.25rem` (`2rem` on mobile) with 100%-fit `.user-avatar` so message bubbles never touch avatars.
- **Plugin HUD & Store Actions**:
  - Shift `.plugin-floating-hud.pos-top-right` / `.pos-bottom-right` inside chat column when `.workspace-shell:has(.details-panel, .partner-info-sidebar)` is open so HUD never overlaps user banners/avatars.
  - Style `.mod-section-heading` and `.mod-store-empty` cleanly.

### 3. [app/src/components/messages.rs](file:///run/media/ngson/DATA1/DATA/source/chatw/app/src/components/messages.rs) & [app/src/pages/chat.rs](file:///run/media/ngson/DATA1/DATA/source/chatw/app/src/pages/chat.rs)
- Standardize message `Avatar { size: 36 }` matching `.message-avatar-slot`.
- Keep DM/Group header `Avatar` size consistent (`38`).

### 4. [app/src/components/plugins_view.rs](file:///run/media/ngson/DATA1/DATA/source/chatw/app/src/components/plugins_view.rs)
- **Plugin header cleanup**: Remove `span.bot-header-icon` (`PluginIcon`) and `span.chat-header-meta` (`"Tùy chỉnh giao diện, ứng dụng bên trái và lệnh"`) from `PluginCustomPageView` when `plugin_id == "studio"`.
- **Full-bleed SVG logos**: In `plugin_logo_data_url`, render edge-to-edge fill (`rx="0"` or no inset gap) so circular avatars show uniform color fill.
- **Complete Dark/Light Theme Token Engine (`sync_plugins_to_dom`)**:
  - Parse hex color luminance (`hex_luminance`) for `bg_chat`/`bg_app`/`accent`.
  - Output complete semantic tokens (`color-scheme`, `--bg-primary`, `--bg-input`, `--bg-input-focus`, `--bg-active`, `--bg-selected`, `--text-selected`, `--text-link`, `--border`, `--border-subtle`, `--border-strong`, `--bg-warning`, `--bg-error`, `--bg-success`, `--bg-overlay`, `--status-warning`, `--on-accent`, `--msg-sent-text`, `--msg-sent-meta`, `--msg-recv-meta`) so dark themes (Voxel, Cyber Neon, custom) style ALL inputs, `.segmented` tabs, `.composer-box`, search inputs, and `.e2ee-notice` cleanly with readable contrast.
- **Install (`Tải`) & Uninstall (`Gỡ`) Store Lifecycle**:
  - Add `#[serde(default = "default_true")] pub installed: bool` to `PluginMod`.
  - Default catalog: keep active/core tools installed (`Sổ tay & Việc làm`, `Trợ lý lệnh nhanh`, plus any currently enabled plugins during one-time migration `chatw_plugin_store_v1`) and mark disabled/extra catalog plugins (`Cyber Neon`, `Sakura`, etc.) uninstalled ready for `Tải`.
  - Filter `sync_plugins_to_dom`, `PluginGlobalOverlay`, `enabled_left_apps`, `enabled_rail_pages`, `enabled_slash_commands`, `enabled_composer_buttons`, `enabled_bot_plugins_as_items` by `p.installed && p.enabled`.
  - Render **Đã tải** section (with **`Gỡ`** button per card) and **Kho Plugin** section (with **`Tải`** button per card).
  - Uninstalling (`Gỡ`) disables the plugin, closes any active editor/page/left-app for it, clears custom theme CSS if needed, and moves built-in plugins back to Kho Plugin (or deletes user-imported plugins with a status confirmation / moves them out cleanly).
