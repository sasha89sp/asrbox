# MAP: `crates/platform-windows`

Реализация трейтов `crates/platform-api` через `windows-rs` (Windows 10/11 x64): хуки, док-окна, захват экрана, метрики процессов, Named Pipes, Credential Manager. Единственный крейт с `unsafe` (`// SAFETY`). Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | Корневой модуль крейта `asrbox-platform-windows` (каркас v0.0.1). |
