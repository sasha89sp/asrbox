# MAP: `crates/platform-windows`

Единственный крейт workspace, в котором разрешён `unsafe` и прямые вызовы Windows 10/11 (x64) API. Реализует трейты из `crates/platform-api` (трей, перехват `WM_CLOSE`, хуки клавиатуры, захват экрана, чтение метрик CPU/RAM, работа с устройствами).

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | Корневой модуль крейта `asrbox-platform-windows` (каркас v0.0.1). |
