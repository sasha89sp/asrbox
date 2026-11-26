# LICENSES.md — Реестр зависимостей и лицензий `asrbox`

Каждая новая внешняя зависимость добавляется только после записи обоснования в `DECISIONS.md` и фиксации её лицензии в этом документе.

## Текущее состояние (v0.1.0)

Прямые внешние зависимости ограничены платформенным слоем (`windows` в `crates/platform-windows`) и декларативным UI-слоем (`slint`, `slint-build`, `window-vibrancy` в `crates/ui-slint`); остальные крейты бизнес-логики используют только стандартную библиотеку Rust (`std`).

| Крейт | Версия | Лицензия | Где используется | ADR |
| --- | --- | --- | --- | --- |
| `windows` | `0.61` | `MIT OR Apache-2.0` | `crates/platform-windows` (только `cfg(windows)`) | `ADR-008` |
| `slint` | `1.18.1` | `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0` | `crates/ui-slint` (`backend-winit`, `renderer-software`, `unstable-winit-030`) | `ADR-021` |
| `slint-build` | `1.18.1` | `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0` | `crates/ui-slint` (`[build-dependencies]`) | `ADR-021` |
| `window-vibrancy` | `0.8.1` | `MIT OR Apache-2.0` | `crates/ui-slint` (только `cfg(windows)`, Windows 11 Mica) | `ADR-021` |
