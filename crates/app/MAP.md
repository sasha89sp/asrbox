# MAP.md — `crates/app`

Исполняемый бинарник `asrbox.exe`. Работает как мастер-супервизор (Fluent-трей, Настройки с браузерными вкладками, бэкап/WebDAV/синхронизация) с изолированными дочерними процессами модулей (`--worker dock|widgets|overlay`) и резервным `inproc`-режимом (`--inproc`).

## Файлы
- `src/main.rs` — точка входа, CLI (`--version`, `--selftest`, `--safe-mode`, `--inproc`, `--worker`, `--parent-pid`) и `AppController` (включая плавную анимацию темы).
- `src/supervisor.rs` — `ModuleSupervisor`: запуск/остановка процессов модулей в Win32 `JobObject`, stdin/stdout IPC и автоперезапуск при сбое.
- `src/worker.rs` — изолированный цикл воркера док-панелей (`--worker dock`), чтение `SupervisorCmd` (включая `SetTheme`), watchdog PID и отправка логов/метрик.
- `src/worker_widgets.rs` — изолированный цикл воркера виджетов (`--worker widgets`): Google Календарь и Системный монитор.
- `src/worker_overlay.rs` — изолированный цикл воркера игрового оверлея (`--worker overlay`) и интеграция настроек/режима настройки с `AppController`.
- `src/docks.rs` — `DockManager`: жизненный цикл док-панелей, индикатор запущенных приложений и `docks.cfg`.
- `src/dock_events.rs` — диспетчер событий док-панелей для `inproc`-режима (`handle_dock_event`).
- `src/dock_ops.rs` — обработка меню и кликов док-панели, загрузка иконок, резолв `.exe` и отрисовка кадров.
- `src/dock_drag.rs` — `DockDragState`: перетаскивание значков/разделителей внутри панели и между панелями, удаление и перемещение откреплённого дока.
- `src/backup_ctl.rs` — `BackupController`: локальные `.zip`-бэкапы (`mod-backup`), мультисерверная выгрузка, WebDAV-черновик и синхронизация файлов/папок.
- `src/webdav_ctl.rs` — управление серверами WebDAV (`WebDavStore`), Credential Manager, тест/обзор/создание папок и обработка `WebDavResult`.
- `src/sync_ctl.rs` — периодическая и ручная синхронизация произвольных файлов и папок Windows (`sync.cfg`).
- `src/settings_ctl.rs` — мост между Fluent-окном `mod-settings` и конфигами `modules.cfg`, `docks.cfg`, автозагрузкой и браузерными вкладками.
- `src/settings_general.rs` — обработка действий раздела «Общие настройки» (тема, акцент, язык, бэкап, WebDAV-меню, обзор папок, синхронизация).
- `src/settings_widgets.rs` — применение настроек виджетов (`widgets.cfg`, `sysmon.cfg`) и синхронизации календаря.
- `src/tools.rs` — Fluent ПКМ-меню трея и вспомогательные окна «Диагностика» и «О программе».
- `src/worker_bus.rs` — маршрутизация `SupervisorCmd`/`WorkerMsg` между супервизором и воркерами, а также `inproc`-диспетчер виджетов.
- `src/selftest.rs` — кроссплатформенный `--selftest` (ядро, доки, виджеты, оверлей).
- `src/selftest_tools.rs` — кроссплатформенный `--selftest` для `mod-settings`, `mod-backup`, `mod-webdav`, `mod-debug` и `mod-about`.
