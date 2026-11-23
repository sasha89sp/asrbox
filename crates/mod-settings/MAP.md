# MAP: crates/mod-settings

> macOS-style resizable unified Settings UI grouped by modules.
> Pure Rust layout & software BGRA + ClearType renderer; depends only on `core` and `platform-api`.

## Files
- `src/lib.rs` — `SettingsUiState`: pointer/slider/wheel/resize state machine emitting `SettingsAction`.
- `src/types.rs` — DTOs (`SettingsTab`, `SettingsSnapshot`, `DockSettingsView`, `CalendarSettingsView`, `ModuleStatusView`, `SettingsAction`).
- `src/layout.rs` — Responsive macOS Ventura/Sonoma sidebar + `LayoutBuilder` core types and hit rects.
- `src/layout_tabs.rs` — Per-tab card/control builders (`Modules`, `Dock`, `Debug`, `About`).
- `src/layout_cal.rs` — Calendar settings tab builder (commented-URL recovery banner, `.ics` source list, display controls).
- `src/draw.rs` — 2D BGRA drawing primitives (rounded cards, borders, macOS toggles, sliders, pills).
- `src/render.rs` — Renders `SettingsLayout` + `SettingsSnapshot` into `(WidgetFrame, Option<PixelRect>)`.
