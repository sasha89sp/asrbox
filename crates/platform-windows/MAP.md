# MAP.md — `crates/platform-windows`

Единственный крейт workspace с `unsafe` (каждый блок снабжён комментарием `// SAFETY:`). Реализует контракты `crates/platform-api` через `windows` (`0.61`).

## Файлы
- `src/lib.rs` — экспорт `WindowsDockHost`, `WindowsIconProvider`, `SingleInstanceGuard`, `present_layered_bgra`, `query_primary_work_area`.
- `src/window.rs` — `WindowsDockHost`, регистрация классов окон, создание/удаление док-окон, `WM_DISPLAYCHANGE`, `WM_HTTP_DONE`, `WM_WAKE_EVENTS` и цикл сообщений.
- `src/dock_proc.rs` — оконная процедура док-панели (`dock_wnd_proc`), события мыши, таймер анимации и `DragAcceptFiles`/`WM_DROPFILES`.
- `src/widget_win.rs` — реализация `WidgetPlatform` для `WindowsDockHost` (окна виджетов, кадры, таймеры, tooltip, HTTP, URL и открытие `.cfg` в Notepad).
- `src/widget_proc.rs` — оконная процедура виджета (`widget_wnd_proc`), перетаскивание за шапку, слои (`Wallpaper`/`Normal`/`Topmost`) и ПКМ-меню с пресетами прозрачности.
- `src/widget_text.rs` — рендер кириллического текста `Segoe UI` (`CreateFontIndirectW` + `DrawTextW`) с альфа-композитингом на BGRA-кадр, рендер tooltip и `GetLocalTime`/`GetTimeZoneInformation`.
- `src/http.rs` — фоновый HTTPS/HTTP GET-клиент на WinHTTP (`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` + таймауты) в отдельном потоке с уведомлением UI через `WM_HTTP_DONE`.
- `src/icons.rs` — извлечение качественных иконок (`IShellItemImageFactory` до 256×256, WIC для `.png`/`.ico`, `ExtractIconExW` для `.exe`/`.dll`).
- `src/tray.rs` — иконка в трее (`Shell_NotifyIconW`) и нативные ПКМ-меню док-панели и трея (разделители, настройки, Диагностика, О программе).
- `src/shell.rs` — диалог выбора иконки (`GetOpenFileNameW`) и запуск программ, URL, `notepad.exe` и `explorer.exe` (`ShellExecuteW`).
- `src/metrics.rs` — `ProcessMetricsSampler` (Working Set RAM, % CPU процесса, счётчики GDI/USER дескрипторов и uptime).
- `src/tools_win.rs` — состояние шрифтов `ToolsWinState`, буфер обмена и `impl ToolsPlatform for WindowsDockHost`.
- `src/tools_proc.rs` — нативные окна Win32 «Диагностика и логи» (`AsrboxDebugWindow`) и «О программе» (`AsrboxAboutWindow`), процедуры и раскладка контролов.
