# MAP: crates/ui-slint

Декларативный UI-слой `asrbox` на Slint (`~1.18`, `winit` + `renderer-software`) с безрамочными стеклянными окнами (`no-frame: true`) и нативным Windows 11 Mica / Acrylic (`window-vibrancy`).

## Файлы Rust

| Файл | Роль |
|---|---|
| `build.rs` | Компиляция `ui/app.slint` со стилем `fluent` через `slint-build` |
| `src/lib.rs` | Публичные точки входа `run_settings_window`, `run_notes_window`, `selftest_slint_ipc` |
| `src/generated.rs` | Изолированный модуль для макроса `slint::include_modules!()` |
| `src/vibrancy.rs` | Выбор `winit`-бэкенда, `apply_mica` / `apply_acrylic`, drag/min/max окна |
| `src/ipc_proto.rs` | Протокол `UI_CMD` / `STATUS` между процессом-хостом и Slint UI-процессами |
| `src/settings_state.rs` | Загрузка темы, поиска, аккордеонов, общих настроек, бэкапа и синхронизации |
| `src/settings_state_tabs.rs` | Загрузка состояния вкладок Док-панели, Виджеты и Игровой оверлей |
| `src/settings_bridge.rs` | Контроллер `SettingsWindow`, аккордеоны `settings_ui.cfg`, IPC-поток stdin |
| `src/notes_bridge.rs` | Контроллер 3-колоночного `NotesWindow`, `Ctrl+1/2/3`, CRUD, автосохранение 450 мс |
| `src/notes_view.rs` | Построение Slint-моделей дерева папок, карточек заметок и чек-листов |

## Файлы Slint (`ui/`)

- `ui/app.slint`, `ui/theme.slint` — дизайн-токены Concentric Fluent Glass (Dark/Light, 5 акцентов) и структуры.
- `ui/components/asr_logo.slint`, `ui/components/window_chrome.slint`, `ui/components/controls.slint` — кастомные контролы (0 дефолтных виджетов).
- `ui/settings.slint`, `ui/settings_general.slint`, `ui/settings_dock.slint`, `ui/settings_widgets.slint`, `ui/settings_overlay.slint` — окно настроек.
- `ui/notes.slint` — 3-колоночное окно заметок и чек-листов.
