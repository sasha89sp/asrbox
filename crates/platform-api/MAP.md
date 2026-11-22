# MAP.md — `crates/platform-api`

Контракты платформенного слоя без `unsafe` и зависимостей от WinAPI.

## Файлы
- `src/lib.rs` — геометрические типы (`PixelPoint`, `PixelRect`), буферы (`RgbaImage`, `BgraFrame`), события (`PlatformEvent`), меню (`DockMenuAction`, `TrayAction`) и трейты `IconProvider`, `DockPlatform`.
