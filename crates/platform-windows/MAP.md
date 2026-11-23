# MAP.md — `crates/platform-windows`

Единственный крейт workspace с `unsafe` (каждый блок снабжён комментарием `// SAFETY:`). Реализует контракты `crates/platform-api` через `windows` (`0.61`).

## Файлы
- `src/lib.rs` — экспорт `WindowsDockHost`, `WindowsIconProvider`, `SingleInstanceGuard`, `JobObject`, `is_process_alive`, `wake_host_window`, `present_layered_bgra`, `query_primary_work_area`.
- `src/app_icon.rs` — процедурный 32-bit BGRA `HICON` в стиле NotePlan (16×16 трей и 32×32 панель задач/окна) с кэшированием и `WM_SETICON`.
- `src/autostart.rs` — автозагрузка Windows (`HKCU\...\Run\asrbox`) и Windows Credential Manager (`CredWriteW`/`CredReadW`/`CredDeleteW`).
- `src/webdav_http.rs` — фоновый WinHTTP HTTPS WebDAV-клиент (`PROPFIND` тест, `MKCOL` + потоковый `PUT`) с `WM_WEBDAV_DONE`.
- `src/window.rs` — `WindowsDockHost` (master с треем и `new_worker()`), регистрация классов с NotePlan `HICON`, таймеры, `WM_IPC_WAKE`, `WM_WEBDAV_DONE` и цикл сообщений.
- `src/dock_win.rs` — реализация `DockPlatform` для `WindowsDockHost` (окна доков, слой на рабочем столе/обычный/поверх, рендер, меню, запуск/фокус).
- `src/dock_proc.rs` — оконная процедура док-панели (`dock_wnd_proc`), удержание слоя `Wallpaper` (`WM_WINDOWPOSCHANGING`), захват ЛКМ, ПКМ, анимация и `WM_DROPFILES`.
- `src/dock_menu.rs` — ПКМ-меню док-панели с подменю положения, слоя отображения, прозрачности и поведения.
- `src/running.rs` — определение запущенных приложений (`EnumWindows` + `QueryFullProcessImageNameW`), резолв `.lnk` (`IShellLinkW`) и активация открытого окна по клику.
- `src/job.rs` — Win32 `JobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) для изоляции дочерних процессов модулей и `is_process_alive(pid)`.
- `src/settings_proc.rs` — изменяемое окно настроек (`AsrboxSettingsWindow`, NotePlan Dark, `WM_SETICON`), двойная буферизация DIB и 3 слота `EDIT` (URL/логин, пароль `ES_PASSWORD`, custom URL).
- `src/widget_win.rs` — реализация `WidgetPlatform` для `WindowsDockHost` (окна виджетов, границы ресайза, таймеры, запрос телеметрии).
- `src/widget_proc.rs` — оконная процедура виджета: перетаскивание за заголовок, 8-сторонний интерактивный ресайз с курсорами и управление слоем.
- `src/widget_menu.rs` — ПКМ-меню виджетов (Календарь с подменю синхронизации и Системный монитор с подменю частоты обновления).
- `src/widget_text.rs` — рендер кириллического текста `Segoe UI` (`DrawTextW`) на BGRA-кадр, tooltip и локальное время.
- `src/sys_telemetry.rs` — фоновый поток сбора телеметрии (`WM_TELEMETRY_DONE`, uptime, план питания, разрешение таймера ОС `NtQueryTimerResolution`).
- `src/sys_procs.rs` — реальная телеметрия CPU/частоты (`CallNtPowerInformation`), ОЗУ/кэша/PF, топ-5 процессов CPU/ОЗУ и дисков (`IOCTL_DISK_PERFORMANCE`).
- `src/sys_gpu.rs` — телеметрия GPU (DXGI VRAM, частота экрана Гц, динамическая загрузка `nvml.dll` при наличии NVIDIA).
- `src/sys_net.rs` — телеметрия сети (IPv4, детектор VPN, трафик `GetIfTable2`, ICMP-пинг шлюза и `1.1.1.1` с джиттером и потерями).
- `src/http.rs` — фоновый WinHTTP GET-клиент с `WM_HTTP_DONE`.
- `src/icons.rs` — извлечение иконок (`IShellItemImageFactory`, WIC, `ExtractIconExW`).
- `src/tray.rs` — иконка NotePlan в трее и сгруппированное по модулям ПКМ-подменю (`Док-панели`, `Виджеты`, `Бэкап и Облако`, `Автозагрузка`).
- `src/shell.rs` — диалог выбора иконки и запуск программ, URL, `notepad.exe`, `explorer.exe`.
- `src/metrics.rs` — `ProcessMetricsSampler` (RAM, % CPU, GDI/USER, uptime).
- `src/tools_win.rs` — шрифты `ToolsWinState`, буфер обмена и `impl ToolsPlatform for WindowsDockHost`.
- `src/tools_proc.rs` — окна «Диагностика и логи» и «О программе» в тёмной палитре NotePlan с NotePlan `HICON`.
