# MAP: `crates/mod-backup`

Модуль портативного резервного копирования (`BK-001..BK-006`): архивация настроек, заметок, программы целиком и исходного кода GitHub в стандартный `.zip` (STORE или WinZip AE-2 AES-256 на чистом `std`, без внешних крейтов и без `unsafe`). Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/crypto.rs` | Чистый Rust SHA-1, HMAC-SHA1, PBKDF2-HMAC-SHA1 и AES-256-CTR (WinZip AE-2) с тест-векторами NIST/RFC. |
| `src/github.rs` | Конфиг `github_backup.cfg` и строгая валидация `owner/repo` и ветки для скачивания исходников GitHub. |
| `src/zip.rs` | Генератор `.zip`-архивов (STORE + IEEE CRC-32 и WinZip AE-2 AES-256 с extra-полем `0x9901`). |
| `src/lib.rs` | Конфиг `backup.cfg`, сборка файлов (`data/config`, `data/notes`, `.exe`), создание и поиск бэкапов в `data/backups/`. |
