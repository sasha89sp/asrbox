# MAP: `crates/core`

Контракт модулей (`Module`, `ModuleId`, `ModuleHealth`), настройки изоляции процессов и вкл/выкл модулей (`modules.cfg`), а также текстовый протокол IPC между супервизором и дочерними процессами модулей. Без `unsafe`. Лимит: <= 100 строк.

## Файлы

| Путь | Назначение |
| --- | --- |
| `src/lib.rs` | Трейт `Module`, `ModuleHealth` и публичные реэкспорты `asrbox-core`. |
| `src/modules_cfg.rs` | `ModuleId`, `IsolationMode`, `ModulesConfig` и чтение/запись `data/config/modules.cfg`. |
| `src/ipc.rs` | Кодирование/декодирование команд супервизора (`SupervisorCmd`) и сообщений воркеров (`WorkerMsg`). |
