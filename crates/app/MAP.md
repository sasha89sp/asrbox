# MAP.md — `crates/app`

Точка входа `asrbox.exe` (`#![windows_subsystem = "windows"]` в релизе), CLI (`--version`, `--selftest`, `--safe-mode`), защита одного экземпляра и оркестрация неограниченного числа док-панелей через `platform-api`, `mod-dock` и `platform-windows`.

## Файлы
- `src/main.rs` — точка входа, `--selftest`, загрузка/сохранение `./data/config/docks.cfg` и цикл событий `AppController`.
