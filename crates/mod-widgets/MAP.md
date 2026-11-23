# MAP: crates/mod-widgets

Модуль виджетов рабочего стола (`WG-001..WG-018`): Google Календарь (`.ics`) и Системный монитор в едином стиле macOS Dark-Glass с интерактивным изменением размеров окон.
Зависит только от `crates/platform-api` (без `unsafe`, без сетевых/Win32-крейтов).

## Файлы

- `Cargo.toml` — манифест крейта `asrbox-mod-widgets`.
- `src/lib.rs` — публичные реэкспорты контроллеров виджетов, парсера, RRULE, рендереров и хранилищ.
- `src/date.rs` — календарная арифметика (`IcsDateTime`, `days_from_civil`, `civil_from_days`, `days_in_month`, `weekday_mon0`).
- `src/rrule.rs` — движок развёртывания повторов `RRULE` (`DAILY/WEEKLY/MONTHLY/YEARLY`, `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY`, `BYMONTHDAY`, `BYMONTH`, `WKST`).
- `src/rrule_parse.rs` — парсер строки `RRULE` (`parse_rrule`).
- `src/ics_time.rs` — парсинг дат/времени RFC 5545, таблица часовых поясов, unescape и детектор `Busy`.
- `src/ics.rs` — потоковый построчный парсер RFC 5545 (`parse_ics_bytes`).
- `src/format.rs` — форматирование сроков (`дата`, `23.11`, `отсчёт`), цвета индикаторов, подсказки и ссылки Google Calendar.
- `src/storage.rs` — загрузка/сохранение `widgets.cfg` и атомарный кэш `calendar_cache_<id>.ics`.
- `src/draw.rs` — общие BGRA-примитивы macOS Dark-Glass (стеклянная карточка, светящиеся cyan pill-бары, frosted badges, линии, уголок ресайза).
- `src/render.rs` — рендер карточки Календаря (`render_calendar_widget`), `rows_for_height` и `hit_test_widget`.
- `src/calendar_view.rs` — фильтрация событий, строки отображения, подсказки и строка статуса подвала.
- `src/calendar.rs` — контроллер `CalendarWidget` (окно, ресайз, кэш, фоновая загрузка, клики, прокрутка, меню и автосохранение).
- `src/sys_config.rs` — загрузка/сохранение `sysmon.cfg` (позиция, размер, слой, прозрачность, частота обновления `refresh_interval_ms`).
- `src/sys_render.rs` — каркас и шапка рендера Системного монитора (`render_sysmon_widget`, `hit_test_sysmon`, хелперы и тесты).
- `src/sys_sections.rs` — секции рендера Системного монитора (CPU, GPU, RAM/диски, сеть/задержки, топ-5 процессов).
- `src/sys_widget.rs` — контроллер `SysMonitorWidget` (`SYS_WIDGET_ID = 2`, ресайз, таймер опроса, меню и телеметрия).
