# MAP: crates/ui-slint

Полный декларативный UI-слой `asrbox` на Slint (`~1.18`, `winit` + `renderer-software`) с безрамочными стеклянными окнами (`no-frame: true`), нативными скруглениями Windows 11 DWM (`DWMWCP_ROUND`), `LWA_ALPHA` прозрачностью, 8-сторонним ресайзом, векторными иконками и нативными `PopupWindow` меню.

## Файлы Rust

| Файл | Роль |
|---|---|
| `build.rs` | Компиляция `ui/app.slint` со стилем `fluent` через `slint-build` |
| `src/lib.rs` | Экспорт `run_settings_window`, `run_notes_window`, `run_widgets_host`, `run_dock_host`, `run_tray_menu_host`, `selftest_slint_ipc` |
| `src/generated.rs` | Изолированный модуль для макроса `slint::include_modules!()` |
| `src/vibrancy.rs` | Выбор `winit`-бэкенда, DWM-скругления, `LWA_ALPHA`, `WS_EX_TOOLWINDOW`/`ITaskbarList` скрытие из панели задач, `winit_scale_factor`, `focus_window`, 8-сторонний ресайз и хуки |
| `src/win_os.rs` | Кроссплатформенный фасад над `platform-windows` (WinHTTP, CredMgr, shell, running, icons, нативные Fluent-попапы) и `WorkerMsg` |
| `src/ipc_proto.rs` | Протоколы `UI_CMD`, `STATUS` (вкл. живые логи диагностики) и `SHOW` (`TrayShowPayload`) + unit-тесты |
| `src/settings_state.rs` | Загрузка темы, поиска, аккордеонов, общих настроек, бэкапа, синхронизации, живой диагностики и `CHANGELOG.md` |
| `src/settings_state_tabs.rs` | Загрузка состояния вкладок Док-панели (вкл. `icon_frame_px`), Виджеты и Игровой оверлей |
| `src/settings_bridge.rs` | Контроллер `SettingsWindow` (вкладки `0..=5`, сброс скролла, таймер живой диагностики 1.5 с, `focus_window`), 8-сторонний ресайз, IPC stdin |
| `src/notes_bridge.rs` | Контроллер 3-колоночного `NotesWindow`, `Ctrl+1/2/3`, сохранение позиции/размера, нативное ПКМ-меню, автосохранение 450 мс |
| `src/notes_view.rs` | Построение Slint-моделей папок/заметок/задач и нативного ПКМ-меню Заметок (`build_notes_context_menu`) |
| `src/widgets_runner.rs` | Хост `--worker widgets`: жизненный цикл Погоды, Монитора и Календаря + `SupervisorCmd` |
| `src/widgets_menu.rs` | Общий построитель и декодер нативного Win32 Fluent ПКМ-меню для Погоды, Системного монитора и Календаря |
| `src/widgets_weather.rs` | Контроллер `WeatherWindow`, таймер, защита стартовых bounds, 8-сторонний ресайз, нативное Fluent ПКМ-меню (`weather.cfg`) |
| `src/widgets_weather_data.rs` | Блокирующий fetch (Open-Meteo + гибрид Яндекс `3–7д`) и синхронизация модели Погоды |
| `src/widgets_sysmon.rs` | Контроллер `SysMonitorWindow`, фоновый `SysTelemetryCollector`, асинхронный запуск утилит (`taskmgr`/`ms-settings:display`/`perfmon`/`ncpa.cpl`), нативное Fluent ПКМ-меню |
| `src/widgets_sysmon_view.rs` | Форматирование телеметрии CPU/GPU/ОЗУ/Дисков/Сети/Топ-5 для `SysMonitorWindow` |
| `src/widgets_calendar.rs` | Контроллер `CalendarWindow`, кэш + WinHTTP `.ics`, фильтры периодов, нативное Fluent ПКМ-меню, 8-сторонний ресайз |
| `src/dock_icons.rs` | Кэш 128px Windows-иконок RGBA -> `slint::Image` и резолв относительных путей |
| `src/dock_runner.rs` | Хост `--worker dock`: мультидок, DPI-позиционирование, двойная фиксация `enforce_skip_taskbar` (60/250 мс), drag/drop, нативные ПКМ-меню и веерный попап папок |
| `src/dock_view.rs` | DPI-aware расчёт геометрии дока с учётом `icon_frame_px`, синхронизация модели, `build_dock_menu_state` и `apply_native_dock_action` |
| `src/tray_runner.rs` | Фолбэк-хост `--ui-tray` для `TrayMenuWindow` с привязкой к work-area и автоскрытием |

## Файлы Slint (`ui/`)

- `ui/app.slint`, `ui/theme.slint` — токены Concentric Fluent Glass (Dark/Light, 5 акцентов) и модели.
- `ui/components/{asr_logo,window_chrome,controls}.slint` — векторные иконки (`VectorIcon*`), полноширинная перетаскиваемая шапка с красной `✕`, `WindowResizeHandles`, `GlassSlider` и `AccordionCard` (с `accent-top`).
- `ui/settings*.slint` (`settings`, `settings_general`, `settings_dock`, `settings_widgets`, `settings_overlay`, `settings_tools`) — окно Настроек (6 разделов вкл. «Диагностика и логи» и «История версий»).
- `ui/notes.slint` — 3-колоночное окно Заметок и задач с нативным ПКМ-меню.
- `ui/widget_{weather,sysmon,calendar}.slint` — 3 десктопных виджета с 8-сторонним ресайзом и вызовом нативного Win32 Fluent ПКМ-меню по отпусканию ПКМ.
- `ui/dock.slint` — стеклянная док-панель с постоянными настраиваемыми рамками плиток (`icon-frame-px`), стеклянным hover-тултипом и вызовами нативного ПКМ-меню.
- `ui/tray_menu.slint` — фолбэк-меню системного трея с векторными кнопками быстрого ряда.
