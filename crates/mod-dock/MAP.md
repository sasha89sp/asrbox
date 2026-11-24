# MAP.md — `crates/mod-dock`

Неограниченное количество независимых док-панелей в стиле Windows 11 Fluent (Light/Dark) внутри одного процесса (`specs/mod-dock.md`).
Без `unsafe`, без `unwrap()`/`expect()`, без прямых вызовов WinAPI.

## Файлы
- `src/lib.rs` — публичный API крейта и реэкспорт типов.
- `src/config.rs` — `DockRegistry`, `DockConfig`, `DockItem`, `DockEdge`, `DockAlignment`, `IconSource`, `HoverZoomConfig` и валидация.
- `src/labels.rs` — 4 индивидуальных режима отображения имён (`DockLabelMode`), расчёт геометрии подписей/пилюль и `default_label_for_path`.
- `src/layout.rs` — расчёт геометрии капсулы с учётом `DockLabelMode`, 4 краёв экрана, центровки, ручки перетаскивания и одноиконочного увеличения.
- `src/layout_ops.rs` — позиция вставки при drag-and-drop (`drop_insert_index`) и привязка перемещённой панели к краям (`apply_dragged_window_pos`).
- `src/render.rs` — безопасный 32-битный рендерер Fluent-капсулы (`render_dock_widget_frame` с `WidgetFrame`, темой и подписями/пилюлями).
- `src/render_marks.rs` — отрисовка ручки перетаскивания, индикатора запущенных программ, маркера вставки и разделителей.
- `src/render_ops.rs` — низкоуровневые пиксельные примитивы: альфа-смешивание, SDF скруглённого прямоугольника, шестерёнка настроек, заглушка и билинейное масштабирование.
- `src/state.rs` — событийный автомат `DockInstance`, анимация запуска (2-тактный отскок 460 мс, 0% CPU в покое), подсказки `hovered_tooltip` и счётчики метрик.
- `src/storage.rs` — портативное сохранение и загрузка неограниченного числа док-панелей (`label_mode`, `layer`, `launch_animation_enabled` и др.) в `./data/config/docks.cfg`.
