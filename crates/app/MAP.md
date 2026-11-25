# MAP.md — `crates/app`

Исполняемый бинарник `asrbox.exe`. Работает как мастер-супервизор (Fluent-трей, Настройки с браузерными вкладками и аккордеонами, бэкап/AES-256/GitHub/WebDAV/синхронизация) с изолированными дочерними процессами модулей (`--worker dock|widgets|overlay`) и резервным `inproc`-режимом (`--inproc`).

## Файлы
- `build.rs` — zero-dependency генератор Win32-ресурса `.res` (`RT_GROUP_ICON` + 7 размеров 16..256 px) для встраивания фирменной иконки «Стеклянный блок ASR» в `asrbox.exe`.
- `src/main.rs` — точка входа, CLI (`--version`, `--selftest`, `--safe-mode`, `--inproc`, `--worker`, `--parent-pid`) и `AppController` (включая `ContextMenuConfig` и плавную анимацию темы).
- `src/supervisor.rs` — `ModuleSupervisor`: запуск/остановка процессов модулей в Win32 `JobObject`, stdin/stdout IPC и автоперезапуск при сбое.
- `src/worker.rs` — изолированный цикл воркера док-панелей (`--worker dock`), чтение `SupervisorCmd`, `menus.cfg`, watchdog PID и отправка логов/метрик.
- `src/worker_widgets.rs` — изолированный цикл воркера виджетов (`--worker widgets`): Google Календарь, Системный монитор, Погода и Заметки.
- `src/worker_overlay.rs` — изолированный цикл воркера игрового оверлея (`--worker overlay`) и интеграция настроек/режима настройки с `AppController`.
- `src/docks.rs` — `DockManager`: жизненный цикл док-панелей, обновление окон на месте, видимость пунктов ПКМ (`DockMenuVisibility`), индикатор запущенных приложений и `docks.cfg`.
- `src/dock_events.rs` — диспетчер событий док-панелей для `inproc`-режима (`handle_dock_event`).
- `src/dock_ops.rs` — обработка меню и кликов док-панели, запуск анимации отскока, загрузка иконок, резолв `.exe` и отрисовка кадров.
- `src/dock_shortcuts.rs` — портативное копирование `.lnk`/`.url` в `data/shortcuts/`, автомиграция внешних ярлыков и очистка неиспользуемых копий.
- `src/dock_drag.rs` — `DockDragState`: перетаскивание значков/разделителей (с проверкой `lock_items`), удаление и свободное перемещение откреплённого дока.
- `src/backup_ctl.rs` — `BackupController`: локальные `.zip`-бэкапы (`mod-backup`), мультисерверная выгрузка, WebDAV-черновик и синхронизация файлов/папок.
- `src/github_ctl.rs` — шифрование бэкапов AES-256 (Credential Manager + CSPRNG) и скачивание/включение в бэкап исходного кода с GitHub.
- `src/webdav_ctl.rs` — управление серверами WebDAV (`WebDavStore`), Credential Manager, тест/обзор/создание папок и обработка `WebDavResult`.
- `src/sync_ctl.rs` — периодическая и ручная синхронизация произвольных файлов и папок Windows (`sync.cfg`).
- `src/settings_ctl.rs` — мост между Fluent-окном `mod-settings` и конфигами модулей, синхронизация полей ввода и сбор `SettingsSnapshot`.
- `src/settings_combo.rs` — обработка выпадающих списков (ComboBox) окна настроек через `show_fluent_popup_menu`.
- `src/settings_general.rs` — обработка действий раздела «Общие настройки» (дизайн-тема, тема, акцент, язык, контекстные меню, бэкап/AES-256/GitHub/WebDAV, подтверждённый сброс).
- `src/settings_widgets.rs` — применение настроек док-панелей (`docks.cfg`) и виджетов (`widgets.cfg`, `sysmon.cfg`, `weather.cfg`, `notes.cfg`).
- `src/tools.rs` — Fluent ПКМ-меню трея (`TrayMenuVisibility`, Погода, Заметки), crash-hook и вспомогательные окна «Диагностика» и «О программе».
- `src/worker_bus.rs` — маршрутизация `SupervisorCmd`/`WorkerMsg` между супервизором и воркерами.
- `src/inproc_widgets.rs` — `inproc`-диспетчер событий для всех 4 виджетов (Календарь, Монитор, Погода, Заметки).
- `src/selftest.rs` — кроссплатформенный `--selftest` (ядро, тема/логотип, доки, все 4 виджета, оверлей).
- `src/selftest_tools.rs` — кроссплатформенный `--selftest` для `mod-settings`, `mod-backup` (AES-256 + GitHub), `mod-webdav`, `mod-debug` и `mod-about`.
