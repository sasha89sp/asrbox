# ATLAS.md — Карта проекта `asrbox` (уровень L0)

Проверка актуальности: `cargo xtask atlas-check` (или `cargo run -p xtask -- atlas-check`). Лимит документа: <= 150 строк.

## 1. Крейты workspace

| Путь | Пакет | Режим | Ответственность | Карта L1 |
| --- | --- | --- | --- | --- |
| `xtask` | `xtask` | CLI | Служебные команды workspace (`atlas-check`). | `xtask/MAP.md` |
| `crates/core` | `asrbox-core` | lib | Трейт `Module`, конфиг модулей (`modules.cfg`, `IsolationMode`) и протокол IPC (`SupervisorCmd`/`WorkerMsg`). | `crates/core/MAP.md` |
| `crates/platform-api` | `asrbox-platform-api` | lib | Безопасные трейты `DockPlatform`, `WidgetPlatform`, `ToolsPlatform`, `IconProvider`, `FluentPalette`, `AppearanceConfig`, `FontRenderer` и `FluentMenuSpec` (без `unsafe`). | `crates/platform-api/MAP.md` |
| `crates/platform-windows` | `asrbox-platform-windows` | lib | Реализация `platform-api` через `windows-rs` (`JobObject` с breakaway, layered-окна, Fluent `HICON` и Fluent-меню, шрифты GDI ClearType + DirectWrite / `Segoe UI Variable`, `.lnk`/WIC иконки, WinHTTP+WebDAV/PROPFIND/sync, Credential Manager, автозагрузка, телеметрия; единственный `unsafe`). | `crates/platform-windows/MAP.md` |
| `crates/mod-keyboard` | `asrbox-mod-keyboard` | process | Переназначение клавиш и аварийное отключение хука. | `crates/mod-keyboard/MAP.md` |
| `crates/mod-dock` | `asrbox-mod-dock` | process (1) | Неограниченное число независимых док-панелей (Windows 11 Fluent, Light/Dark): 4 режима имён, слой, drag-перенос/удаление, защита от второй копии, анимация запуска, прозрачность и размер. | `crates/mod-dock/MAP.md` |
| `crates/mod-speech` | `asrbox-mod-speech` | process | Голос в текст (Whisper, ленивая загрузка + SHA-256). | `crates/mod-speech/MAP.md` |
| `crates/mod-backup` | `asrbox-mod-backup` | on-demand | Портативные `.zip`-бэкапы настроек и программы (`data/backups/`, STORE ZIP + CRC-32) с авто/ручной выгрузкой на подключённые WebDAV-серверы. | `crates/mod-backup/MAP.md` |
| `crates/mod-webdav` | `asrbox-mod-webdav` | on-demand | Неограниченное число WebDAV-серверов (`Яндекс.Диск`, `Mail.ru`, `Свой HTTPS`, миграция v1->v2), парсер `PROPFIND` и периодическая синхронизация файлов/папок Windows (`sync.cfg`). Пароли — только в Credential Manager. | `crates/mod-webdav/MAP.md` |
| `crates/mod-monitor` | `asrbox-mod-monitor` | on-open | Онлайн-монитор нагрузки (CPU/RAM/события/функции) и процессов. | `crates/mod-monitor/MAP.md` |
| `crates/mod-screenshot` | `asrbox-mod-screenshot` | on-demand | Скриншоты (процесс по запросу). | `crates/mod-screenshot/MAP.md` |
| `crates/mod-huawei` | `asrbox-mod-huawei` | process | Модем Huawei (AT-команды). | `crates/mod-huawei/MAP.md` |
| `crates/mod-widgets` | `asrbox-mod-widgets` | process | Виджеты рабочего стола в стиле Windows 11 Fluent (Light/Dark) с ресайзом: Google Календарь и Системный монитор. | `crates/mod-widgets/MAP.md` |
| `crates/mod-overlay` | `asrbox-mod-overlay` | process | Игровой HUD-оверлей (FPS/1% low, GPU/VRAM, CPU, RAM, NET/PL): автозапуск в играх, FSO/Topmost, Fluent-палитра, авто-выход из setup, 100% click-through, углы/своя точка, прозрачность и ресайз. | `crates/mod-overlay/MAP.md` |
| `crates/mod-debug` | `asrbox-mod-debug` | on-open | Отладочный модуль: живые логи по модулям, переключатель TRACE/файл, метрики RAM/CPU/GDI и диагностический отчёт (вкладка «Диагностика»). | `crates/mod-debug/MAP.md` |
| `crates/mod-about` | `asrbox-mod-about` | on-open | Модуль истории версий и путей: портативный статус, текущие пути на диске и встроенная история версий (`CHANGELOG.md`). | `crates/mod-about/MAP.md` |
| `crates/mod-settings` | `asrbox-mod-settings` | on-open | Единое Fluent-окно настроек (4 раздела: Общие настройки, Док-панели, Виджеты [Календарь\|Монитор], Игровой оверлей + браузерные вкладки Диагностика, История версий, Обзор WebDAV). | `crates/mod-settings/MAP.md` |
| `crates/app` | `asrbox` | supervisor | Точка входа `asrbox.exe`, crash-логгер, супервизор с `JobObject` (`--worker dock|widgets|overlay`), портативные ярлыки `data/shortcuts/`, плавная тема, бэкап/WebDAV/синхронизация, Fluent-трей. | `crates/app/MAP.md` |

## 2. Куда смотреть

| Задача / область | Путь |
| --- | --- |
| Главные правила для ИИ, запреты и протокол закрытия | `AGENTS.md` |
| Текущее состояние, статус CI и следующий шаг | `HANDOFF.yaml` |
| Специфика проекта и бюджеты `NF-001..NF-010` | `SPEC.md` |
| Детальные спецификации и критерии приёмки модулей | `specs/README.md`, `specs/mod-dock.md`, `specs/mod-widgets.md`, `specs/mod-overlay.md`, `specs/mod-settings.md`, `specs/mod-debug.md`, `specs/mod-about.md` |
| Модули, трейты, процессы, IPC и защита | `ARCHITECTURE.md` |
| Архитектурные решения (ADR) и обоснования зависимостей | `DECISIONS.md` |
| Открытые баги и журнал попыток | `BUGS.md` |
| Архив закрытых багов | `docs/archive/bugs-closed.md` |
| История изменений по версиям | `CHANGELOG.md` |
| Чеклист релиза и реестр лицензий | `docs/RELEASE_CHECKLIST.md`, `docs/LICENSES.md` |
| CI и релизный пайплайн GitHub Actions | `.github/workflows/ci.yml`, `.github/workflows/release.yml` |
| Алиас `cargo xtask` и настройки Cargo | `.cargo/config.toml` |
