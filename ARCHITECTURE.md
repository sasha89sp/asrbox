# ARCHITECTURE.md — Архитектура `asrbox`

Версия: `0.2.0` (завершены все 4 этапа миграции UI на декларативный Slint `~1.18` + Windows 11 Mica/Acrylic `window-vibrancy 0.8.1` в `crates/ui-slint`: Настройки `--ui-settings`, 3-колоночные Заметки `--ui-notes`, предпрогретое меню трея `--ui-tray`, виджеты рабочего стола `--worker widgets` и док-панели `--worker dock`; на 100% сохранены Rust-бэкенды `mod-dock`, `mod-widgets`, `mod-overlay`, `mod-backup` [AES-256 + GitHub], `mod-webdav`, `mod-debug`, `mod-about` и супервизор `crates/app`).

## 1. Правило зависимостей (строго вниз)

- `crates/mod-*` и `crates/platform-windows` зависят **только** от `crates/core` и `crates/platform-api`.
- Прикладные модули не вызывают друг друга напрямую — только сообщения через `crates/core` и супервизор `crates/app`.
- Весь `unsafe` и прямые вызовы WinAPI (`windows = "0.61"`) изолированы исключительно в `crates/platform-windows`; в остальных крейтах действует `unsafe_code = "forbid"` (в `crates/ui-slint` разрешён только сгенерированный макрос `slint::include_modules!()` в изолированном `src/generated.rs`).
- Во всём workspace действуют `unwrap_used = "deny"` и `expect_used = "deny"`.

```text
                      ┌───────────────────────────────┐
                      │          crates/app           │  Супервизор, JobObject, Fluent-трей,
                      │ (asrbox.exe / --worker <mod>) │  Backup/WebDAV/Sync (--inproc/--selftest)
                      └───────────────┬───────────────┘
          ┌───────────────────────────┼───────────────────────────┐
          ▼                           ▼                           ▼
┌───────────────────┐       ┌──────────────────┐       ┌─────────────────────────┐
│   crates/mod-*    │       │   crates/core    │       │ crates/platform-windows │
│ dock, widgets,    │       │ Module, IPC,     │       │  windows-rs, WinAPI     │
│ settings, backup, │       │ modules.cfg      │       │  FluentMenu/WinHTTP/    │
│ webdav, debug, ...│       └────────▲─────────┘       │  CredMgr (ONLY unsafe)  │
└─────────┬─────────┘                │                 └────────────┬────────────┘
          └──────────────┬───────────┴─────────────────┐            │
                         ▼                             ▼            ▼
                 ┌───────────────┐            ┌────────────────────────────┐
                 │  crates/core  │            │    crates/platform-api     │
                 └───────────────┘            │ DockPlatform, WidgetPlatform│
                                              │ ToolsPlatform, FluentPalette│
                                              │ (no unsafe)                │
                                              └────────────────────────────┘
```

## 2. Модель модулей, процессов и IPC

- **Единый контракт**: каждый модуль реализует трейт `Module` из `crates/core` (`start`, `stop`, `health`).
- **Управление в настройках (`data/config/modules.cfg`)**: каждый модуль (`dock`, `widgets`, `overlay`) включается и отключается независимо (выключенный не запускается, 0% CPU и 0 МБ RAM, `NF-007`), а режим изоляции (`process` по умолчанию или `inproc`) переключается на лету в окне «Настройки asrbox».
- **Один исполняемый файл**: `asrbox.exe` выступает супервизором (системный трей, `mod-backup`, `mod-webdav`, `mod-debug`, `mod-about`), запускает изолированные Slint-воркеры `asrbox.exe --worker <dock|widgets> --parent-pid <PID>`, игровой оверлей `--worker overlay`, предпрогретое меню трея `--ui-tray` и on-demand Slint-окна `--ui-settings` / `--ui-notes` (`SlintUiBridge` под `JobObject` с построчным `UI_CMD`/`STATUS`/`SHOW` IPC; резервный флаг `--legacy-worker <mod>` сохраняет старый Win32-рендер).
- **Гарантия очистки (`JobObject`)**: все воркеры и Slint-окна привязаны к Win32 `JobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`) + проверяют живость родительского PID: при выходе супервизора дочерние процессы гарантированно завершаются без «сирот», а сторонние программы пользователя, запущенные через док-панель, отвязываются от Job и продолжают работать (`BUG-006`).
- **Политика перезапуска (`NF-006`, `NF-008`)**: при аварийном завершении воркера остальные модули и супервизор продолжают работать; супервизор автоматически перезапускает упавший модуль (до 3 попыток за 60 секунд, затем останавливает с записью в лог диагностики).
- **IPC (`crates/core/src/ipc.rs`, `crates/ui-slint/src/ipc_proto.rs`)**: построчный текстовый протокол (`SupervisorCmd`/`WorkerMsg` для воркеров и `UI_CMD`/`STATUS`/`SHOW`/`HIDE`/`REFRESH`/`TAB`/`SHUTDOWN` для Slint-окон), пробуждающий цикл сообщений через `WM_IPC_WAKE` без поллинга.

### Режимы исполнения модулей

| Модуль | Режим по умолчанию | Причина изоляции |
| --- | --- | --- |
| `ui-slint` | Воркеры (`--worker dock`, `--worker widgets`) + дочерние окна (`--ui-settings`, `--ui-notes`, `--ui-tray`) | Полный декларативный UI на Slint (`~1.18`, `backend-winit`, `renderer-software`) + Windows 11 Mica/Acrylic (`window-vibrancy 0.8.1`), Concentric Fluent Glass, нативные `PopupWindow` ПКМ-меню (`close-on-click-outside`); 0% CPU в простое. |
| `mod-dock` | Логика/конфиг + Slint-воркер `--worker dock` | Неограниченное число док-панелей в одном процессе: реальные иконки Windows (`WindowsIconProvider` -> `Rgba8Pixel`), hover-zoom, точка запущенных окон, 4 режима подписей, drag-and-drop, защита `lock_items`, свободный drag без автопримагничивания (`NF-006`). |
| `mod-widgets` | Логика/парсеры + Slint-воркер `--worker widgets` | Виджеты рабочего стола (`WeatherWindow` Open-Meteo + гибридная Яндекс.Погода `3–7д`, `SysMonitorWindow` с ленивым `SysTelemetryCollector`, `CalendarWindow` `.ics` + кэш) и модель данных `NotesDb` для `--ui-notes` (`NF-007`). |
| `mod-overlay` | Отдельный процесс (`--worker overlay`) | Игровой HUD-оверлей (FPS/1% low, GPU/VRAM, CPU, RAM, NET/PL) с автозапуском только в играх, FSO/Topmost и 100% click-through без инъекции DLL (`OV-001..OV-009`). |
| `mod-settings` | Библиотека состояния/действий | Контракт `SettingsAction`, хранение состояния аккордеонов (`settings_ui.cfg`) и кроссплатформенные тесты. |
| `mod-backup` | По запросу (`on-demand`) | Портативные `.zip`-бэкапы настроек и программы (`data/backups/`, STORE ZIP + опциональное шифрование WinZip AE-2 AES-256 через `BCryptGenRandom`) и бэкап исходников GitHub (`github_backup.cfg`, PAT в Credential Manager) без внешних крейтов; 0% CPU в простое. |
| `mod-webdav` | По запросу (`on-demand`) | Неограниченное число серверов WebDAV (`Яндекс.Диск` / `Mail.ru` / `Свой HTTPS`), обзор/создание папок (`PROPFIND`/`MKCOL`) и синхронизация файлов/папок Windows (`sync.cfg`) с паролями только в Credential Manager. |
| `mod-debug` | Окно/вкладка по открытию (`on-open`) | Живые логи супервизора и всех воркеров (`app`, `dock`, `widgets`, `overlay`, `http`, `platform`), переключаемый 1 Гц монитор RAM/CPU/GDI/USER и экспорт отчёта. |
| `mod-about` | Вкладка по открытию (`on-open`) | Версия, портативный режим, живые пути на диске и встроенная полная история версий (`CHANGELOG.md`). |
| `mod-keyboard` / `speech` / `huawei` / `screenshot` / `monitor` | Отдельный процесс / по запросу | Зарезервированы каркасами workspace под следующие этапы. |

## 3. Защита при работе и хранение данных

- **Портативный каталог**: все данные лежат в `./data/` рядом с `asrbox.exe` (`data/config/`, `data/notes/`, `data/shortcuts/`, `data/logs/`, `data/cache/`, `data/backups/`, `data/models/`). Замена `asrbox.exe` в той же папке сохраняет все конфиги, заметки, локальные копии ярлыков и кэши.
- **Автозагрузка (опционально)**: при явном включении пользователем создаётся единственная запись `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\asrbox` на текущий путь `asrbox.exe`; при выключении запись удаляется.
- **Один экземпляр**: именованный Windows mutex предотвращает запуск второй копии и активирует уже работающее окно.
- **Безопасный режим**: запуск с `--safe-mode` или зажатой клавишей `Shift` поднимает только системный трей без загрузки модулей.
- **Аварийное отключение клавиатуры**: жёстко заданная, не переназначаемая комбинация клавиш мгновенно останавливает `mod-keyboard`.
- **Кнопка `X`**: скрывает окно в системный трей; полный выход доступен только из меню трея.
- **Настройки и бэкапы**: локальные бэкапы (только настройки или `asrbox.exe` + настройки + опциональный `github-source.zip`, с опциональным шифрованием AES-256) сохраняются в `./data/backups/` с авто/ручной выгрузкой в WebDAV; сброс настроек требует явного подтверждения и не затрагивает `data/backups/`, `data/notes/` и Credential Manager.
- **Секреты и Whisper**: пароли WebDAV, пароль AES-256 бэкапов (`asrbox:backup:zip_password`), GitHub PAT (`asrbox:github:token`) и опциональный ключ Яндекс.Погоды (`asrbox:weather:yandex_key`) хранятся исключительно в Windows Credential Manager (`CredWriteW`/`CredReadW`/`CredDeleteW`); секретные URL `.ics` и токены автоматически маскируются в логах `mod-debug`; модель Whisper скачивается лениво при первом включении с проверкой SHA-256.

## 4. Наблюдаемость и диагностика (`NF-010`)

- **Живая диагностика (`mod-debug`, пункт трея «Диагностика и логи...»)**:
  - кольцевой буфер последних 1500 записей по всем модулям (`app`, `dock`, `widgets`, `http`, `platform`, `debug`, `about`);
  - переключение подробных логов `TRACE` и записи в файл `data/logs/asrbox-debug.log` (ротация при 2 МБ) на лету без перезапуска;
  - фильтр по модулю, автоматическая редакция секретных ссылок/токенов;
  - живые метрики процесса (RAM МБ, % CPU, GDI/USER дескрипторы, uptime) и помодульная таблица нагрузки (события, перерисовки, ошибки, среднее/максимальное время кадра);
  - экспорт полного отчёта в буфер обмена («Копировать отчёт») и в файл `data/logs/diag-report.txt` («Сохранить отчёт»).
- **Онлайн-монитор (`mod-monitor`)**: зарезервирован для расширенного мониторинга дочерних процессов-модулей (`--module`).

