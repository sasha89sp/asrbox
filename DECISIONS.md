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

---

## ADR-016: Единый Windows 11 Fluent Design, кастомные Fluent-меню, «Общие настройки» с браузерными вкладками и мультисерверный WebDAV + синхронизация (`v0.0.5`)

- **Статус**: Принято (v0.0.5, согласовано владельцем)
- **Решение**:
  1. Заменить фиксированную палитру NotePlan Dark на единую систему **Windows 11 Fluent Design** (`crates/platform-api/src/{theme.rs, palette.rs}`): Светлая (`#F3F3F3`/`#FFFFFF`) и Тёмная (`#202020`/`#191919`/`#2B2B2B`) темы, 6 цветовых акцентов, язык `Русский`/`English` (`./data/config/appearance.cfg`), синхронизация заголовка `DWMWA_USE_IMMERSIVE_DARK_MODE` и плавная поканальная интерполяция `FluentPalette::lerp` (220 мс с полной остановкой таймера после завершения, `0%` CPU в простое) во всех окнах, доках, виджетах и игровом оверлее (`SupervisorCmd::SetTheme`).
  2. Заменить системные Win32 `TrackPopupMenuEx` в трее, доках, виджетах и выборе WebDAV-провайдера на кастомные слоистые окна **Fluent Popup Menu** (`crates/platform-windows/src/fluent_menu*.rs`, `UpdateLayeredWindow` + `DrawTextW`) с быстрым верхним рядом, вложенными подменю и полной поддержкой светлой/тёмной темы.
  3. Объединить вкладки «О программе», «Диагностика» и «Бэкап и облако» в единый раздел **«Общие настройки»** (`SettingsTab::General`) и добавить верхнюю панель браузерных вкладок внутри окна настроек (`BrowserTab::Diagnostics` с живым 1 Гц монитором, `BrowserTab::VersionHistory` с полным `CHANGELOG.md`, `BrowserTab::WebDavFolder` с навигатором `PROPFIND Depth: 1` и созданием папок `MKCOL`).
  4. Расширить `crates/mod-webdav` до неограниченного числа серверов (`WebDavStore`, автомиграция `webdav.cfg` v1 -> v2 с сохранением `asrbox:webdav:mailru` в Windows Credential Manager, пароли по-прежнему только в Credential Manager) и добавить периодическую синхронизацию произвольных файлов/папок Windows (`sync.cfg`, `OFN_ALLOWMULTISELECT` + `IFileOpenDialog`). Новых внешних крейтов не добавлено (`docs/LICENSES.md` без изменений).
- **Причина**: Цельный современный интерфейс Windows 11, удобная работа с несколькими облаками и произвольной синхронизацией, мгновенный доступ к живой диагностике и истории версий в одном окне без роста потребления CPU/RAM.

---

## ADR-017: Двойной рендер шрифтов Windows 11 (`GDI ClearType` / `DirectWrite`), 4-вкладочные настройки, портативные ярлыки и стабильность `JobObject`/Fluent-меню (`v0.0.6`)

- **Статус**: Принято (v0.0.6, согласовано владельцем)
- **Решение**:
  1. Ввести в `AppearanceConfig` (`appearance.cfg`) и `FluentPalette` выбор движка рендера текста `FontRenderer` (`GdiClearType` по умолчанию | `DirectWrite`) с автоопределением системного шрифта Windows 11 `Segoe UI Variable` (`Display`/`Text`/`Small` с фолбэком `Segoe UI` на Windows 10). Для `DirectWrite` активирована только фича `Win32_Graphics_DirectWrite` существующего крейта `windows = "0.61"` (новых внешних крейтов нет, `docs/LICENSES.md` без изменений), с ленивой инициализацией `IDWriteFactory` только при выборе режима `DirectWrite`.
  2. Добавить флаг `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` в `JobObject` (`crates/platform-windows/src/job.rs`), чтобы сторонние приложения, запускаемые из воркера `--worker dock`, не завершались при закрытии `asrbox` (`BUG-006`).
  3. Убрать реентрантные вызовы Win32 внутри `RefCell::borrow_mut()` в `fluent_menu*.rs`/`settings_proc.rs`/`tools_state.rs` (`BUG-007`) и добавить запись паник в `data/logs/crash.log`.
  4. Копировать перетаскиваемые `.lnk`/`.url` ярлыки в портативную папку `./data/shortcuts/` (`crates/app/src/dock_shortcuts.rs`) с автомиграцией существующих внешних ярлыков и автоматической очисткой неиспользуемых копий.
  5. Обновлять живые окна док-панелей на месте (`reload_runtimes_in_place` без пересоздания `HWND` и перевыгрузки иконок) + дедупликация ползунка для устранения моргания; добавить отключаемую анимацию запуска ярлыка (`launch_animation_enabled`, 460 мс, 0% CPU в покое) и усилить активацию уже запущенной копии (`AllowSetForegroundWindow` + `SW_RESTORE` + `AttachThreadInput`).
  6. Оптимизировать навигацию окна настроек до 4 вкладок 1-в-1 с процессами (`Общие настройки`, `Док-панели`, `Виджеты` с переключателем `Google Календарь | Системный монитор`, `Игровой оверлей`), перенести «Изоляцию модулей» и «Перезапустить все модули» в «Общие настройки», карточку «МОДУЛЬ И ПРОЦЕСС» (статус/PID + Перезапуск + Вкл/Выкл) — в верх каждой модульной вкладки, а опрос метрик диагностики сделать строго ленивым (0 обращений вне вкладки «Диагностика»).
- **Причина**: Чёткая типографика Windows 11, прозрачное 1-к-1 управление модулями, неубиваемые ярлыки после очистки Рабочего стола, плавная настройка дока и устранение критических сбоев `BUG-006` и `BUG-007`.

---

## ADR-018: Встроенный ресурс иконки PE без зависимостей (`build.rs`), нативная палитра Windows 11 Settings, «Открыть расположение файла» и аудит стабильности (`v0.0.7`)

- **Статус**: Принято (v0.0.7, согласовано владельцем)
- **Решение**:
  1. Добавить zero-dependency `crates/app/build.rs`, который при сборке под `windows-msvc` генерирует стандартный 32-битный Win32 `.res` (`RT_ICON` для 16/24/32/48/64/128 px + `RT_GROUP_ICON #1`) и подключает его через `cargo:rustc-link-arg-bins` без новых внешних крейтов (`docs/LICENSES.md` без изменений). В `crates/platform-windows/src/app_icon.rs` загружать иконку через `LoadImageW(LR_SHARED)` по DPI-метрикам `SM_CXICON`/`SM_CXSMICON`, а фолбэк `CreateIconIndirect` перевести на bottom-up `BITMAPV5HEADER` (`BUG-008`).
  2. Приблизить палитру и карточки `mod-settings` к нативному Windows 11 Settings: единый фон `#202020` / `#F3F3F3`, раздельные скруглённые карточки `#2B2B2B` с зазором `3 px`, мягкая акцентная подсветка активных сегментов и клиппинг текста прокрутки по границе шапки.
  3. Добавить `DockMenuAction::OpenItemFolder(u64)` и `DockPlatform::open_item_location(&Path)` с разыменованием `.lnk` (`IShellLinkW`) и открытием Проводника `explorer.exe /select,"..."`.
  4. Закрыть найденные аудитом узкие места: кэшировать `SettingsSnapshot` в `AppController` (0 дисковых чтений на `MouseMove`/`Closed`), отправлять `PointerLeave` при внешней потере захвата `WM_CAPTURECHANGED`, вызывать `ReleaseCapture()` перед Fluent-меню/диалогами, использовать `"Segoe UI Variable"` без двойной гаммы в `DirectWrite` и чистить `data/shortcuts/` при удалении дока/значков.
- **Причина**: Корректное отображение фирменной иконки на всех системных поверхностях Windows (Проводник, Диспетчер задач, панель задач, трей), узнаваемый спокойный стиль Windows 11, удобный доступ к папке ярлыка и снижение лишнего I/O в UI без добавления зависимостей.

---

## ADR-019: Расширяемая дизайн-система «Стеклянный блок ASR», чистый Rust WinZip AES-256, GitHub-бэкап, настраиваемые меню и виджеты «Погода» и «Заметки» (`v0.0.8`)

- **Статус**: Принято (v0.0.8, согласовано владельцем)
- **Решение**:
  1. Ввести расширяемое перечисление стилей `DesignTheme` (`AsrGlass` по умолчанию + `Win11Classic`, `appearance.cfg`) с общим генератором фирменного логотипа «Стеклянный блок ASR» (`crates/platform-api/src/logo.rs` + PE-иконка `16..256 px` в `crates/app/build.rs`), аккордеон-секциями в Настройках (`settings_ui.cfg`), глобальным поиском по всем 4 разделам, Fluent-комбобоксами (`ComboBox`) и безопасным подтверждением сброса настроек внутри «Общие настройки».
  2. Добавить независимую блокировку значков док-панели (`lock_items` в `docks.cfg`) и отключить автопримагничивание к краям при свободном перемещении откреплённой панели (смена края — только через Настройки/ПКМ-меню).
  3. Реализовать опциональное шифрование `.zip`-бэкапов по стандарту WinZip AE-2 AES-256 на чистом Rust `std` (`crates/mod-backup/src/crypto.rs`, zero new crates, `docs/LICENSES.md` без изменений) с криптостойкой солью через `BCryptGenRandom` (активирована фича `Win32_Security_Cryptography` существующего `windows = "0.61"`) и хранением пароля исключительно в Windows Credential Manager (`asrbox:backup:zip_password`).
  4. Добавить раздел «Для разработчика · Бэкап исходного кода GitHub» (`github_backup.cfg`, PAT только в Windows Credential Manager `asrbox:github:token`, строгая валидация `owner/repo` и `branch`, фоновая загрузка `zipball` через WinHTTP с возможностью вложения в ZIP-бэкап).
  5. Вынести состав ПКМ-меню трея, док-панелей и виджетов в портативный `./data/config/menus.cfg` (`ContextMenuConfig`) и добавить кликабельные системные действия в виджете «Системный монитор» (`taskmgr.exe`, `explorer.exe shell:MyComputerFolder`, `resmon.exe`, `ms-settings:display`, `ncpa.cpl`, `msinfo32.exe`).
  6. Разместить два новых виджета — **Погода** (`WEATHER_WIDGET_ID = 3`, Open-Meteo без API-ключа, `weather.cfg`) и **Заметки и задачи** (`NOTES_WIDGET_ID = 4`, 3-колоночный офлайн-редактор `data/notes/notes_db.cfg` + `notes.cfg`) — внутри существующего воркера `--worker widgets` (`crates/mod-widgets`) без лишних процессов и без новых внешних крейтов.
- **Причина**: Полное сохранение бюджетов `NF-001..NF-005` (ни одного нового внешнего крейта, минимальный `.exe` и RAM), безопасность паролей/токенов через Credential Manager, расширяемость тем оформления и богатый набор автономных десктоп-виджетов в едином стиле.

---

## ADR-020: Надёжный сброс Fluent-меню (`WH_MOUSE_LL`), гибридный прогноз Яндекс.Погоды (`3–7д`) и визуальная спецификация Concentric Fluent Glass (`v0.0.10`)

- **Статус**: Принято (v0.0.10, согласовано владельцем)
- **Решение**:
  1. Устранить зависание контекстных меню на неактивирующих `WS_EX_NOACTIVATE`-окнах виджетов и доков (`BUG-012`): при открытии `FluentMenu` или `FolderPopup` устанавливать временный низкоуровневый хук мыши `WH_MOUSE_LL` (`MenuOutsideHookGuard` с гарантированным снятием в `Drop`), принудительно передавать фокус окну меню через `AttachThreadInput` + `SetForegroundWindow`, обрабатывать `WM_MENU_DISMISS`/`WM_CANCELMODE`/`WM_KILLFOCUS`/`WM_ACTIVATE(WA_INACTIVE)` и блокировать повторный вход ПКМ пока меню открыто.
  2. Решить ограничение бесплатного тарифа Яндекс.Погоды API v2 (`2` дня в `forecasts[]`, `BUG-013`) через прозрачный **гибридный режим**: текущая погода, почасовой прогноз и 1–2 дня берутся из Яндекс.Погоды, а при `< 7` днях автоматически выполняется фоновый добор дней `3–7` из Open-Meteo по координатам выбранного города (без ключа); в подвале виджета и в описании настроек Погоды явно отображается гибридный источник `Яндекс + Open-Meteo (3–7д)`.
  3. Довести док-панель, раздел «Док-панели» Настроек и трей-меню до визуальной спецификации **Concentric Fluent Glass**: `menu_bg` `0.94`/`0.96`, ползунки с кольцом `#FFFFFF` и капсульными бейджами `48×26 px`, встроенная шапка `[ Док N ⌄ ]` + `+ Новый док`, настройка `edge_margin_px`/`corner_radius_px`/`blur_enabled`, вертикальный список ярлыков/папок `46 px` (`↑`/`↓`/`✎`/`✕` с красным hover `#E81123`) и кнопкой `+ Добавить папку / ярлык`, верхний `1 px` стеклянный блик дока, стеклянный капсульный тултип `radius: 6 px` и веерный попап папок (`AsrboxFolderPopupWindow`, сетка до 4 колонок + `↗ Проводник`). Новых внешних крейтов не добавлено (`docs/LICENSES.md` без изменений).
- **Причина**: Полное исключение «залипания» всплывающих меню на Windows 10/11, полноценный 3- и 7-дневный прогноз даже на бесплатном тарифе Яндекс.Погоды и цельный визуальный язык Concentric Fluent Glass с 0% CPU в простое.

---

## ADR-021: Полная миграция UI на Slint (`crates/ui-slint`) + Windows 11 Mica (`window-vibrancy`), этапы 1+2 (`v0.1.0`)

- **Статус**: Принято (v0.1.0, согласовано владельцем)
- **Решение**:
  1. Выполнить полную миграцию слоя отрисовки интерфейса на декларативный Rust-фреймворк **Slint** в два пакета: Этап 1+2 (`v0.1.0` — единая дизайн-система `ui/theme.slint`, `ui/components/`, безрамочные окна `SettingsWindow` `860×560` и `NotesWindow` `780×520` с нативным Windows 11 Mica) и Этап 3+4 (виджеты рабочего стола, док-панель и трей-меню). Вся существующая Rust бизнес-логика (`mod-dock`, `mod-widgets`, `mod-overlay`, `mod-backup`, `mod-webdav`, `mod-debug`, `mod-about`, Credential Manager, WinHTTP, AES-256, GitHub) сохраняется на 100%.
  2. Выделить UI-слой в изолированный крейт `crates/ui-slint` (`asrbox-ui-slint`) с подключением внешних крейтов:
     - `slint = "1.18.1"` (`default-features = false`, фичи `["compat-1-2", "std", "backend-winit", "renderer-software", "unstable-winit-030"]`) — лицензия `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0`.
     - `slint-build = "1.18.1"` (`[build-dependencies]`) — лицензия `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0`.
     - `window-vibrancy = "0.8.1"` (под `[target.'cfg(windows)'.dependencies]`) — лицензия `MIT OR Apache-2.0`.
  3. Выбрать программный рендерер `renderer-software` (без тяжёлых Skia/FemtoVG/OpenGL), чтобы сохранить лимит `.exe < 25 МБ` (`NF-005`), исключить конфликты прозрачности с Windows 11 Mica/Acrylic и гарантировать `0.0% CPU` в простое (реактивная перерисовка только по событиям).
  4. Сгенерированный код `slint::include_modules!()` изолировать в `crates/ui-slint/src/generated.rs` (поэтому `crates/ui-slint` не наследует `unsafe_code = "forbid"`, сохраняя `unwrap_used = "deny"` и `expect_used = "deny"` во всём рукописном коде без единого `unsafe`).
  5. Запускать Slint-окна `SettingsWindow` и `NotesWindow` как on-demand дочерние процессы единого `asrbox.exe` (`--ui-settings`, `--ui-notes`, под `JobObject` супервизора с IPC через stdin/stdout), что исключает конфликт циклов событий Win32 `GetMessageW` и `winit` и обеспечивает `0 МБ RAM` / `0% CPU` при закрытых окнах.
- **Причина**: Точное соответствие эталонному дизайну Concentric Fluent Glass с настоящим Windows 11 Mica, безрамочной кастомной шапкой, плавной прокруткой, полноценным многострочным редактором Заметок и нулевой деградацией существующей бизнес-логики и ресурсных бюджетов.

---

## ADR-022: Этапы 3+4 миграции на Slint — виджеты, док-панель и предзагруженное трей-меню (`v0.2.0`)

- **Статус**: Принято (v0.2.0, согласовано владельцем)
- **Решение**:
  1. Завершить полную миграцию оставшихся поверхностей интерфейса в `crates/ui-slint` без добавления новых внешних крейтов (`docs/LICENSES.md` без изменений, добавлена только внутренняя workspace-зависимость `asrbox-platform-windows` под `cfg(windows)`):
     - **Этап 3 (`--worker widgets`)**: `WeatherWindow` (Open-Meteo + Яндекс Гибрид 3–7д, геокодинг), `SysMonitorWindow` (фоновый `SysTelemetryCollector` с паузой при скрытии -> `0% CPU`), `CalendarWindow` (Google `.ics` через WinHTTP + кэш). Все контекстные меню переведены на Slint `PopupWindow` (полное устранение залипания `BUG-012`).
     - **Этап 4 (`--worker dock` и `--ui-tray`)**: `DockWindow` (плавающая капсула Concentric Fluent Glass, нативные иконки Windows в `slint::Image`, hover-zoom, bounce, Drag-and-Drop файлов, свободное перетаскивание без автоприлипания к краям и `lock_items`) и предзагруженный резидентный `--ui-tray` (`TrayMenuWindow`, показываемый по IPC `TrayShowPayload` и скрывающийся по потере фокуса / `Esc`).
  2. На Windows 11 для виджетов, док-панели и трей-меню применяется аппаратное полупрозрачное стекло `apply_acrylic` (`window-vibrancy`), с фоллбеком на полупрозрачный `rgba`-фон.
  3. Сохранить `--inproc` и `--legacy-worker <dock|widgets|overlay>` в качестве резервных режимов запуска без удаления бизнес-логики и GDI+ fallback-кода.
  4. Добавить `[profile.release]` (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`) в корневой `Cargo.toml` для удержания `asrbox.exe` в пределах `NF-005 < 25 МБ` без урезания функций.
- **Причина**: Единый визуальный язык Concentric Fluent Glass на всех окнах `asrbox`, отсутствие конфликта циклов событий Win32/`winit`, мгновенное открытие трей-меню и соблюдение ресурсных бюджетов `NF-001..NF-005`.




