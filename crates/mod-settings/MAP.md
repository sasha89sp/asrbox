# MAP: crates/mod-settings

> Единое окно настроек (`AsrGlass` default + `Win11Classic`), аккордеоны с сохранением состояния, глобальный поиск по 4 вкладкам и выпадающие списки ComboBox.
> Pure Rust layout & software BGRA + ClearType renderer; depends only on `core` and `platform-api`.

## Files
- `src/lib.rs` — `SettingsUiState`: состояние окна (860×560), аккордеоны, подтверждение сброса, браузерные вкладки и рендер.
- `src/ui_state_store.rs` — загрузка/сохранение раскрытых аккордеонов в `data/config/settings_ui.cfg`.
- `src/events.rs` — обработка кликов, аккордеонов, ComboBox, ползунков, скролла и поиска.
- `src/tests.rs` — модульные тесты всех вкладок, 4 подвкладок виджетов, глобального поиска и тем.
- `src/types.rs` — DTO (`SettingsTab`, 4 `WidgetSubTab`, `SettingsSnapshot`, `WeatherSettingsView`, `NotesSettingsView`, `GitHubBackupView`).
- `src/actions.rs` — `SettingsAction`, `ComboKind` и `SliderKind`.
- `src/i18n.rs` — мгновенная RU/EN локализация (`tr`).
- `src/layout.rs` — сайдбар `210px`, строка поиска (без кнопки «Сбросить» в шапке), глобальный поиск и `LayoutBuilder::accordion`/`push_combo`.
- `src/layout_types.rs` — геометрия (`MIN 720×460`), `HitKind` (`AccordionHeader`, `ComboBox`, `StartResetConfirm`), `CardBlock`.
- `src/layout_general.rs` — вкладка «Общие настройки»: Оформление, Система + подтверждение сброса, Меню трея, Бэкап AES-256, Диагностика, Пути, GitHub.
- `src/layout_dev.rs` — аккордеон «Для разработчика (GitHub)» и настройка пунктов контекстного меню (Трей / Док / Виджеты).
- `src/layout_browser_tabs.rs` — внутренние вкладки: Диагностика, История версий и WebDAV браузер.
- `src/layout_tabs.rs` — общая карточка модуля и вкладка «Виджеты» с 4 подвкладками (Календарь, Монитор, Погода, Заметки).
- `src/layout_weather_notes.rs` — настройки виджетов «Погода» (Open-Meteo) и «Заметки и задачи».
- `src/layout_dock.rs` — вкладка «Док-панели»: внешний вид, «Закрепить значки», переименование и меню дока.
- `src/layout_cal.rs` — настройки Google Календаря.
- `src/layout_sys.rs` — настройки Системного монитора.
- `src/layout_overlay.rs` — настройки Игрового оверлея в аккордеонах.
- `src/layout_backup.rs` — аккордеон Бэкапа, шифрование ZIP AES-256 (слот 5) и WebDAV серверы.
- `src/layout_sync.rs` — черновик WebDAV сервера и синхронизация файлов/папок.
- `src/draw.rs` — 2D BGRA примитивы (карточки, скругления, переключатели, ползунки).
- `src/render.rs` — рендерер с логотипом `ASR Glass` `52×52`, радиусом 10px и 9 слотами полей ввода.
- `src/render_controls.rs` — рендер аккордеонов, ComboBox, кнопок, переключателей и вкладок.
