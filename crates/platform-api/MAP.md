# MAP.md — `crates/platform-api`

Контракты платформенного слоя без `unsafe` и зависимостей от WinAPI.

## Файлы
- `src/lib.rs` — геометрические типы (`PixelPoint`, `PixelRect`), время (`LocalDateTime`), буферы (`RgbaImage`, `BgraFrame`, `WidgetFrame`), события (`PlatformEvent`), HTTP (`HttpFetchResult`) и трейты `IconProvider`, `DockPlatform`, `WidgetPlatform`.
- `src/menu.rs` — типы контекстных меню (`DockMenuState`, `DockMenuAction`, `WidgetLayer`, `WidgetMenuState`, `WidgetMenuAction`, `TrayMenuEntry`, `TrayModuleFlags`, `TrayAction`).
- `src/palette.rs` — полная палитра Windows 11 Fluent Design (`FluentPalette` Light/Dark), поканальная интерполяция `lerp`/`lerp_rgba` и тесты.
- `src/telemetry.rs` — DTO системного мониторинга (`SysTelemetry`, `CpuTelemetry`, `GpuTelemetry`, `RamTelemetry`, `DriveTelemetry`, `NetTelemetry`, `ProcUsage`, `OverlayTelemetry`) и ограничения ресайза виджетов (`WidgetSizeBounds`).
- `src/theme.rs` — настройки оформления (`ThemeMode`, `AccentPreset`, `AppLanguage`, `AppearanceConfig`, `appearance.cfg`) и реэкспорт `FluentPalette`/`lerp_rgba`.
- `src/tools.rs` — `ProcessMetrics`, `DebugUiCommand`, `AboutUiCommand`, `SettingsUiEvent`, `SettingsInputRects`, `FluentMenuSpec`, `FluentMenuItem`, `FluentQuickBtn`, `WebDavResult`, `WebDavDirEntry` и трейт `ToolsPlatform`.
