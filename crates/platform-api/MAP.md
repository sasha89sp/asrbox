# MAP.md — `crates/platform-api`

Контракты платформенного слоя без `unsafe` и зависимостей от WinAPI.

## Файлы
- `src/lib.rs` — геометрические типы (`PixelPoint`, `PixelRect`), время (`LocalDateTime`), буферы (`RgbaImage`, `BgraFrame`, `WidgetFrame`), события (`PlatformEvent`), HTTP (`HttpFetchResult`) и трейты `IconProvider`, `DockPlatform`, `WidgetPlatform`.
- `src/logo.rs` — программный рендер фирменного логотипа «Стеклянный блок ASR» (`draw_asr_glass_logo`, `sample_asr_glyph`) для UI-модулей.
- `src/menu.rs` — типы контекстных меню (`DockMenuState`, `DockMenuAction`, `WidgetLayer`, `WidgetMenuState`, `WidgetMenuAction`, `TrayMenuEntry`, `TrayModuleFlags`, `TrayAction`).
- `src/menu_cfg.rs` — настраиваемая видимость пунктов контекстных меню трея, док-панелей и виджетов (`ContextMenuConfig`, `menus.cfg`).
- `src/palette.rs` — полная палитра тем `AsrGlass` и `Win11Classic` (`FluentPalette` Light/Dark), поканальная интерполяция `lerp`/`lerp_rgba` и тесты.
- `src/telemetry.rs` — DTO системного мониторинга (`SysTelemetry`, `CpuTelemetry`, `GpuTelemetry`, `RamTelemetry`, `DriveTelemetry`, `NetTelemetry`, `ProcUsage`, `OverlayTelemetry`) и ограничения ресайза виджетов (`WidgetSizeBounds`).
- `src/theme.rs` — настройки оформления (`DesignTheme`, `ThemeMode`, `AccentPreset`, `AppLanguage`, `FontRenderer`, `AppearanceConfig`, `appearance.cfg`) и реэкспорт `FluentPalette`/`lerp_rgba`.
- `src/tools.rs` — `ProcessMetrics`, `DebugUiCommand`, `AboutUiCommand`, `SettingsUiEvent`, `SettingsInputRects`, `NotesUiEvent`, `NotesInputRects`, `GitHubFetchResult`, `FluentMenuSpec`, `WebDavResult` и трейт `ToolsPlatform`.
