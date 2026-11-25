# MAP.md — `crates/platform-windows`

Единственный крейт workspace с `unsafe` (каждый блок снабжён комментарием `// SAFETY:`). Реализует контракты `crates/platform-api` через `windows` (`0.61`).

## Файлы
- `src/lib.rs` — экспорт `WindowsDockHost`, `WindowsIconProvider`, `SingleInstanceGuard`, `JobObject`, `is_process_alive`, `wake_host_window`, `present_layered_bgra`, `query_primary_work_area`.
- `src/app_icon.rs` — DPI-aware загрузка встроенной многоразмерной иконки PE (`LoadImageW` + `RT_GROUP_ICON #1`) с bottom-up фолбэком `CreateIconIndirect`, кэшем и `WM_SETICON`.
- `src/autostart.rs` — автозагрузка Windows (`HKCU\...\Run\asrbox`) и Windows Credential Manager (`CredWriteW`/`CredReadW`/`CredDeleteW`).
- `src/fluent_menu.rs` — точка входа и цикл кастомного слоистого Fluent-меню (`show_fluent_popup_menu`).
- `src/fluent_menu_win.rs` — Win32-окно `AsrboxFluentMenuWindow`, презентация, вложенные подменю, hit-test и обработка мыши/клавиатуры.
- `src/fluent_menu_layout.rs` — раскладка геометрии Fluent-меню, быстрый верхний ряд и привязка к границам экрана.
- `src/fluent_menu_render.rs` — premultiplied BGRA-рендер Fluent-меню (скругления, тень, акценты, иконки, хоткеи, разделители).
- `src/webdav_wire.rs` — низкоуровневый WinHTTP HTTPS транспорт, парсер URL, Base64 и `ensure_remote_dirs`.
- `src/webdav_http.rs` — фоновые операции WebDAV (`PROPFIND` тест, `PUT` загрузка архива) и очередь `WM_WEBDAV_DONE`.
- `src/webdav_sync.rs` — фоновые WebDAV-операции: обзор папок (`PROPFIND Depth: 1`), создание папок (`MKCOL`) и синхронизация файлов/папок Windows.
- `src/webdav_xml.rs` — парсер WebDAV `PROPFIND` XML (`parse_propfind_xml`), `normalize_dir` и `%XX`-декодирование.
- `src/window.rs` — `WindowsDockHost` (master с треем и `new_worker()`), хранение `FluentPalette`, регистрация классов, таймеры, `WM_IPC_WAKE`, `WM_WEBDAV_DONE` и цикл сообщений.
- `src/dock_win.rs` — реализация `DockPlatform` для `WindowsDockHost` (окна доков, слои, рендер, Fluent-меню, тема, запуск/фокус).
- `src/dock_proc.rs` — оконная процедура док-панели (`dock_wnd_proc`), удержание слоя `Wallpaper`, захват ЛКМ, ПКМ, анимация и `WM_DROPFILES`.
- `src/dock_menu.rs` — Fluent ПКМ-меню док-панели (открытие расположения файла/папки ярлыка, быстрые действия + подменю положения, слоя, имён, прозрачности и поведения).
- `src/running.rs` — определение запущенных приложений (`EnumWindows` + `QueryFullProcessImageNameW`), резолв `.lnk` (`IShellLinkW`) и усиленная активация открытого/свёрнутого окна (`AllowSetForegroundWindow`/`SW_RESTORE`/`AttachThreadInput`).
- `src/job.rs` — Win32 `JobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`) для изоляции дочерних процессов модулей без закрытия запущенных из дока программ.
- `src/settings_proc.rs` — изменяемое Fluent-окно настроек (`AsrboxSettingsWindow`, защита `try_borrow`, события мыши/клавиатуры/колеса, 5 слотов `EDIT` с `EN_CHANGE`, таймер анимации темы).
- `src/settings_helpers.rs` — вспомогательные функции окна настроек: `DWMWA_USE_IMMERSIVE_DARK_MODE`, позиционирование/стилизация полей `EDIT` (`Segoe UI Variable`) и GDI-вывод кадра.
- `src/widget_win.rs` — реализация `WidgetPlatform` для `WindowsDockHost` (окна виджетов, ресайз, клик-сквозь, видимость, таймеры, тема, Fluent-меню и тултипы).
- `src/widget_proc.rs` — оконная процедура виджета: клик-сквозь (`HTTRANSPARENT`/`MA_NOACTIVATE`), перетаскивание, 8-сторонний ресайз и управление слоем.
- `src/widget_menu.rs` — Fluent ПКМ-меню виджетов (Календарь и Системный монитор с верхним рядом быстрых действий и подменю).
- `src/font_engine.rs` — автоопределение системного шрифта Win11 `Segoe UI Variable` (`Display`/`Text`/`Small` -> фолбэк `Segoe UI`), кэш GDI `HFONT` с `CLEARTYPE_NATURAL_QUALITY` и гамма-блендинг маски текста.
- `src/dwrite_text.rs` — ленивый движок `DirectWrite` (`IDWriteFactory` + `IDWriteGdiInterop`) для плавного сглаживания текста на выбор пользователя.
- `src/dwrite_vtable.rs` — статический COM vtable `IDWriteTextRenderer` без аллокаций на кадр для `dwrite_text`.
- `src/widget_text.rs` — диспетчер рендера текста (`GDI ClearType` / `DirectWrite`) на BGRA-кадр, Fluent tooltip с темой и локальное время.
- `src/overlay_game.rs` — детектор активных игр (D3D fullscreen / borderless покрытие монитора, чёрный список, `extra_games`, rect монитора).
- `src/overlay_telemetry.rs` — фоновый поток телеметрии игрового оверлея (`WM_OVERLAY_DONE`, DWM FPS + 1% low / Гц, GPU/VRAM, CPU, RAM, ICMP-пинг и % потерь).
- `src/sys_telemetry.rs` — фоновый поток сбора телеметрии (`WM_TELEMETRY_DONE`, uptime, план питания, `NtQueryTimerResolution`).
- `src/sys_procs.rs` — телеметрия CPU/частоты (`CallNtPowerInformation`), ОЗУ/кэша/PF, топ-5 процессов и дисков (`IOCTL_DISK_PERFORMANCE`).
- `src/sys_gpu.rs` — телеметрия GPU (DXGI VRAM, частота экрана Гц, динамическая загрузка `nvml.dll`).
- `src/sys_net.rs` — телеметрия сети (IPv4, VPN, трафик `GetIfTable2`, ICMP-пинг шлюза и `1.1.1.1` с джиттером и потерями).
- `src/http.rs` — фоновый WinHTTP GET-клиент с `WM_HTTP_DONE`.
- `src/icons.rs` — извлечение иконок (`IShellItemImageFactory`, WIC, `PrivateExtractIconsW`/`ExtractIconExW`, восстановление альфы legacy `HICON`).
- `src/icons_lnk.rs` — разрешение `.lnk` (`IShellLinkW::GetIconLocation`/`GetPath`, `ExpandEnvironmentStringsW`), `SHGetFileInfoW` и `has_explorer_frame_box`.
- `src/tray.rs` — Fluent-иконка в трее и показ кастомного Fluent-меню.
- `src/tray_menu.rs` — сборка `FluentMenuSpec` системного трея (быстрый ряд + подменю модулей) и декодирование `TrayAction`.
- `src/shell.rs` — диалоги выбора иконки, нескольких файлов (`OFN_ALLOWMULTISELECT`) и папки (`IFileOpenDialog`), `ReleaseCapture`, буфер обмена, открытие расположения файла (`explorer /select`), запуск программ/URL/Блокнота.
- `src/metrics.rs` — `ProcessMetricsSampler` (RAM, % CPU, GDI/USER, uptime).
- `src/tools_win.rs` — `impl ToolsPlatform for WindowsDockHost`.
- `src/tools_state.rs` — состояние `ToolsWinState`, кэш GDI-шрифтов и чтение/запись текста контролов.
- `src/tools_proc.rs` — вспомогательные нативные окна «Диагностика и логи» и «О программе».
