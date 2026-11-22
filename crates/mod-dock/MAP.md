# MAP.md — `crates/mod-dock`

Неограниченное количество независимых док-панелей в стиле macOS внутри одного процесса (`specs/mod-dock.md`).
Без `unsafe`, без `unwrap()`/`expect()`, без прямых вызовов WinAPI.

## Файлы
- `src/lib.rs` — публичный API крейта и реэкспорт типов.
- `src/config.rs` — `DockRegistry`, `DockConfig`, `DockItem`, `DockEdge`, `DockAlignment`, `IconSource`, `HoverZoomConfig` и валидация.
- `src/layout.rs` — расчёт геометрии капсулы, 4 краёв экрана, центровки, позиции вставки при drag-and-drop и одноиконочного увеличения.
- `src/render.rs` — безопасный 32-битный рендерер macOS-капсулы с альфа-тенью, кромкой и билинейным масштабированием иконок.
- `src/state.rs` — событийный автомат `DockInstance`, остановка таймера в покое и счётчики метрик.
- `src/storage.rs` — портативное сохранение и загрузка неограниченного числа док-панелей в `./data/config/docks.cfg`.
