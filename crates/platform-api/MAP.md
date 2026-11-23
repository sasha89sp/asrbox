# MAP.md — `crates/platform-api`

Контракты платформенного слоя без `unsafe` и зависимостей от WinAPI.

## Файлы
- `src/lib.rs` — геометрические типы (`PixelPoint`, `PixelRect`), время (`LocalDateTime`), буферы (`RgbaImage`, `BgraFrame`, `WidgetFrame`), события (`PlatformEvent`), меню (`DockMenuAction`, `WidgetMenuAction`, `TrayAction`), HTTP (`HttpFetchResult`) и трейты `IconProvider`, `DockPlatform`, `WidgetPlatform`.
- `src/tools.rs` — `ProcessMetrics`, `DebugUiCommand`, `AboutUiCommand` и трейт `ToolsPlatform` для окон диагностики, «О программе», буфера обмена и открытия папок/конфигов.
