# MAP: crates/mod-settings

> Windows 11 Fluent Design (Light/Dark + 6 accents) resizable unified Settings UI with browser-style tabs.
> Pure Rust layout & software BGRA + ClearType renderer; depends only on `core` and `platform-api`.

## Files
- `src/lib.rs` — `SettingsUiState`: window state, browser tab management, theme transition, and frame rendering entry point.
- `src/events.rs` — Pointer, slider drag, scroll wheel, text input, and browser tab click dispatch emitting `SettingsAction`.
- `src/tests.rs` — Unit tests for layout, hit-testing, browser tabs, General Settings, and theme rendering.
- `src/types.rs` — DTOs (`SettingsTab`, `BrowserTab`, `SettingsSnapshot`, `WebDavServerView`, `FileSyncView`, `SettingsAction`).
- `src/i18n.rs` — Instant RU/EN localization strings (`tr`) for settings labels, tabs, and actions.
- `src/layout.rs` — Top browser tab bar, responsive Fluent sidebar, and `LayoutBuilder` construction.
- `src/layout_types.rs` — Core layout geometry constants, control enums (`Control`, `ControlKind`, `ButtonStyle`), and hit-test helpers.
- `src/layout_general.rs` — Unified «Общие настройки» builder (1. Theme/Accent, 2. Language/Autostart, 3. Backup/WebDAV/Sync, 4. Diagnostics, 5. Version History, 6. Paths).
- `src/layout_browser_tabs.rs` — Browser tab views inside the Settings window: Live Diagnostics Monitor, Version History (`CHANGELOG.md`), and WebDAV Folder Browser.
- `src/layout_tabs.rs` — «Модули» tab builder (isolation mode, module toggles, PID/restarts).
- `src/layout_dock.rs` — Per-dock independent settings tab (edge, layer, 4 label modes, sliders, shortcut chips + rename/reset).
- `src/layout_cal.rs` — Calendar settings tab builder (`.ics` source list, sync interval, display/size controls).
- `src/layout_sys.rs` — System Monitor settings tab builder (refresh interval presets, opacity, layer, size sliders).
- `src/layout_overlay.rs` — Game Overlay settings tab builder (auto-games, FSO/Topmost, setup mode, corner/custom pos, opacity, size, refresh).
- `src/layout_backup.rs` — Backup & saved WebDAV servers list + remote path + «Открыть папку →» builder.
- `src/layout_sync.rs` — Inline WebDAV new-server draft card and Windows file/folder sync card builder.
- `src/draw.rs` — 2D BGRA Fluent drawing primitives (rounded cards, borders, pill toggles, sliders, chips, accent bars).
- `src/render.rs` — Top-level renderer turning `SettingsLayout` into `(WidgetFrame, SettingsInputRects)`.
- `src/render_controls.rs` — Per-control Fluent renderer (`draw_control`, toggles, sliders, segmented chips, inputs, buttons).
