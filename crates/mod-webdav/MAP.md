# MAP: `crates/mod-webdav`

Модуль WebDAV (`WD-001..WD-004`): пресеты Яндекс.Диск и Облако Mail.ru (+ свой HTTPS WebDAV), хранение несекретных настроек в `data/config/webdav.cfg` (пароли приложения — только в Windows Credential Manager), валидация HTTPS и Base64 Basic Auth. Без `unsafe`, без внешних крейтов. Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | `WebDavProvider` (Яндекс.Диск / Mail.ru / Свой), `WebDavConfig`, `webdav.cfg`, `validate_https_url`, `base64_encode`. |
