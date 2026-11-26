# MAP: crates/ui-slint

Полный декларативный UI-слой `asrbox` на Slint (`~1.18`, `winit` + `renderer-software`) с безрамочными стеклянными окнами (`no-frame: true`), нативными скруглениями Windows 11 DWM (`DWMWCP_ROUND`), `LWA_ALPHA` прозрачностью, 8-сторонним ресайзом, векторными иконками и нативными `PopupWindow` меню.

## Файлы Rust

| Файл | Роль |
|---|---|
| `build.rs` | Компиляция `ui/app.slint` со стилем `fluent` через `slint-build` |
| `src/lib.rs` | Экспорт `run_settings_window`, `run_notes_window`, `run_widgets_host`, `run_dock_host`, `run_tray_menu_host`, `selftest_slint_ipc` |
| `src/generated.rs` | Изолированный модуль для макроса `slint::include_modules!()` |
| `src/vibrancy.rs` | Выбор `winit`-бэкенда, DWM-скругления, `LWA_ALPHA` прозрачность, 8-сторонний ресайз, `DesktopWindowSpec`, центрирование, перерисовка и хуки |
| `src/win_os.rs` | Кроссплатформенный фасад над `platform-windows` (WinHTTP, CredMgr, shell, Проводник, running, icons) и `WorkerMsg` |
| `src/ipc_proto.rs` | Протоколы `UI_CMD`, `STATUS` и `SHOW` (`TrayShowPayload`) + unit-тесты |
| `src/settings_state.rs` | Загрузка темы, поиска, аккордеонов, общих настроек, бэкапа и синхронизации |
| `src/settings_state_tabs.rs` | Загрузка состояния вкладок Док-панели, Виджеты и Игровой оверлей |
| `src/settings_bridge.rs` | Контроллер `SettingsWindow`, аккордеоны `settings_ui.cfg`, 8-сторонний ресайз, IPC-поток stdin |
| `src/notes_bridge.rs` | Контроллер 3-колоночного `NotesWindow`, `Ctrl+1/2/3`, CRUD, автосохранение 450 мс, 8-сторонний ресайз |
| `src/notes_view.rs` | Построение Slint-моделей дерева папок, карточек заметок и чек-листов |
| `src/widgets_runner.rs` | Хост `--worker widgets`: жизненный цикл Погоды, Монитора и Календаря + `SupervisorCmd` |
| `src/widgets_weather.rs` | Контроллер `WeatherWindow`, таймер, bounds, 8-сторонний ресайз, ПКМ-действия (`weather.cfg`) |
| `src/widgets_weather_data.rs` | Блокирующий fetch (Open-Meteo + гибрид Яндекс `3–7д`) и синхронизация модели Погоды |
| `src/widgets_sysmon.rs` | Контроллер `SysMonitorWindow`, фоновый `SysTelemetryCollector` с паузой при скрытии, 8-сторонний ресайз |
| `src/widgets_sysmon_view.rs` | Форматирование телеметрии CPU/GPU/ОЗУ/Дисков/Сети/Топ-5 для `SysMonitorWindow` |
| `src/widgets_calendar.rs` | Контроллер `CalendarWindow`, кэш + WinHTTP `.ics`, фильтры периодов, даты, 8-сторонний ресайз |
| `src/dock_icons.rs` | Кэш 128px Windows-иконок RGBA -> `slint::Image`, резолв относительных путей, чтение каталогов для веерного попапа папок |
| `src/dock_runner.rs` | Хост `--worker dock`: мультидок, свободный drag, drop файлов, веерный попап папок, running poll |
| `src/dock_view.rs` | Расчёт геометрии дока, синхронизация модели и веерного попапа папок, `apply_dock_mutation` и хелперы конфига |
| `src/tray_runner.rs` | Предпрогретый хост `--ui-tray` для `TrayMenuWindow` с привязкой к work-area, принудительной перерисовкой и автоскрытием |

## Файлы Slint (`ui/`)

- `ui/app.slint`, `ui/theme.slint` — токены Concentric Fluent Glass (Dark/Light, 5 акцентов) и модели (включая `SlintFolderEntryRow`).
- `ui/components/{asr_logo,window_chrome,controls}.slint` — векторные иконки (`VectorIcon*`), кнопки шапки с красной `✕`, 8-сторонние ручки ресайза (`WindowResizeHandles`) и контролы.
- `ui/settings*.slint` — окно Настроек (4 раздела).
- `ui/notes.slint` — 3-колоночное окно Заметок и задач.
- `ui/widget_{weather,sysmon,calendar}.slint` — 3 десктопных виджета с 8-сторонним ресайзом и `PopupWindow` ПКМ-меню.
- `ui/dock.slint` — стеклянная док-панель с веерным `PopupWindow` папок, стеклянным hover-тултипом и ПКМ-меню.
- `ui/tray_menu.slint` — всплывающее меню системного трея с векторными кнопками быстрого ряда и секциями.
