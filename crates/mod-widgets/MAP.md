# MAP: crates/mod-widgets

Модуль виджетов рабочего стола (`WG-001..WG-024`): Google Календарь (`.ics`), интерактивный Системный монитор, Погода (Open-Meteo) и Заметки/Задачи (3-колоночный редактор) с темами `AsrGlass` и `Win11Classic`.
Зависит только от `crates/platform-api` (без `unsafe`, без сетевых/Win32-крейтов).

## Файлы

- `Cargo.toml` — манифест крейта `asrbox-mod-widgets`.
- `src/lib.rs` — публичные реэкспорты всех 4 виджетов, парсеров, рендереров и хранилищ.
- `src/date.rs` — календарная арифметика (`IcsDateTime`, `days_from_civil`, `civil_from_days`, `days_in_month`, `weekday_mon0`).
- `src/rrule.rs` — движок развёртывания повторов `RRULE`.
- `src/rrule_parse.rs` — парсер строки `RRULE` (`parse_rrule`).
- `src/ics_time.rs` — парсинг дат/времени RFC 5545, часовые пояса, unescape и детектор `Busy`.
- `src/ics.rs` — потоковый построчный парсер RFC 5545 (`parse_ics_bytes`).
- `src/format.rs` — форматирование сроков, цвета индикаторов, подсказки и ссылки Google Calendar.
- `src/storage.rs` — загрузка/сохранение `widgets.cfg` и кэш `calendar_cache_<id>.ics`.
- `src/draw.rs` — общие BGRA-примитивы карточек (`AsrGlass` 12px и `Win11Classic` 16px, полосы прогресса, бейджи, уголок ресайза).
- `src/render.rs` — рендер карточки Календаря (`render_calendar_widget`) и `hit_test_widget`.
- `src/calendar_view.rs` — фильтрация событий, строки отображения, подсказки и подвал Календаря.
- `src/calendar.rs` — контроллер `CalendarWidget` (`CALENDAR_WIDGET_ID = 1`).
- `src/sys_config.rs` — загрузка/сохранение `sysmon.cfg`.
- `src/sys_render.rs` — каркас, интерактивный hit-test секций (`SysHitTarget`) и шапка Системного монитора.
- `src/sys_sections.rs` — кликабельные секции Системного монитора (CPU, GPU, RAM/диски, сеть, топ-5 процессов) с hover-подсветкой.
- `src/sys_widget.rs` — контроллер `SysMonitorWidget` (`SYS_WIDGET_ID = 2`, запуск системных утилит по клику).
- `src/weather_config.rs` — загрузка/сохранение `weather.cfg` (город, координаты, интервал, размер, слой, прозрачность).
- `src/weather_api.rs` — URL-генератор и парсер JSON ответов Open-Meteo (geocoding + forecast) без сторонних крейтов.
- `src/weather_render.rs` — рендер карточки Погоды (шапка, температура, 3 плашки, 5 почасовых слотов, 4 дня с полосами диапазона).
- `src/weather_widget.rs` — контроллер `WeatherWidget` (`WEATHER_WIDGET_ID = 3`, фоновая загрузка, меню, ресайз).
- `src/notes_model.rs` — модель `NotesWidgetConfig`, `NotesDb`, `FolderNode`, `NoteDocument`, `TaskItem` и видимые колонки.
- `src/notes_storage.rs` — загрузка/сохранение `notes.cfg` и оффлайн-базы `data/notes/notes_db.cfg`.
- `src/notes_render.rs` — каркас 3-колоночного окна Заметок, верхняя панель `[◧][◫][▭]` и колонка папок.
- `src/notes_panes.rs` — рендер средней колонки списка заметок и правой колонки редактора с чекбоксами задач.
- `src/notes_menu.rs` — контекстные меню папок, карточек заметок и окна Заметок (`WidgetMenuVisibility`).
- `src/notes_widget.rs` — контроллер `NotesWidget` (`NOTES_WIDGET_ID = 4`, обработка `NotesUiEvent` и сохранение).
