# MAP: crates/mod-widgets

Модуль виджетов рабочего стола (`WG-001..WG-012`); первый виджет — Google Календарь (`.ics`, стиль macOS Dark).
Зависит только от `crates/platform-api` (без `unsafe`, без сетевых/Win32-крейтов).

## Файлы

- `Cargo.toml` — манифест крейта `asrbox-mod-widgets`.
- `src/lib.rs` — публичные реэкспорты контроллера, парсера, RRULE, рендерера и хранилища.
- `src/date.rs` — календарная арифметика (`IcsDateTime`, `days_from_civil`, `civil_from_days`, `days_in_month`, `weekday_mon0`).
- `src/rrule.rs` — движок развёртывания повторов `RRULE` (`DAILY/WEEKLY/MONTHLY/YEARLY`, `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` с `2MO`/`-1FR`, `BYMONTHDAY`, `BYMONTH`, `WKST`).
- `src/rrule_parse.rs` — парсер строки `RRULE` (`parse_rrule`).
- `src/ics_time.rs` — парсинг дат/времени RFC 5545, таблица часовых поясов, unescape (`\,`, `\;`, `\n`) и детектор `Busy`.
- `src/ics.rs` — потоковый построчный парсер RFC 5545 (`parse_ics_bytes`): unfolding, `VTIMEZONE`, `EXDATE`, `RECURRENCE-ID`, `STATUS:CANCELLED`, `VALUE=DATE`, быстрый пропуск архива.
- `src/format.rs` — форматирование сроков (`дата`, `23.11`, `отсчёт`), цвета индикаторов (сегодня/завтра/позже/кастом), подсказки и ссылки Google Calendar.
- `src/storage.rs` — загрузка/сохранение `widgets.cfg` и атомарный кэш `calendar_cache_<id>.ics`.
- `src/draw.rs` — программные BGRA-примитивы (скруглённая карточка, рамка, иконка календаря, стрелки).
- `src/render.rs` — рендер карточки macOS Dark (`render_calendar_widget`) и `hit_test_widget`.
- `src/calendar_view.rs` — фильтрация событий, строки отображения, подсказки и строка статуса подвала.
- `src/calendar.rs` — контроллер `CalendarWidget` (окно, кэш, фоновая загрузка, клики, прокрутка, меню и автосохранение).
