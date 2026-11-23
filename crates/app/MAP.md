# MAP.md — `crates/app`

Точка входа `asrbox.exe` (`#![windows_subsystem = "windows"]` в релизе), CLI (`--version`, `--selftest`, `--safe-mode`), защита одного экземпляра и оркестрация док-панелей (`mod-dock`) и виджетов (`mod-widgets`) через `platform-api` и `platform-windows`.

## Файлы
- `src/main.rs` — точка входа, цикл событий `AppController`, маршрутизация событий док-панелей, виджета Google Календарь и меню трея.
- `src/docks.rs` — менеджер неограниченного числа док-панелей `DockManager` (загрузка/сохранение `docks.cfg`, иконки, события и ПКМ-меню).
- `src/selftest.rs` — встроенная проверка `--selftest` для `mod-dock` (рендер + roundtrip) и `mod-widgets` (парсинг `.ics`, RRULE, рендер карточки, roundtrip настроек).
