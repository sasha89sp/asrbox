# MAP: `crates/mod-dock`

Неограниченное количество независимых док-панелей в стиле macOS внутри одного процесса-модуля без лишнего расхода ОЗУ. Зависит только от `core` и `platform-api`. Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | Публичный API и реэкспорты крейта `asrbox-mod-dock`. |
| `src/config.rs` | `DockRegistry` (неограниченное число панелей), `DockConfig`, `DockEdge`, `DockAlignment`, `IconSource`, `HoverZoomConfig` и валидация. |
| `src/layout.rs` | Геометрия панели у краёв экрана, центровка, hit-test, индекс вставки drag-and-drop и масштабирование только активной иконки. |
| `src/state.rs` | Событийный конечный автомат `DockInstance`, плавная анимация одной иконки с 0 тиков в покое и метрики `DockMetrics`. |
