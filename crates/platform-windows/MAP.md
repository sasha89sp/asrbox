# MAP.md — `crates/platform-windows`

Единственный крейт workspace с `unsafe` (каждый блок снабжён комментарием `// SAFETY:`). Реализует контракты `crates/platform-api` через `windows` (`0.61`).

## Файлы
- `src/lib.rs` — экспорт `WindowsDockHost`, `WindowsIconProvider`, `SingleInstanceGuard`, `JobObject`, `is_process_alive`, `wake_host_window`, `present_layered_bgra`, `query_primary_work_area`.
- `src/window.rs` — `WindowsDockHost` (режимы master с треем и `new_worker()` без трея), регистрация классов окон, таймеры (`ANIMATION`, `RUNNING_POLL`, `SUPERVISOR_TICK`), `WM_IPC_WAKE` и цикл сообщений.
- `src/dock_win.rs` — реализация `DockPlatform` для `WindowsDockHost` (окна доков, рендер, меню, запуск/фокус).
- `src/dock_proc.rs` — оконная процедура док-панели (`dock_wnd_proc`), захват ЛКМ (`SetCapture`) для перетаскивания значков/панели, ПКМ, анимация и `WM_DROPFILES`.
- `src/dock_menu.rs` — ПКМ-меню док-панели с вложенными подменю положения, прозрачности и поведения.
- `src/running.rs` — определение запущенных приложений (`EnumWindows` + `QueryFullProcessImageNameW`), резолв `.lnk` (`IShellLinkW`) и активация открытого окна по клику.
- `src/job.rs` — Win32 `JobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) для изоляции дочерних процессов модулей и `is_process_alive(pid)`.
- `src/settings_proc.rs` — изменяемое окно настроек (`AsrboxSettingsWindow`, `WS_OVERLAPPEDWINDOW`, min 680×480), двойная буферизация DIB и встроенный `EDIT` для URL календаря.
- `src/widget_win.rs` — реализация `WidgetPlatform` для `WindowsDockHost`.
- `src/widget_proc.rs` — оконная процедура виджета, перетаскивание, слои и ПКМ-меню с пунктом «Настройки календаря...».
- `src/widget_text.rs` — рендер кириллического текста `Segoe UI` (`DrawTextW`) на BGRA-кадр, tooltip и локальное время.
- `src/http.rs` — фоновый WinHTTP GET-клиент с `WM_HTTP_DONE`.
- `src/icons.rs` — извлечение иконок (`IShellItemImageFactory`, WIC, `ExtractIconExW`).
- `src/tray.rs` — иконка в трее и сгруппированное по модулям ПКМ-подменю (`MF_POPUP`) трея.
- `src/shell.rs` — диалог выбора иконки и запуск программ, URL, `notepad.exe`, `explorer.exe`.
- `src/metrics.rs` — `ProcessMetricsSampler` (RAM, % CPU, GDI/USER, uptime).
- `src/tools_win.rs` — шрифты `ToolsWinState`, буфер обмена и `impl ToolsPlatform for WindowsDockHost`.
- `src/tools_proc.rs` — нативные окна «Диагностика и логи» и «О программе».
