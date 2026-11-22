# MAP.md — `crates/platform-windows`

Единственный крейт workspace с `unsafe` (каждый блок снабжён комментарием `// SAFETY:`). Реализует контракты `crates/platform-api` через `windows` (`0.61`).

## Файлы
- `src/lib.rs` — экспорт `WindowsDockHost`, `WindowsIconProvider`, `SingleInstanceGuard`.
- `src/window.rs` — прозрачные layered-окна док-панелей (`UpdateLayeredWindow`), `DragAcceptFiles`, таймеры анимации, `WM_DISPLAYCHANGE`, `SingleInstanceGuard` и цикл сообщений.
- `src/icons.rs` — извлечение качественных иконок (`IShellItemImageFactory` до 256×256, WIC для `.png`/`.ico`, `ExtractIconExW` для `.exe`/`.dll`).
- `src/tray.rs` — иконка в системном трее (`Shell_NotifyIconW`), нативные ПКМ-меню док-панели и трея, диалог выбора иконки (`GetOpenFileNameW`), запуск целей (`ShellExecuteW`).
