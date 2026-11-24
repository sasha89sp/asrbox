# MAP: `crates/mod-webdav`

Модуль WebDAV (`WD-001..WD-006`): мультисерверное хранилище WebDAV (`Яндекс.Диск`, `Облако Mail.ru`, `Свой WebDAV`), автоматическая миграция v1 -> v2, парсер `PROPFIND Depth: 1` и настройки периодической синхронизации произвольных файлов и папок Windows. Пароли — только в Windows Credential Manager. Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | `WebDavProvider`, `WebDavServer`, `WebDavStore`, миграция v1, `webdav.cfg`, `validate_https_url`, `normalize_remote_dir`, `base64_encode`. |
| `src/propfind.rs` | Чистый Rust-парсер WebDAV `PROPFIND` XML (`parse_propfind_xml`), декодирование UTF-8 `%XX`, `parent_remote_dir`. |
| `src/sync.rs` | `SyncInterval`, `SyncItem`, `FileSyncConfig` и сохранение `data/config/sync.cfg` для синхронизации файлов и папок. |
