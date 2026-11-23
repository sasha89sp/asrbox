# CHANGELOG.md — История изменений `asrbox`

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), нумерация версий следует [Semantic Versioning](https://semver.org/lang/ru/).

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
- Расширение `crates/platform-api` (`WidgetPlatform`, `WidgetFrame`, `HttpFetchResult`) и `crates/platform-windows` (`http.rs` на WinHTTP, `widget_win.rs`, `widget_text.rs` с `Segoe UI` + `DrawTextW`, DWM backdrop, пункты виджета в трее).

### Исправлено
- `BUG-001`: устранены расхождения `cargo fmt --check` в `xtask`, `crates/mod-dock` и пустых `src/lib.rs` (закрыто после зелёного CI, перенесено в `docs/archive/bugs-closed.md`).
- `BUG-002` (ожидает CI): отформатирован workspace (`cargo fmt`), исправлены фичи/импорты `windows 0.61` (`Win32_Graphics_Dwm`, `Win32_Security`, `ClientToScreen`, `WM_MOUSELEAVE`) и предупреждения Clippy 1.100 в `icons.rs` и тесте `storage.rs`.

### Изменено
- Снято фиксированное ограничение «8 док-панелей» в пользу неограниченного числа независимых панелей (`ADR-007`, `ATLAS.md`, `ARCHITECTURE.md`, `crates/mod-dock/MAP.md`).
