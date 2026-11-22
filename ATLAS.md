# ATLAS.md — Карта репозитория `asrbox` (уровень L0)

Документ поддерживается в актуальном состоянии автоматически проверяемой командой:
`cargo run -p xtask -- atlas-check`

## 1. Карта крейтов workspace (L0)

| Путь крейта | Имя пакета | Слой | Ответственность | Карта файлов |
| --- | --- | --- | --- | --- |
| `xtask` | `xtask` | Tooling | Самопроверка репозитория (`atlas-check`: покрытие крейтов в `ATLAS.md` и `.rs`-файлов в `MAP.md`). | `xtask/MAP.md` |
| `crates/core` | `asrbox-core` | Core | Доменная модель, ошибки, конфиг, шина событий, контракты логирования и профилирования. | `crates/core/MAP.md` |
| `crates/platform-api` | `asrbox-platform-api` | Platform API | Платформонезависимые трейты взаимодействия с ОС (без `unsafe`). | `crates/platform-api/MAP.md` |
| `crates/platform-windows` | `asrbox-platform-windows` | Platform Impl | Реализация `platform-api` для Windows 10/11 x64; единственный крейт с `unsafe`. | `crates/platform-windows/MAP.md` |
| `crates/mod-keyboard` | `asrbox-mod-keyboard` | Module | Хоткеи, раскладки, перехват и обработка клавиатурных событий. | `crates/mod-keyboard/MAP.md` |
| `crates/mod-dock` | `asrbox-mod-dock` | Module | Панель быстрого запуска и управления окнами (dock). | `crates/mod-dock/MAP.md` |
| `crates/mod-speech` | `asrbox-mod-speech` | Module | Распознавание и синтез речи (ASR/TTS). | `crates/mod-speech/MAP.md` |
| `crates/mod-webdav` | `asrbox-mod-webdav` | Module | Синхронизация и обмен файлами по протоколу WebDAV. | `crates/mod-webdav/MAP.md` |
| `crates/mod-monitor` | `asrbox-mod-monitor` | Module | Онлайн-монитор нагрузки (CPU, RAM, потоки, профилировщик функций). | `crates/mod-monitor/MAP.md` |
| `crates/mod-screenshot` | `asrbox-mod-screenshot` | Module | Захват, обработка и сохранение скриншотов. | `crates/mod-screenshot/MAP.md` |
| `crates/mod-huawei` | `asrbox-mod-huawei` | Module | Интеграция с устройствами и сервисами Huawei. | `crates/mod-huawei/MAP.md` |
| `crates/app` | `asrbox` | App | Composition root, сборка `asrbox.exe`, окно, сворачивание в трей по `X`. | `crates/app/MAP.md` |

## 2. Таблица «Куда смотреть»

| Если нужно найти или изменить… | Куда смотреть |
| --- | --- |
| Правила кодирования, запрет `unwrap`/`expect`, границы `unsafe` и модулей, протокол закрытия | `AGENTS.md` |
| Специфика проекта и бюджеты `NF-001..NF-010` (логи, профилирование, монитор, трей по `X`) | `SPEC.md` |
| Архитектурные слои, граф зависимостей и схема каталога `./data/` | `ARCHITECTURE.md` |
| Архитектурные решения (ADR) и причины выбора подходов | `DECISIONS.md` |
| Текущий статус версии, прогресс и следующие задачи для продолжения работы | `HANDOFF.yaml` |
| Известные дефекты и ограничения среды | `BUGS.md` |
| История изменений по версиям | `CHANGELOG.md` |
| Чеклист подготовки релиза портативной сборки | `docs/RELEASE_CHECKLIST.md` |
| Пайплайн сборки, форматирования, Clippy и тестов на Windows | `.github/workflows/ci.yml` |
| Проверку соответствия `ATLAS.md` и `MAP.md` | `xtask/src/main.rs` |
| Конфигурацию, шину событий, общие типы ошибок, контракты логов/профилировщика | `crates/core` |
| Трейты доступа к ОС (трей, окна, хоткеи, захват экрана, системные метрики) | `crates/platform-api` |
| Вызовы WinAPI (`WM_CLOSE` -> трей, хуки, скриншоты, метрики процесса) | `crates/platform-windows` |
| Логику конкретного функционального модуля | `crates/mod-<имя>` |
| Инициализацию приложения, сборку зависимостей и цикл событий `asrbox.exe` | `crates/app` |
