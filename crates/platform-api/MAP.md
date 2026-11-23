# MAP.md — `crates/platform-api`

Контракты платформенного слоя без `unsafe` и зависимостей от WinAPI.

## Файлы
- `src/lib.rs` — геометрические типы (`PixelPoint`, `PixelRect`), время (`LocalDateTime`), буферы (`RgbaImage`, `BgraFrame`, `WidgetFrame`), события (`PlatformEvent`), меню (`DockMenuAction`, `WidgetMenuAction`, `TrayAction`), HTTP (`HttpFetchResult`) и трейты `IconProvider`, `DockPlatform`, `WidgetPlatform`.
