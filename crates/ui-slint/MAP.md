# MAP: crates/ui-slint

Полный декларативный UI-слой `asrbox` на Slint (`~1.18`, `winit` + `renderer-software`) с безрамочными стеклянными окнами (`no-frame: true`), нативными скруглениями Windows 11 DWM (`DWMWCP_ROUND`), `LWA_ALPHA` прозрачностью, 8-сторонним ресайзом, векторными иконками и нативными `PopupWindow` меню.

## Файлы Rust

| Файл | Роль |
|---|---|
| `build.rs` | Компиляция `ui/app.slint` со стилем `fluent` через `slint-build` |
| `src/lib.rs` | Экспорт `run_settings_window`, `run_notes_window`, `run_widgets_host`, `run_dock_host`, `run_tray_menu_host`, `selftest_slint_ipc` |
| `src/generated.rs` | Изолированный модуль для макроса `slint::include_modules!()` |
| `src/vibrancy.rs` | Выбор `winit`-бэкенда, DWM-скругления, `LWA_ALPHA`, `WS_EX_TOOLWINDOW` скрытие из панели задач, иконка Настроек, 8-сторонний ресайз, `restore_or_center_window` и хуки |
| `src/win_os.rs` | Кроссплатформенный фасад над `platform-windows` (WinHTTP, CredMgr, shell, running, icons, нативные Fluent-попапы) и `WorkerMsg` |
| `src/ipc_proto.rs` | Протоколы `UI_CMD`, `STATUS` и `SHOW` (`TrayShowPayload`) + unit-тесты |
| `src/settings_state.rs` | Загрузка темы, поиска, аккордеонов, общих настроек, бэкапа и синхронизации |
| `src/settings_state_tabs.rs` | Загрузка состояния вкладок Док-панели (вкл. `icon_frame_px`), Виджеты и Игровой оверлей |
| `src/settings_bridge.rs` | Контроллер `SettingsWindow`, иконка приложения в панели задач, аккордеоны `settings_ui.cfg`, 8-сторонний ресайз, IPC stdin |
| `src/notes_bridge.rs` | Контроллер 3-колоночного `NotesWindow`, `Ctrl+1/2/3`, сохранение позиции/размера, нативное ПКМ-меню, автосохранение 450 мс |
| `src/notes_view.rs` | Построение Slint-моделей папок/заметок/задач и нативного ПКМ-меню Заметок (`build_notes_context_menu`) |
| `src/widgets_runner.rs` | Хост `--worker widgets`: жизненный цикл Погоды, Монитора и Календаря + `SupervisorCmd` |
| `src/widgets_weather.rs` | Контроллер `WeatherWindow`, таймер, защита стартовых bounds, 8-сторонний ресайз, ПКМ-действия (`weather.cfg`) |
| `src/widgets_weather_data.rs` | Блокирующий fetch (Open-Meteo + гибрид Яндекс `3–7д`) и синхронизация модели Погоды |
| `src/widgets_sysmon.rs` | Контроллер `SysMonitorWindow`, фоновый `SysTelemetryCollector`, асинхронный запуск утилит (`taskmgr`/`ms-settings:display`/`perfmon`/`ncpa.cpl`), защита стартовых bounds |
| `src/widgets_sysmon_view.rs` | Форматирование телеметрии CPU/GPU/ОЗУ/Дисков/Сети/Топ-5 для `SysMonitorWindow` |
| `src/widgets_calendar.rs` | Контроллер `CalendarWindow`, кэш + WinHTTP `.ics`, фильтры периодов, защита стартовых bounds, 8-сторонний ресайз |
| `src/dock_icons.rs` | Кэш 128px Windows-иконок RGBA -> `slint::Image` и резолв относительных путей |
| `src/dock_runner.rs` | Хост `--worker dock`: мультидок, свободный drag с защитой стартовой позиции, drop файлов, нативные ПКМ-меню и веерный попап папок |
| `src/dock_view.rs` | Расчёт геометрии дока с учётом `icon_frame_px`, синхронизация модели, `build_dock_menu_state` и `apply_native_dock_action` |
| `src/tray_runner.rs` | Фолбэк-хост `--ui-tray` для `TrayMenuWindow` с привязкой к work-area и автоскрытием |

## Файлы Slint (`ui/`)

- `ui/app.slint`, `ui/theme.slint` — токены Concentric Fluent Glass (Dark/Light, 5 акцентов) и модели.
- `ui/components/{asr_logo,window_chrome,controls}.slint` — векторные иконки (`VectorIcon*`), полноширинная перетаскиваемая шапка с красной `✕`, `WindowResizeHandles` и контролы.
- `ui/settings*.slint` — окно Настроек (4 раздела, компактные подвкладки виджетов `34 px`, ползунок рамки иконок дока).
- `ui/notes.slint` — 3-колоночное окно Заметок и задач с обработкой ПКМ.
- `ui/widget_{weather,sysmon,calendar}.slint` — 3 десктопных виджета с 8-сторонним ресайзом и `PopupWindow` ПКМ-меню.
- `ui/dock.slint` — стеклянная док-панель с постоянными настраиваемыми рамками плиток (`icon-frame-px`), стеклянным hover-тултипом и вызовами нативного ПКМ-меню.
- `ui/tray_menu.slint` — фолбэк-меню системного трея с векторными кнопками быстрого ряда.
