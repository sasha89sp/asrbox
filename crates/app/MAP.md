# MAP.md — `crates/app`

Точка входа `asrbox.exe` (`#![windows_subsystem = "windows"]` в релизе), CLI (`--version`, `--selftest`, `--safe-mode`), защита одного экземпляра и оркестрация `mod-dock`, `mod-widgets`, `mod-debug` и `mod-about` через `platform-api` и `platform-windows`.

## Файлы
- `src/main.rs` — точка входа, цикл событий `AppController`, маршрутизация событий док-панелей и виджета Google Календарь.
- `src/tools.rs` — обработчики трея, окна диагностики (`mod-debug`), «О программе» (`mod-about`) и определение портативных путей.
- `src/docks.rs` — менеджер неограниченного числа док-панелей `DockManager` (загрузка/перезагрузка/сохранение `docks.cfg`, разделители, кнопка настроек, иконки, события и ПКМ-меню).
- `src/selftest.rs` — встроенная проверка `--selftest` для `mod-dock`, `mod-widgets`, `mod-debug` и `mod-about`.
