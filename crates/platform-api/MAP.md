# MAP: `crates/platform-api`

Платформонезависимые трейты ОС (`KeyboardHook`, `DockWindow`, `ScreenCapture`, `ProcessMonitor`, транспорт IPC). Без `unsafe` и WinAPI-зависимостей. Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | Корневой модуль крейта `asrbox-platform-api` (каркас v0.0.1). |
