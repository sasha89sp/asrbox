# MAP.md — `crates/app`

Исполняемый бинарник `asrbox.exe`. Работает как мастер-супервизор (трей, Настройки, Диагностика, О программе) с изолированными дочерними процессами модулей (`--worker dock|widgets|overlay`) и резервным `inproc`-режимом (`--inproc`).

## Файлы
- `src/main.rs` — точка входа, CLI (`--version`, `--selftest`, `--safe-mode`, `--inproc`, `--worker`, `--parent-pid`) и `AppController`.
- `src/supervisor.rs` — `ModuleSupervisor`: запуск/остановка процессов модулей в Win32 `JobObject`, stdin/stdout IPC и автоперезапуск при сбое.
- `src/worker.rs` — изолированный цикл воркера док-панелей (`--worker dock`), чтение `SupervisorCmd`, watchdog PID и отправка логов/метрик.
- `src/worker_widgets.rs` — изолированный цикл воркера виджетов (`--worker widgets`): Google Календарь и Системный монитор.
- `src/worker_overlay.rs` — изолированный цикл воркера игрового оверлея (`--worker overlay`) и интеграция настроек/режима настройки с `AppController`.
- `src/docks.rs` — `DockManager`: жизненный цикл док-панелей, индикатор запущенных приложений и `docks.cfg`.
- `src/dock_ops.rs` — обработка меню и кликов док-панели, загрузка иконок, резолв `.exe` и отрисовка кадров.
- `src/dock_drag.rs` — `DockDragState`: перетаскивание значков/разделителей внутри панели и между панелями, удаление за пределами панели и перемещение откреплённого дока.
- `src/backup_ctl.rs` — `BackupController`: локальные `.zip`-бэкапы (`mod-backup`), конфиг WebDAV (`mod-webdav`), пароли в Windows Credential Manager и авто/ручная выгрузка в облако.
- `src/settings_ctl.rs` — мост между NotePlan-окном `mod-settings` и конфигами `modules.cfg`, `docks.cfg`, автозагрузкой.
- `src/settings_widgets.rs` — применение настроек виджетов (`widgets.cfg`, `sysmon.cfg`), синхронизации календаря и действий бэкапа/WebDAV.
- `src/tools.rs` — сгруппированное ПКМ-меню трея (включая игровой оверлей, автозагрузку и бэкап), окна «Диагностика» и «О программе».
- `src/worker_bus.rs` — маршрутизация `SupervisorCmd`/`WorkerMsg` между супервизором и воркерами, а также `inproc`-диспетчер виджетов.
- `src/selftest.rs` — кроссплатформенный `--selftest` (`core`, `mod-dock`, `mod-widgets`, `mod-overlay`, `mod-settings`, `mod-backup`, `mod-webdav`, `mod-debug`, `mod-about`).
