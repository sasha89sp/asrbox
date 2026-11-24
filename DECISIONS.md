# DECISIONS.md — Архитектурные решения (ADR) `asrbox`

Все принятые архитектурные решения, версии внешних крейтов и их причины фиксируются здесь.

---

## ADR-001: Единый Rust workspace (`resolver = "2"`) и единый источник версии

- **Статус**: Принято (v0.0.1)
- **Решение**: Корневой `Cargo.toml` задаёт `[workspace]`, `resolver = "2"` и `[workspace.package]` (`version = "0.0.1"`, `edition = "2021"`). Все 12 крейтов наследуют `version.workspace = true`.
- **Причина**: Гарантирует совпадение версии в бинарнике `asrbox.exe`, дочерних процессах `--module`, трее, `--version`, логах и теге `release.yml` (`NF-009`).

---

## ADR-002: Изоляция `unsafe` в `crates/platform-windows` и запрет `unwrap()`/`expect()`

- **Статус**: Принято (v0.0.1)
- **Решение**: В `[workspace.lints.rust]` включено `unsafe_code = "forbid"`, в `[workspace.lints.clippy]` — `unwrap_used = "deny"` и `expect_used = "deny"`. Только `crates/platform-windows` не наследует `unsafe_code = "forbid"` (сохраняя запрет `unwrap`/`expect`) и реализует трейты `platform-api` через `windows-rs`.
- **Причина**: Исключает неконтролируемые паники и концентрирует аудит небезопасного FFI в одном крейте.

---

## ADR-003: Мультипроцессная изоляция через один `.exe` (`--module`) и Named Pipes

- **Статус**: Принято (v0.0.1)
- **Решение**: Единый `asrbox.exe` выступает супервизором и запускает изолированные модули как дочерние процессы `--module <имя>`, общаясь через Windows Named Pipes. Каждый модуль реализует `Module (start, stop, health)` и поддерживает `inproc`/`process` режимы.
- **Причина**: Падение или зависание хука, модема, сети или Whisper не роняет основное приложение и позволяет полностью освобождать память (`NF-002`, `NF-006`, `NF-007`, `NF-008`).

---

## ADR-004: Портативность `./data/`, Windows Credential Manager и сворачивание по `X`

- **Статус**: Принято (v0.0.1)
- **Решение**: Все настройки, бэкапы, кеши и логи хранятся в `./data/` рядом с `.exe`; секреты WebDAV и токены — только в Windows Credential Manager; модель Whisper скачивается лениво с проверкой SHA-256; кнопка `X` сворачивает окно в трей.
- **Причина**: Нулевая зависимость от установщика, безопасность учётных данных и постоянная готовность фоновых модулей.

---

## ADR-005: `xtask atlas-check` на `std`, алиас `cargo xtask` и лимиты документов

- **Статус**: Принято (v0.0.1)
- **Решение**: Крейт `xtask` написан исключительно на стандартной библиотеке Rust (`std`), подключён через `.cargo/config.toml` (`cargo xtask atlas-check`). Установлены жёсткие лимиты: файл <= 400 строк, функция <= 60 строк, `ATLAS.md` <= 150, `MAP.md` <= 100, `HANDOFF.yaml` <= 80.
- **Причина**: Нулевой оверхед в CI и гарантированная передача контекста между сессиями без деградации документации.

---

## ADR-006: Поэтапное включение `cargo-deny` и `--selftest` в CI

- **Статус**: Принято (v0.0.1)
- **Решение**: На этапе пустого каркаса (`0.0.1`) внешние крейты отсутствуют (реестр `docs/LICENSES.md` пуст), поэтому `ci.yml` проверяет `fmt`, `clippy`, `xtask atlas-check`, `test`, `build --release`, лимит размера `< 25 МБ` (`NF-005`), смоук-прогон `asrbox.exe --selftest` и выгрузку артефакта. Шаг `cargo deny check` с `deny.toml` будет активирован при добавлении первой внешней зависимости.
- **Причина**: Обеспечивает зелёный, детерминированный CI с первого коммита и сохраняет целевую схему проверок из руководства.

---

## ADR-007: Неограниченное число док-панелей, увеличение одной иконки и стратегия иконок

- **Статус**: Принято (v0.0.1, `specs/mod-dock.md`)
- **Решение**:
  1. Снять жёсткое ограничение в 8 док-панелей: хранить произвольное число независимых панелей (`DockRegistry` / `DockConfig`) внутри одного процесса `mod-dock`.
  2. Для минимизации нагрузки на CPU (`NF-001`) на первом этапе использовать режим `HoverZoomMode::SingleIcon` (увеличивается только активная иконка под курсором, без волнового пересчёта соседей) с резервом пространства (`zoom headroom`) внутри окна и остановкой таймера анимации в покое.
  3. Для качественных иконок на Windows использовать `IconSource::AutoShell` (`IShellItemImageFactory` до 256×256 + кэш в `data/cache/icons/`) с возможностью ручного выбора `CustomFile(.ico/.png)` или `ExeResource`.
- **Причина**: Обеспечивает гибкость настройки, чёткую графику в стиле macOS и предсказуемо низкое потребление CPU/RAM.

---

## ADR-008: Зависимость `windows = "0.61"`, трейты `platform-api` и рендер `UpdateLayeredWindow`

- **Статус**: Принято (v0.0.1, согласовано владельцем)
- **Решение**:
  1. Подключить единственную внешнюю зависимость `windows = "0.61"` (лицензия `MIT OR Apache-2.0`) исключительно в `crates/platform-windows` под `[target.'cfg(windows)'.dependencies]`.
  2. Зафиксировать безопасные платформенные контракты `DockPlatform`, `IconProvider`, `PlatformEvent`, `DockMenuAction`, `TrayAction`, `RgbaImage`, `BgraFrame` в `crates/platform-api` (без `unsafe`).
  3. Рендер полупрозрачной macOS-капсулы с мягкой тенью и билинейным масштабированием иконок выполнять в безопасном чистом Rust (`crates/mod-dock/src/render.rs`) с передачей 32-битного premultiplied BGRA в `UpdateLayeredWindow`.
  4. Извлечение иконок выполнять через `IShellItemImageFactory` (до 256×256) + WIC (`IWICImagingFactory`) для пользовательских `.ico`/`.png` + `ExtractIconExW` для `.exe`/`.dll`; выбор файла — через `GetOpenFileNameW`, запуск — через `ShellExecuteW`.
  5. Хранить неограниченное число док-панелей в портативном `./data/config/docks.cfg` (`settings_version=1`, `written_by`) без внешних крейтов сериализации.
- **Причина**: Минимальный размер `.exe` (< 25 МБ), минимальный расход RAM (< 30 МБ), 0% CPU в покое за счёт событийного `UpdateLayeredWindow` и строгая изоляция `unsafe` в `crates/platform-windows`.

---

## ADR-009: Модуль `crates/mod-widgets`, потоковый `.ics`/RRULE, WinHTTP и GDI-рендер текста

- **Статус**: Принято (v0.0.1, согласовано владельцем, `specs/mod-widgets.md`)
- **Решение**:
  1. Добавить в workspace крейт `crates/mod-widgets` (`asrbox-mod-widgets`) без `unsafe` и без внешних зависимостей (зависит только от `crates/platform-api`).
  2. Реализовать потоковый построчный парсер RFC 5545 (`ics.rs`) и движок повторов `RRULE` (`rrule.rs`) на чистом `std` с быстрым пропуском архива, поддержкой `VTIMEZONE`/`UTC`, `EXDATE`, `RECURRENCE-ID`, `STATUS:CANCELLED` и многодневных `VALUE=DATE`.
  3. Расширить `crates/platform-api` контрактом `WidgetPlatform`, `WidgetFrame`, `WidgetTextCmd`, `WidgetLayer`, `HttpFetchResult` и событиями `PlatformEvent::Widget*`/`HttpFetchDone`.
  4. В `crates/platform-windows` активировать фичи `Win32_Networking_WinHttp`, `Win32_Graphics_Dwmapi`, `Win32_System_SystemInformation`, `Win32_System_Time` существующего крейта `windows = "0.61"` (новых крейтов не добавлено): фоновая загрузка `.ics` в потоке WinHTTP, рендер кириллического шрифта `Segoe UI` (`CreateFontIndirectW` + `DrawTextW`) с альфа-композитингом на 32-битный BGRA-буфер `UpdateLayeredWindow`, слои `Wallpaper`/`Normal`/`Topmost` и опциональный Windows 11 Acrylic (`DWMWA_SYSTEMBACKDROP_TYPE`).
  5. Хранить настройки в `./data/config/widgets.cfg`, а кэш календарей — в `./data/cache/calendar_cache_<id>.ics`. Двустороннее редактирование через Google Calendar OAuth2 API вынесено в этап 2.
- **Причина**: Нулевое раздувание `.exe` (без `reqwest`/`rustls`/`chrono`/`regex`), мгновенный офлайн-старт из кэша, 0% CPU в простое и строгое соблюдение границ `unsafe`.

---

## ADR-010: Замена Acrylic на прозрачность, разделители дока и модули `mod-debug` / `mod-about`

- **Статус**: Принято (v0.0.1, согласовано владельцем)
- **Решение**:
  1. Убрать системное размытие DWM Acrylic (`DwmSetWindowAttribute`, фича `Win32_Graphics_Dwm`) из виджетов в пользу настраиваемой прозрачности фона (`opacity=20..100`, пресеты `100%/85%/70%/50%/35%`) в `mod-widgets`.
  2. Открывать текстовые `.cfg`-файлы и диагностические отчёты через `notepad.exe` (`open_text_file`), не перезаписывая существующий файл настроек на диске при открытии; в `widgets.cfg` и `docks.cfg` поддерживать UTF-8 BOM и упрощённые форматы строк.
  3. Добавить в `mod-dock` разделители (`DockItemKind::Separator`, строка `separator=<id>` в `docks.cfg`) и встроенную кнопку настроек (`⚙`, `show_settings_button`, по умолчанию включена).
  4. Добавить в workspace два независимых крейта без `unsafe` и без внешних зависимостей: `crates/mod-debug` (живой журнал с `TRACE`, опциональный файл с ротацией 2 МБ, автоматическая редакция секретов, метрики процесса и помодульная нагрузка, отчёты) и `crates/mod-about` (версия, описание портативности, динамические пути на диске, встроенный `CHANGELOG.md`).
  5. В `crates/platform-windows` активировать фичи `Win32_System_DataExchange`, `Win32_System_Memory`, `Win32_System_ProcessStatus` существующего крейта `windows = "0.61"` (новых крейтов не добавлено) и перевести WinHTTP на `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` с таймаутами.
- **Причина**: Понятное управление прозрачностью без платформенных артефактов DWM, надёжное открытие конфигов на чистой Windows, удобная организация дока и прозрачная диагностика проблем в реальном времени без нагрузки в простое.

---

## ADR-011: Мультипроцессная изоляция воркеров (`JobObject` + stdin/stdout IPC), `mod-settings` и расширение `mod-dock`

- **Статус**: Принято (v0.0.1, согласовано владельцем)
- **Решение**:
  1. Реализовать мультипроцессную изоляцию через один портативный `asrbox.exe`: мастер-супервизор запускает активные модули (`--worker dock`, `--worker widgets`, `--parent-pid <PID>`) внутри Win32 `JobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), общается построчным IPC (`SupervisorCmd`/`WorkerMsg` в `crates/core/src/ipc.rs`) с пробуждением через `WM_IPC_WAKE` и перезапускает упавший модуль (до 3 раз/мин). Включение/выключение модулей и режим (`process`/`inproc`) хранятся в `./data/config/modules.cfg`.
  2. Добавить крейт `crates/mod-settings` (без `unsafe`, без внешних зависимостей) — единое изменяемое macOS-окно настроек (`AsrboxSettingsWindow`, `WS_OVERLAPPEDWINDOW`, min `680×480`, программный BGRA + `DrawTextW` + нативное поле `EDIT` для вставки `.ics` URL) и сгруппировать ПКМ-меню трея по подменю модулей (`MF_POPUP`).
  3. В `crates/platform-windows` активировать фичи `Win32_System_JobObjects` и `Win32_System_Com_StructuredStorage` существующего крейта `windows = "0.61"` (новых внешних крейтов не добавлено) для `JobObject` и резолва `.lnk` через `IShellLinkW`, а определение запущенных приложений и активацию окон реализовать через `EnumWindows` + `QueryFullProcessImageNameW`.
  4. В `mod-dock` добавить ЛКМ-перетаскивание значков и разделителей внутри/между панелями, удаление выносом за пределы панелей, закрепление/открепление с ручкой и привязкой `Offset(px)`, прозрачность (`20..100%`), произвольный размер иконок (`24..128 px`) и индикатор/активацию запущенных приложений.
- **Причина**: Сбой любого прикладного модуля больше не роняет остальную программу, отключённые модули не тратят память/CPU, а все настройки собраны в одном понятном интерфейсе без увеличения числа внешних зависимостей.

---

## ADR-012: Виджет «Системный монитор», единый стиль macOS Dark-Glass и ресайз виджетов

- **Статус**: Принято (v0.0.1, согласовано владельцем, `specs/mod-widgets.md` `WG-014..WG-018`)
- **Решение**:
  1. Разместить виджет «Системный монитор» (`SYS_WIDGET_ID = 2`, конфиг `./data/config/sysmon.cfg`) внутри существующего крейта `crates/mod-widgets` и воркера `--worker widgets` (без лишнего процесса и без новых внешних крейтов).
  2. Активировать в `crates/platform-windows` дополнительные модули существующего крейта `windows = "0.61"`: `Win32_Graphics_Dxgi`, `Win32_NetworkManagement_IpHelper`, `Win32_NetworkManagement_Ndis`, `Win32_Networking_WinSock`, `Win32_System_Diagnostics_ToolHelp`, `Win32_System_IO`, `Win32_System_Ioctl`, `Win32_System_Power`, `Win32_System_Registry` (реестр `docs/LICENSES.md` не меняется, т.к. крейт `windows` уже зафиксирован).
  3. Собирать реальную телеметрию в ленивом фоновом потоке (`sys_telemetry.rs`, `sys_procs.rs`, `sys_gpu.rs`, `sys_net.rs`) с настраиваемой периодичностью (`1с/2с/3с/5с/10с`, по умолчанию `2с`) и доставкой снимка через `WM_TELEMETRY_DONE`; для датчиков NVIDIA динамически подгружать `nvml.dll` (`LoadLibraryW`) с фолбэком VRAM на DXGI `IDXGIAdapter3`, а для разрешения таймера ОС — `NtQueryTimerResolution` из `ntdll.dll`. Аппаратные сенсоры, недоступные без драйвера уровня ядра (например, температура CPU), честно отображаются как `—` без подстановки фиктивных значений.
  4. Унифицировать оформление обоих виджетов общими примитивами macOS Dark-Glass (`fill_macos_glass_card`, `fill_glow_bar`, `fill_pill_badge`, `draw_resize_grip`) и реализовать прямое изменение размеров за 8 краёв/углов в `widget_proc.rs` (`WidgetSizeBounds`, `PlatformEvent::WidgetResized`) + быстрое выравнивание размеров друг с другом в `mod-settings`.
- **Причина**: Нулевое раздувание бинарника, отсутствие прав администратора и драйверов ядра, честная системная телеметрия без нагрузки в скрытом состоянии и визуальная целостность виджетов на рабочем столе.

---

## ADR-013: Концепция macOS NotePlan Dark, программная иконка `HICON`, автозагрузка `HKCU`, `mod-backup` и WebDAV (`v0.0.2`)

- **Статус**: Принято (v0.0.2, согласовано владельцем)
- **Решение**:
  1. Принять единую визуальную концепцию **macOS NotePlan Dark** (`#1C1D22` полотно, `#16171B` сайдбар, `#23252C` карточки, `#30333E` тонкие границы, тёплый коралловый акцент `#E87A48`) для всех виджетов, док-панелей, окна настроек и инструментов диагностики/«О программе» без добавления GPU/DWM-эффектов (0% лишней нагрузки CPU).
  2. Генерировать фирменную 32-битную иконку приложения в стиле NotePlan программно через `CreateIconIndirect` (`crates/platform-windows/src/app_icon.rs`, кэшируемые 16×16 для системного трея/`ICON_SMALL` и 32×32 для панели задач/`ICON_BIG`, без внешних ресурсов/крейтов) и назначать её трею и всем окнам `asrbox`.
  3. Добавить поддержку слоя окна для док-панелей (`WidgetLayer::Wallpaper` по умолчанию, `Normal`, `Topmost` через `WM_WINDOWPOSCHANGING` + `HWND_BOTTOM`) и настраиваемую частоту синхронизации Google Календаря (`5м / 15м / 30м / 1ч / 3ч`) вместе с отображением всех сегодняшних событий, включая уже прошедшие.
  4. Реализовать автозагрузку Windows без прав администратора через `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\asrbox` (`crates/platform-windows/src/autostart.rs`).
  5. Добавить крейт `crates/mod-backup` (чистый `std`, стандартный STORE ZIP + IEEE CRC-32, режимы «только настройки» и «программа + настройки» в `./data/backups/`) и активировать `crates/mod-webdav` (пресеты Яндекс.Диск `https://webdav.yandex.ru`, Mail.ru `https://webdav.cloud.mail.ru`, Свой HTTPS) с фоновой отправкой через WinHTTP (`PROPFIND`, `MKCOL`, `PUT` в `webdav_http.rs`). В `crates/platform-windows` добавлена только фича `Win32_Security_Credentials` существующего крейта `windows = "0.61"` (новых внешних зависимостей нет; `docs/LICENSES.md` не требует изменений) для хранения паролей приложений исключительно в **Windows Credential Manager**.
- **Причина**: Полное соответствие бюджетам `NF-001..NF-005` (ноль новых внешних крейтов, крошечный `.exe`), безопасность учётных данных и цельный визуальный язык всех компонентов.

---

## ADR-014: Изолированный модуль «Игровой оверлей» (`crates/mod-overlay`, `--worker overlay`, без инъекции DLL, `v0.0.3`)

- **Статус**: Принято (v0.0.3, согласовано владельцем, `specs/mod-overlay.md`)
- **Решение**:
  1. Добавить независимый крейт `crates/mod-overlay` (`asrbox-mod-overlay`, без `unsafe` и без новых внешних крейтов), запускаемый отдельным изолированным дочерним процессом `asrbox.exe --worker overlay` под Win32 `JobObject` (`ModuleId::Overlay`, `SupervisorCmd::SetOverlaySetupMode`).
  2. Отказаться от инъекции DLL и хуков DirectX/Vulkan/OpenGL в пользу нативного прозрачного layered-окна Win32 (`UpdateLayeredWindow` + `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`, `WM_NCHITTEST -> HTTRANSPARENT`, `WM_MOUSEACTIVATE -> MA_NOACTIVATE`), чтобы исключить риски античитов (EAC/BattlEye/Vanguard) и не создавать фризов в играх.
  3. Вынести интерактивное перемещение и ресайз в явный режим настройки до игры (`setup_mode = true`), который автоматически отключается при закрытии окна «Настройки».
  4. Активировать в `crates/platform-windows` только фичу `Win32_Graphics_Dwm` существующего крейта `windows = "0.61"` (`docs/LICENSES.md` без изменений) для чтения статистики кадров DWM (`DwmGetCompositionTimingInfo` -> FPS и 1% low с честным фолбэком на `display_hz` при обходе композитора), а обнаружение игр выполнять в фоновом потоке (`overlay_game.rs`: `SHQueryUserNotificationState`/`QUNS_RUNNING_D3D_FULL_SCREEN` + покрытие монитора + чёрный список системных процессов + белый список `extra_games` в `overlay.cfg`).
- **Причина**: Максимальная надёжность, нулевая нагрузка вне игр (`auto_games_only = true`), безопасность для античитов и сохранение компактного портативного бинарника.

---

## ADR-015: Индивидуальные режимы имён док-панелей, исправление конвейера иконок и FSO/Topmost оверлея (`v0.0.4`)

- **Статус**: Принято (v0.0.4, согласовано владельцем)
- **Решение**:
  1. Сохранить строгую индивидуальность каждой док-панели (`DockConfig` без общих глобальных настроек): добавить независимое поле `label_mode` (`IconsOnly` / `HoverTooltip` / `IconAndLabel` / `LabelsOnly`) в `docks.cfg`, ПКМ-меню и вкладку «Док-панели», а также переименование ярлыков через нативное поле `EDIT` с кнопкой сброса.
  2. Перевести вывод кадра док-панели на единый композитор текста `WidgetFrame` (`present_dock_widget_frame` + `set_dock_tooltip` в `DockPlatform`), переиспользуя существующий рендер `Segoe UI` и всплывающих подсказок без дублирования GDI-кода.
  3. Усилить конвейер извлечения иконок (`crates/platform-windows/src/{icons.rs, icons_lnk.rs}`): добавить только фичу `Win32_System_Environment` существующего крейта `windows = "0.61"` (без новых внешних крейтов, `docs/LICENSES.md` без изменений) для `ExpandEnvironmentStringsW` при чтении `IShellLinkW::GetIconLocation`/`GetPath`, отсекать серую обёртку Explorer (`has_explorer_frame_box`), восстанавливать прозрачность legacy 1-битных масок `HICON` двойной растеризацией на чёрном/белом фоне и добавить фолбэк `SHGetFileInfoW(SHGFI_ICON | SHGFI_LARGEICON)`.
  4. Добавить в `mod-overlay` опцию `force_topmost_fullscreen` (по умолчанию `true`) с `WS_EX_TOPMOST` при создании, `SWP_NOZORDER` при переключении click-through, бережным неактивирующим `HWND_TOPMOST` во время активной игры (FSO/DX12/Vulkan без DLL-инъекции) и автоматическим выходом из `setup_mode` при старте игры.
- **Причина**: Чёткие иконки без синих заглушек и серых рамок, полная свобода отображения имён на каждой док-панели отдельно и надёжный античит-безопасный оверлей поверх FSO-игр.



