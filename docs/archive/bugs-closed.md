# Архив закрытых багов `asrbox`

Сюда переносятся закрытые и подтверждённые в CI дефекты из корневого `BUGS.md`, чтобы основной журнал оставался компактным.

## Закрытые записи

### BUG-001: Падение шага `cargo fmt --check` в GitHub Actions (`windows-latest`)
- **Статус**: `closed` (подтверждено зелёной сборкой CI `windows-latest`)
- **Версия**: `0.0.1`
- **Крейты**: `xtask`, `crates/mod-dock`, каркасные `crates/*/src/lib.rs`
- **Причина**: превышение `fn_call_width = 60` в `xtask/src/main.rs`, 9 пустых 0-байтовых `src/lib.rs` без финального `\n`, однострочный struct-вариант `DockEvent::AnimationTick` в `crates/mod-dock/src/state.rs`.
- **Исправление**: приведены к стандарту `rustfmt`, добавлены однострочные `//!`-комментарии с `\n`, `DockEvent::AnimationTick` развёрнут в 3 строки.

### BUG-002: Падение CI на шаге `cargo fmt --check` и импортах `windows 0.61`
- **Статус**: `closed` (подтверждено зелёными шагами `Check formatting`, `Run Clippy`, `Verify ATLAS and MAP coverage`, `Run tests`, `Build release binary`, `< 25 MB` в CI run `45127781497`)
- **Версия**: `0.0.1`
- **Крейты**: `crates/mod-widgets`, `crates/mod-dock`, `crates/platform-api`, `crates/platform-windows`, `crates/app`
- **Причина**: расхождения форматирования Edition 2021, неверные фичи/пути `windows 0.61` (`Win32_Graphics_Dwm`, `Win32_Security`, `ClientToScreen`, `WM_MOUSELEAVE`) и два предупреждения Clippy 1.100 (`icons.rs`, тест `storage.rs`).
- **Исправление**: выполнен `cargo fmt`, исправлены фичи/импорты и предупреждения Clippy.
