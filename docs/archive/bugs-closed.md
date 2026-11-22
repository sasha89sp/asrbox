# Архив закрытых багов `asrbox`

Сюда переносятся закрытые и подтверждённые в CI дефекты из корневого `BUGS.md`, чтобы основной журнал оставался компактным.

## Закрытые записи

### BUG-001: Падение шага `cargo fmt --check` в GitHub Actions (`windows-latest`)
- **Статус**: `closed` (подтверждено зелёной сборкой CI `windows-latest`)
- **Версия**: `0.0.1`
- **Крейты**: `xtask`, `crates/mod-dock`, каркасные `crates/*/src/lib.rs`
- **Причина**: превышение `fn_call_width = 60` в `xtask/src/main.rs`, 9 пустых 0-байтовых `src/lib.rs` без финального `\n`, однострочный struct-вариант `DockEvent::AnimationTick` в `crates/mod-dock/src/state.rs`.
- **Исправление**: приведены к стандарту `rustfmt`, добавлены однострочные `//!`-комментарии с `\n`, `DockEvent::AnimationTick` развёрнут в 3 строки.
