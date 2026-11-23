# MAP: crates/mod-settings

> macOS NotePlan-style resizable unified Settings UI grouped by modules.
> Pure Rust layout & software BGRA + ClearType renderer; depends only on `core` and `platform-api`.

## Files
- `src/lib.rs` — `SettingsUiState`: pointer/slider/wheel/resize state machine emitting `SettingsAction`.
- `src/types.rs` — DTOs (`SettingsTab`, `SettingsSnapshot`, `DockSettingsView`, `CalendarSettingsView`, `BackupSettingsView`, `WebDavSettingsView`, `SettingsAction`).
- `src/layout.rs` — Responsive macOS NotePlan sidebar + `LayoutBuilder` core types and hit rects.
- `src/layout_tabs.rs` — Per-tab card/control builders (`Modules` + autostart, `Dock` + layer, `Debug`, `About`).
- `src/layout_cal.rs` — Calendar settings tab builder (`.ics` source list, sync interval, display/size controls).
- `src/layout_sys.rs` — System Monitor settings tab builder (refresh interval presets, opacity, layer, size sliders).
- `src/layout_backup.rs` — Backup & Cloud WebDAV (Yandex.Disk / Mail.ru / Custom) settings tab builder.
- `src/draw.rs` — 2D BGRA drawing primitives (rounded cards, borders, NotePlan toggles, sliders, pills).
- `src/render.rs` — Renders `SettingsLayout` into `(WidgetFrame, SettingsInputRects)`.
