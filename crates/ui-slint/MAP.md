# MAP: crates/ui-slint

Полный декларативный UI-слой `asrbox` на Slint (`~1.18`, `winit` + `renderer-software`) с безрамочными стеклянными окнами (`no-frame: true`), Windows 11 Mica/Acrylic (`window-vibrancy`) и нативными `PopupWindow` ПКМ-меню.

## Файлы Rust

| Файл | Роль |
|---|---|
| `build.rs` | Компиляция `ui/app.slint` со стилем `fluent` через `slint-build` |
| `src/lib.rs` | Экспорт `run_settings_window`, `run_notes_window`, `run_widgets_host`, `run_dock_host`, `run_tray_menu_host`, `selftest_slint_ipc` |
| `src/generated.rs` | Изолированный модуль для макроса `slint::include_modules!()` |
| `src/vibrancy.rs` | Выбор `winit`-бэкенда, Mica/Acrylic, skip-taskbar, слои, bounds/drop/focus-lost хуки |
| `src/win_os.rs` | Кроссплатформенный фасад над `platform-windows` (WinHTTP, CredMgr, shell, running, icons) и `WorkerMsg` |
| `src/ipc_proto.rs` | Протоколы `UI_CMD`, `STATUS` и `SHOW` (`TrayShowPayload`) + unit-тесты |
| `src/settings_state.rs` | Загрузка темы, поиска, аккордеонов, общих настроек, бэкапа и синхронизации |
| `src/settings_state_tabs.rs` | Загрузка состояния вкладок Док-панели, Виджеты и Игровой оверлей |
| `src/settings_bridge.rs` | Контроллер `SettingsWindow`, аккордеоны `settings_ui.cfg`, IPC-поток stdin |
| `src/notes_bridge.rs` | Контроллер 3-колоночного `NotesWindow`, `Ctrl+1/2/3`, CRUD, автосохранение 450 мс |
| `src/notes_view.rs` | Построение Slint-моделей дерева папок, карточек заметок и чек-листов |
| `src/widgets_runner.rs` | Хост `--worker widgets`: жизненный цикл Погоды, Монитора и Календаря + `SupervisorCmd` |
| `src/widgets_weather.rs` | Контроллер `WeatherWindow`, таймер, bounds, ПКМ-действия (`weather.cfg`) |
| `src/widgets_weather_data.rs` | Блокирующий fetch (Open-Meteo + гибрид Яндекс `3–7д`) и синхронизация модели Погоды |
| `src/widgets_sysmon.rs` | Контроллер `SysMonitorWindow`, фоновый `SysTelemetryCollector` с паузой при скрытии |
| `src/widgets_sysmon_view.rs` | Форматирование телеметрии CPU/GPU/ОЗУ/Дисков/Сети/Топ-5 для `SysMonitorWindow` |
| `src/widgets_calendar.rs` | Контроллер `CalendarWindow`, кэш + WinHTTP `.ics`, фильтры периодов и даты |
| `src/dock_runner.rs` | Хост `--worker dock`: мультидок, свободный drag без автопримагничивания, drop файлов, running poll |
| `src/dock_view.rs` | Кэш реальных Windows-иконок RGBA -> `slint::Image`, расчёт геометрии дока, `apply_dock_mutation` |
| `src/tray_runner.rs` | Предпрогретый хост `--ui-tray` для `TrayMenuWindow` с привязкой к work-area и автоскрытием |

## Файлы Slint (`ui/`)

- `ui/app.slint`, `ui/theme.slint` — токены Concentric Fluent Glass (Dark/Light, 5 акцентов) и модели.
- `ui/components/{asr_logo,window_chrome,controls}.slint` — кастомные контролы.
- `ui/settings*.slint` — окно Настроек (4 раздела).
- `ui/notes.slint` — 3-колоночное окно Заметок и задач.
- `ui/widget_{weather,sysmon,calendar}.slint` — 3 десктопных виджета с `PopupWindow` ПКМ-меню.
- `ui/dock.slint` — горизонтальная/вертикальная стеклянная док-панель с `PopupWindow` ПКМ-меню.
- `ui/tray_menu.slint` — всплывающее меню системного трея с быстрым верхним рядом и секциями.
