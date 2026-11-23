# CHANGELOG.md — История изменений `asrbox`

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), нумерация версий следует [Semantic Versioning](https://semver.org/lang/ru/).

## [0.0.2] - 2026-11-23

### Добавлено
- **Отображение док-панели на рабочем столе (`mod-dock`, `DK-018`)**: настраиваемый слой окна (`layer=wallpaper|normal|topmost` в `docks.cfg`, по умолчанию `На рабочем столе / Обои`) с переключением из ПКМ-меню док-панели и вкладки «Док-панели» окна настроек (`HWND_BOTTOM` через `WM_WINDOWPOSCHANGING`).
- **Все события на сегодня в Google Календаре (`mod-widgets`, `WG-019`)**: события сегодняшнего дня отображаются полностью, включая уже прошедшие по времени (визуально приглушены, а в режиме `Вид: отсчёт` помечены как `завершено · HH:MM`).
- **Настройка частоты синхронизации календаря (`mod-widgets`, `WG-020`)**: выбор интервала обновления `.ics` (`5м / 15м / 30м / 1ч / 3ч`) в ПКМ-меню виджета и вкладке «Google Календарь» окна настроек (`refresh_interval_sec` в `widgets.cfg`).
- **Автозагрузка с Windows**: переключатель включения/отключения автозагрузки через реестр текущего пользователя (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\asrbox` без прав администратора) во вкладке «Модули» окна настроек и в ПКМ-меню трея.
- **Новый модуль бэкапа (`crates/mod-backup`)**: создание локальных `.zip`-архивов в `./data/backups/` без внешних зависимостей (стандартный STORE ZIP + IEEE CRC-32) в двух режимах — **только настройки** (`data/config/*`) и **вся программа + настройки** (`asrbox.exe` + `data/config/*`), доступно из вкладки «Бэкап и Облако» и меню трея.
- **Модуль WebDAV (`crates/mod-webdav` + WinHTTP + Windows Credential Manager)**: встроенные пресеты **Яндекс.Диск** (`https://webdav.yandex.ru`) и **Mail.ru** (`https://webdav.cloud.mail.ru`), поддержка собственного HTTPS WebDAV-сервера, безопасное хранение паролей приложений **исключительно в Windows Credential Manager** (пароли не пишутся в конфиги и логи), проверка подключения (`PROPFIND`), ручная выгрузка последнего бэкапа («Загрузить в облако») и автоматическая выгрузка сразу после создания бэкапа (`auto_upload_cloud`).
- **Фирменная иконка приложения в стиле NotePlan**: программно генерируемый 32-битный BGRA-значок `HICON` (16×16 для системного трея и заголовков + 32×32 для панели задач Windows) с тёплым коралловым акцентом, применённый к трею и всем окнам программы («Настройки», «Диагностика», «О программе»).

### Изменено
- **Единая концепция дизайна macOS NotePlan Dark** (`ADR-013`): виджеты («Google Календарь», «Системный монитор»), док-панели, единое окно настроек и окна «Диагностика» / «О программе» переведены на тёплую графитовую палитру NotePlan (`#1C1D22` полотно, `#16171B` сайдбар, `#23252C` карточки, `#30333E` тонкие границы, коралловый акцент `#E87A48`) с нулевым ростом нагрузки на CPU/RAM.
- Обновлена версия workspace в корневом `Cargo.toml` с `0.0.1` до `0.0.2` (автоматически отображается в трее, «О программе», `--version` и артефактах GitHub Actions).

## [0.0.1] - 2026-11-22

### Добавлено
- Корневой `.gitignore` с исключением `target/`, `data/`, `*.log` и локальных файлов настроек.
- Корневой `Cargo.toml` (`[workspace]`, `resolver = "2"`, `[workspace.package]` с `version = "0.0.1"` и `edition = "2021"`, lints `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`).
- Конфигурация `.cargo/config.toml` с алиасом `cargo xtask`.
- Каркас 11 крейтов в `crates/` (`core`, `platform-api`, `platform-windows`, `mod-keyboard`, `mod-dock`, `mod-speech`, `mod-webdav`, `mod-monitor`, `mod-screenshot`, `mod-huawei`, `app`) с минимальными `Cargo.toml`, пустыми `src/lib.rs` (в `crates/app` — `src/main.rs` с пустой `main`) и картами `MAP.md`.
- Утилита `xtask` (`cargo xtask atlas-check` / `cargo run -p xtask -- atlas-check`) без внешних зависимостей и без `unwrap()`/`expect()`.
- Полный пакет проектной документации по руководству: `AGENTS.md`, `SPEC.md` (с бюджетами `NF-001..NF-010`), `ATLAS.md`, `HANDOFF.yaml`, `ARCHITECTURE.md`, `DECISIONS.md`, `BUGS.md`, `CHANGELOG.md`.
- Каталоги и документы сопровождения: `specs/README.md`, `specs/mod-dock.md` (`DK-001..DK-010`), `docs/RELEASE_CHECKLIST.md`, `docs/LICENSES.md`, `docs/archive/bugs-closed.md`.
- Конвейеры GitHub Actions: `.github/workflows/ci.yml` (`windows-latest`, `Swatinem/rust-cache@v2`, `fmt`, `clippy`, `xtask atlas-check`, `test`, `build --release`, проверка лимита `< 25 МБ`, смоук `--selftest`, выгрузка артефакта) и `.github/workflows/release.yml` (проверка тега `v*`, переименование `asrbox-vX.Y.Z.exe`, SHA-256, GitHub Releases).
- Платформонезависимое ядро `crates/mod-dock` (`config.rs`, `layout.rs`, `render.rs`, `state.rs`, `storage.rs`, `lib.rs`): неограниченное количество независимых док-панелей (`DockRegistry`), привязка к 4 краям и центровка, drag-and-drop вставка, выбор источника иконки (`AutoShell` / `CustomFile` / `ExeResource`), событийное увеличение только наведённой иконки с 0 тиков в покое, 32-битный рендерер macOS-капсулы, портативное сохранение `./data/config/docks.cfg` и модульные тесты.
- Безопасные платформенные контракты в `crates/platform-api` (`DockPlatform`, `IconProvider`, `PlatformEvent`, `DockMenuAction`, `TrayAction`, `RgbaImage`, `BgraFrame`).
- Windows-слой в `crates/platform-windows` (`windows = "0.61"`, `ADR-008`): прозрачные layered-окна (`UpdateLayeredWindow`), `DragAcceptFiles`, извлечение чётких иконок (`IShellItemImageFactory`, WIC, `ExtractIconExW`), системный трей (`Shell_NotifyIconW`), ПКМ-меню настроек док-панели/значка, диалог выбора иконки (`GetOpenFileNameW`), запуск целей (`ShellExecuteW`) и `SingleInstanceGuard`.
- Исполняемый файл `crates/app/src/main.rs` и `crates/app/src/selftest.rs`: запуск без консольного окна (`windows_subsystem = "windows"`), `--version`, `--selftest`, `--safe-mode`, системный трей и цикл управления док-панелями и виджетами.
- Новый модуль `crates/mod-widgets` (`specs/mod-widgets.md`, `WG-001..WG-012`, `ADR-009`) и первый рабочий виджет — **Google Календарь (`.ics`, macOS Dark)**:
  - потоковый парсер RFC 5545 (`ics.rs`, unfolding, UTF-8, `VTIMEZONE`/UTC, `EXDATE`, `RECURRENCE-ID`, `STATUS:CANCELLED`, многодневные `VALUE=DATE`, быстрый пропуск архива на 7 000+ событиях, детектор `Busy`);
  - движок повторов `RRULE` (`rrule.rs`: `DAILY`/`WEEKLY`/`MONTHLY`/`YEARLY`, `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` включая `2MO`/`-1FR`, `BYMONTHDAY`, `BYMONTH`, `WKST`);
  - интерфейс macOS Dark (`render.rs`, `format.rs`, `calendar.rs`): перетаскивание за шапку с сохранением `(X, Y)`, слои (`На обоях` / `Обычное` / `Поверх всех`), опциональный Acrylic, фильтры `Сегодня | 7д | 30д | Все`, 3 формата сроков (`дата`, `23.11`, `отсчёт`), прокрутка колёсиком и `▲ / ▼`, tooltip, клики на Google Calendar, диагностика ошибок сети/HTTP/Busy;
  - локальный кэш `./data/cache/calendar_cache_<id>.ics` и настройки `./data/config/widgets.cfg` (`storage.rs`) с поддержкой нескольких календарей и цветов.
- Расширение `crates/platform-api` (`WidgetPlatform`, `WidgetFrame`, `HttpFetchResult`, `ToolsPlatform`, `ProcessMetrics`) и `crates/platform-windows` (`http.rs` на WinHTTP с `AUTOMATIC_PROXY` и таймаутами, `widget_win.rs`, `widget_text.rs` с `Segoe UI` + `DrawTextW`, `metrics.rs`, `tools_win.rs`).
- Док-панель (`mod-dock`, `DK-011..DK-012`): macOS-разделители (`separator=<id>` в `docks.cfg`, добавление/удаление через ПКМ) и встроенная кнопка настроек (`⚙`) на панели с открытием `data/config/docks.cfg` в Блокноте и пунктом «Перечитать настройки».
- Новый отладочный модуль `crates/mod-debug` (`specs/mod-debug.md`, `DB-001..DB-007`): окно «Диагностика и логи...» из трея с потоком логов в реальном времени, переключателями `TRACE` и записи в `data/logs/asrbox-debug.log` (ротация 2 МБ), фильтром по модулям, автоматической редакцией секретов, живыми метриками процесса (RAM, % CPU, GDI/USER, uptime), помодульной статистикой нагрузки и экспортом отчёта в буфер обмена или `data/logs/diag-report.txt`.
- Новый модуль `crates/mod-about` (`specs/mod-about.md`, `AB-001..AB-004`): окно «О программе...» из трея с версией, описанием портативного режима, актуальными путями на диске (`asrbox.exe`, `data\`, `docks.cfg`, `widgets.cfg`, `cache\`, `logs\`), встроенной историей версий (`CHANGELOG.md`) и кнопками «Открыть папку данных» / «Копировать пути».
- Новый модуль `crates/mod-settings` (`specs/mod-settings.md`, `ST-001..ST-006`, `ADR-011`): единое изменяемое окно настроек в тёмном стиле macOS с вкладками «Модули», «Док-панели», «Google Календарь», «Диагностика» и «О программе», ползунками, переключателями и нативным полем вставки `.ics` URL.
- Мультипроцессная изоляция модулей и вкл/выкл (`crates/core/src/modules_cfg.rs`, `ipc.rs`, `crates/app/src/supervisor.rs`, `worker.rs`, `crates/platform-windows/src/job.rs`): запуск `mod-dock` и `mod-widgets` в отдельных процессах (`--worker dock|widgets`) под Win32 `JobObject` с автоперезапуском при сбое, живым переключением `Процессы / Один процесс` и полным отключением ненужных модулей (`data/config/modules.cfg`).
- Расширение `mod-dock` (`DK-013..DK-017`): перетаскивание значков и разделителей внутри панели и между панелями, удаление значка/разделителя выносом на рабочий стол, индикатор запущенных приложений (точка) и активация открытого окна по клику (`EnumWindows` + `IShellLinkW`), закрепление/открепление панели с ручкой перемещения и привязкой `Offset(px)` (без наслоения двух панелей), настраиваемая прозрачность (`20–100%`) и произвольный размер иконок (`24–128 px`).
- Новый виджет рабочего стола **Системный монитор** (`crates/mod-widgets/src/sys_*`, `WG-014..WG-018`, `ADR-012`) и единый стиль macOS Dark-Glass для обоих виджетов:
  - реальная телеметрия Windows в фоновом потоке (`sys_telemetry.rs`, `sys_procs.rs`, `sys_gpu.rs`, `sys_net.rs`): дата/время/аптайм/профиль питания, CPU (% + частота ГГц + потоки + топ-процесс), GPU (частота экрана Гц, динамический NVML для NVIDIA + DXGI VRAM для AMD/Intel), ОЗУ (использовано/всего/кэш/PF + топ-процесс), до 3 дисков (чтение/запись МБ/с, занятость %, свободно), сеть (IPv4 + `[VPN]`, RX/TX Мбит/с, пинг шлюза и `1.1.1.1` с jitter и потерями, таймер ОС) и Топ-5 процессов по CPU и ОЗУ;
  - настраиваемая частота обновления (`1с / 2с / 3с / 5с / 10с`, по умолчанию `2с`) в `./data/config/sysmon.cfg`, ПКМ-меню виджета и вкладке «Системный монитор» окна настроек (при скрытии виджета опрос полностью останавливается);
  - прямое изменение размеров обоих виджетов (Календарь и Системный монитор) мышью за любые края и углы окна (`widget_proc.rs`, `WidgetResized`) + ползунки размеров и кнопки «Выровнять с Монитором» / «Выровнять с Календарём» в `mod-settings`.

### Исправлено
- `BUG-001`: устранены расхождения `cargo fmt --check` в `xtask`, `crates/mod-dock` и пустых `src/lib.rs` (закрыто после зелёного CI, перенесено в `docs/archive/bugs-closed.md`).
- `BUG-002`: отформатирован workspace (`cargo fmt`), исправлены фичи/импорты `windows 0.61` (`Win32_Graphics_Dwm`, `Win32_Security`, `ClientToScreen`, `WM_MOUSELEAVE`) и предупреждения Clippy 1.100 в `icons.rs` и тесте `storage.rs` (подтверждено CI run `45127781497`, перенесено в `docs/archive/bugs-closed.md`).
- `BUG-003` (ожидает подтверждения лога CI): шаг `Smoke test (--selftest)` в `.github/workflows/ci.yml` переведён на `Start-Process -Wait -PassThru` для корректного ожидания GUI-бинарника (`windows_subsystem = "windows"`) и чтения `$proc.ExitCode`.
- `BUG-004` (ожидает CI и ручной проверки): устранён лаг меню трея и UI — `push_event` мгновенно будит `GetMessageW` через `WM_WAKE_EVENTS`, добавлен `PostMessageW(WM_NULL)` после `TrackPopupMenuEx` (KB135788), кэширован `timer_active` в `DockRuntime` и добавлен ранний выход в `CalendarWidget::on_pointer_move`.
- `BUG-005` (ожидает CI и ручной проверки): исправлено открытие настроек виджета (запуск `notepad.exe` без перезаписи существующего `widgets.cfg`), парсер `widgets.cfg` расширен и дополнен детектором закомментированных реальных ссылок `# calendar=...` с предупреждением в виджете, кнопкой «Включить (убрать #)» и вводом URL в окне настроек; WinHTTP переведён на `AUTOMATIC_PROXY` с таймаутами и кодами ошибок.

### Изменено
- Снято фиксированное ограничение «8 док-панелей» в пользу неограниченного числа независимых панелей (`ADR-007`, `ATLAS.md`, `ARCHITECTURE.md`, `crates/mod-dock/MAP.md`).
- В виджете Календарь удалено системное размытие Acrylic (`DwmSetWindowAttribute` / `Win32_Graphics_Dwm`) и заменено на настраиваемую прозрачность фона `opacity=20..100` (по умолчанию `88%`) с пресетами `100% / 85% / 70% / 50% / 35%` в ПКМ-меню (`ADR-010`).
- Оформление виджета Календарь приведено к единому стилю macOS Dark-Glass с новым виджетом Системный монитор (единая подложка, пилюли, разделители и ручка изменения размера).
- ПКМ-меню системного трея сгруппировано по подменю модулей (`Настройки asrbox...`, `Модуль: Док-панели ▶`, `Модуль: Виджеты ▶` с Календарём и Системным монитором, `Модуль: Диагностика и логи...`, `Модуль: О программе...`).


