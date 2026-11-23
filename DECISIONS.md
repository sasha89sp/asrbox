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

