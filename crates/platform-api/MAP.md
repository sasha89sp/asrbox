# MAP.md — `crates/platform-api`

Контракты платформенного слоя без `unsafe` и зависимостей от WinAPI.

## Файлы
- `src/lib.rs` — геометрические типы (`PixelPoint`, `PixelRect`), время (`LocalDateTime`), буферы (`RgbaImage`, `BgraFrame`, `WidgetFrame`), события (`PlatformEvent`), HTTP (`HttpFetchResult`) и трейты `IconProvider`, `DockPlatform`, `WidgetPlatform`.
- `src/menu.rs` — типы контекстных меню (`DockMenuState`, `DockMenuAction`, `WidgetLayer`, `WidgetMenuState`, `WidgetMenuAction`, `TrayMenuEntry`, `TrayModuleFlags`, `TrayAction`).
- `src/telemetry.rs` — DTO системного мониторинга (`SysTelemetry`, `CpuTelemetry`, `GpuTelemetry`, `RamTelemetry`, `DriveTelemetry`, `NetTelemetry`, `ProcUsage`) и ограничения ресайза виджетов (`WidgetSizeBounds`).
- `src/tools.rs` — `ProcessMetrics`, `DebugUiCommand`, `AboutUiCommand` и трейт `ToolsPlatform` для окон диагностики, «О программе», буфера обмена и открытия папок/конфигов.
