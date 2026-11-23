# MAP.md — `crates/app`

Исполняемый бинарник `asrbox.exe`. Работает как мастер-супервизор (трей, Настройки, Диагностика, О программе) с изолированными дочерними процессами модулей (`--worker dock|widgets`) и резервным `inproc`-режимом (`--inproc`).

## Файлы
- `src/main.rs` — точка входа, CLI (`--version`, `--selftest`, `--safe-mode`, `--inproc`, `--worker`, `--parent-pid`) и `AppController`.
- `src/supervisor.rs` — `ModuleSupervisor`: запуск/остановка процессов модулей в Win32 `JobObject`, stdin/stdout IPC и автоперезапуск при сбое.
- `src/worker.rs` — изолированный цикл воркера док-панелей (`--worker dock`), чтение `SupervisorCmd`, watchdog PID и отправка логов/метрик.
- `src/worker_widgets.rs` — изолированный цикл воркера виджетов (`--worker widgets`): Google Календарь и Системный монитор.
- `src/docks.rs` — `DockManager`: жизненный цикл док-панелей, индикатор запущенных приложений и `docks.cfg`.
- `src/dock_ops.rs` — обработка меню и кликов док-панели, загрузка иконок, резолв `.exe` и отрисовка кадров.
- `src/dock_drag.rs` — `DockDragState`: перетаскивание значков/разделителей внутри панели и между панелями, удаление за пределами панели и перемещение откреплённого дока.
- `src/settings_ctl.rs` — мост между macOS-окном `mod-settings` и конфигами `modules.cfg`, `docks.cfg`.
- `src/settings_widgets.rs` — применение настроек виджетов (`widgets.cfg`, `sysmon.cfg`), частоты опроса и выравнивания размеров окон.
- `src/tools.rs` — сгруппированное ПКМ-меню трея, окна «Диагностика» и «О программе».
- `src/worker_bus.rs` — маршрутизация `SupervisorCmd`/`WorkerMsg` между супервизором и воркерами, а также `inproc`-диспетчер виджетов.
- `src/selftest.rs` — кроссплатформенный `--selftest` (`core`, `mod-dock`, `mod-widgets`, `mod-settings`, `mod-debug`, `mod-about`).
