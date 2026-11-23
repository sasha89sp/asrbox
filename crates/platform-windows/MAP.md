# MAP.md — `crates/platform-windows`

Единственный крейт workspace с `unsafe` (каждый блок снабжён комментарием `// SAFETY:`). Реализует контракты `crates/platform-api` через `windows` (`0.61`).

## Файлы
- `src/lib.rs` — экспорт `WindowsDockHost`, `WindowsIconProvider`, `SingleInstanceGuard`, `present_layered_bgra`, `query_primary_work_area`.
- `src/window.rs` — `WindowsDockHost`, регистрация классов окон, создание/удаление док-окон, `WM_DISPLAYCHANGE`, `WM_HTTP_DONE` и цикл сообщений.
- `src/dock_proc.rs` — оконная процедура док-панели (`dock_wnd_proc`), события мыши, таймер анимации и `DragAcceptFiles`/`WM_DROPFILES`.
- `src/widget_win.rs` — реализация `WidgetPlatform` для `WindowsDockHost` (создание/закрытие окон виджетов, вывод кадров, таймеры, tooltip, HTTP, URL).
- `src/widget_proc.rs` — оконная процедура виджета (`widget_wnd_proc`), перетаскивание за шапку, слои (`Wallpaper`/`Normal`/`Topmost`), DWM Acrylic и ПКМ-меню виджета.
- `src/widget_text.rs` — рендер кириллического текста `Segoe UI` (`CreateFontIndirectW` + `DrawTextW`) с альфа-композитингом на BGRA-кадр, рендер tooltip и `GetLocalTime`/`GetTimeZoneInformation`.
- `src/http.rs` — фоновый HTTPS/HTTP GET-клиент на WinHTTP в отдельном потоке с уведомлением UI через `WM_HTTP_DONE`.
- `src/icons.rs` — извлечение качественных иконок (`IShellItemImageFactory` до 256×256, WIC для `.png`/`.ico`, `ExtractIconExW` для `.exe`/`.dll`).
- `src/tray.rs` — иконка в трее (`Shell_NotifyIconW`), нативные ПКМ-меню док-панели и трея (включая виджет Календарь), диалог выбора иконки, запуск программ и URL (`ShellExecuteW`).
