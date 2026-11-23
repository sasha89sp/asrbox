# ATLAS.md — Карта проекта `asrbox` (уровень L0)

Проверка актуальности: `cargo xtask atlas-check` (или `cargo run -p xtask -- atlas-check`). Лимит документа: <= 150 строк.

## 1. Крейты workspace

| Путь | Пакет | Режим | Ответственность | Карта L1 |
| --- | --- | --- | --- | --- |
| `xtask` | `xtask` | CLI | Служебные команды workspace (`atlas-check`). | `xtask/MAP.md` |
| `crates/core` | `asrbox-core` | lib | Настройки, шина событий, трейт `Module`, логи, профилировщик. | `crates/core/MAP.md` |
| `crates/platform-api` | `asrbox-platform-api` | lib | Трейты `KeyboardHook`, `DockWindow`, `ScreenCapture`, `ProcessMonitor` (без `unsafe`). | `crates/platform-api/MAP.md` |
| `crates/platform-windows` | `asrbox-platform-windows` | lib | Реализация `platform-api` через `windows-rs` (единственное место с `unsafe`). | `crates/platform-windows/MAP.md` |
| `crates/mod-keyboard` | `asrbox-mod-keyboard` | process | Переназначение клавиш и аварийное отключение хука. | `crates/mod-keyboard/MAP.md` |
| `crates/mod-dock` | `asrbox-mod-dock` | process (1) | Неограниченное число док-панелей (стиль macOS) в одном процессе. | `crates/mod-dock/MAP.md` |
| `crates/mod-speech` | `asrbox-mod-speech` | process | Голос в текст (Whisper, ленивая загрузка + SHA-256). | `crates/mod-speech/MAP.md` |
| `crates/mod-webdav` | `asrbox-mod-webdav` | process | WebDAV и синхронизация папок. | `crates/mod-webdav/MAP.md` |
| `crates/mod-monitor` | `asrbox-mod-monitor` | on-open | Онлайн-монитор нагрузки (CPU/RAM/события/функции) и процессов. | `crates/mod-monitor/MAP.md` |
| `crates/mod-screenshot` | `asrbox-mod-screenshot` | on-demand | Скриншоты (процесс по запросу). | `crates/mod-screenshot/MAP.md` |
| `crates/mod-huawei` | `asrbox-mod-huawei` | process | Модем Huawei (AT-команды). | `crates/mod-huawei/MAP.md` |
| `crates/mod-widgets` | `asrbox-mod-widgets` | process | Виджеты рабочего стола (macOS Dark), первый виджет — Google Календарь (`.ics`). | `crates/mod-widgets/MAP.md` |
| `crates/app` | `asrbox` | supervisor | Точка входа `asrbox.exe`, супервизор (`--module`, `--safe-mode`, `--selftest`), трей, настройки. | `crates/app/MAP.md` |

## 2. Куда смотреть

| Задача / область | Путь |
| --- | --- |
| Главные правила для ИИ, запреты и протокол закрытия | `AGENTS.md` |
| Текущее состояние, статус CI и следующий шаг | `HANDOFF.yaml` |
| Специфика проекта и бюджеты `NF-001..NF-010` | `SPEC.md` |
| Детальные спецификации и критерии приёмки модулей | `specs/README.md`, `specs/mod-dock.md`, `specs/mod-widgets.md` |
| Модули, трейты, процессы, IPC (Named Pipes) и защита | `ARCHITECTURE.md` |
| Архитектурные решения (ADR) и обоснования зависимостей | `DECISIONS.md` |
| Открытые баги и журнал попыток | `BUGS.md` |
| Архив закрытых багов | `docs/archive/bugs-closed.md` |
| История изменений по версиям | `CHANGELOG.md` |
| Чеклист релиза и реестр лицензий | `docs/RELEASE_CHECKLIST.md`, `docs/LICENSES.md` |
| CI и релизный пайплайн GitHub Actions | `.github/workflows/ci.yml`, `.github/workflows/release.yml` |
| Алиас `cargo xtask` и настройки Cargo | `.cargo/config.toml` |
