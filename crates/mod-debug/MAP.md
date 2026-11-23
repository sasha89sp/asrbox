# MAP.md — Карта крейта `crates/mod-debug` (уровень L1)

Пакет: `asrbox-mod-debug`. Спецификация: `specs/mod-debug.md` (`DB-001..DB-007`).

## Модули

| Файл | Ответственность |
| --- | --- |
| `src/lib.rs` | `LogLevel`, `LogEntry`, `ModuleStats`, `DebugHub` — живые логи, переключатели `TRACE`/файл, фильтр модулей и счётчики нагрузки. |
| `src/report.rs` | Маскировка секретов (`redact_secrets`), рендер живой сводки RAM/CPU/GDI и экспорт полного диагностического отчёта. |
