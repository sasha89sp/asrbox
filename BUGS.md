# BUGS.md — Открытые баги и журнал попыток `asrbox`

Закрытые баги переносятся в `docs/archive/bugs-closed.md`.

## Протокол исправления багов

1. Фиксация (`BUG-NNN` с версией, крейтом и шагами воспроизведения).
2. Лог / пакет диагностики из трея (ZIP).
3. Локализация причины (с указанием файла и функции).
4. Минимальная правка.
5. Регрессионный тест.
6. Проверка в CI (`windows-latest`).
7. Закрытие и перенос записи в `docs/archive/bugs-closed.md`.

**Правило стоп-крана**: две неудачные попытки исправления подряд — остановка, фиксация попыток в журнале и сбор дополнительной диагностики.

---

## Открытые баги

### BUG-003 — Падение шага CI `Smoke test (--selftest)` с пустым `$LASTEXITCODE`
- **Версия**: `0.0.1`
- **Крейты / файлы**: `.github/workflows/ci.yml` (`Smoke test (--selftest)`), бинарник `crates/app` (`windows_subsystem = "windows"` в release)
- **Воспроизведение**: GitHub Actions `.github/workflows/ci.yml`, run `45127781497` / job `137536033873`.
- **Попытка 1**: заменён запуск на `Start-Process -Wait -PassThru` с проверкой `$proc.ExitCode`.
- **Статус**: исправление применено (пользователь собрал и запустил `.exe`; закроем после подтверждения зелёного лога CI).

### BUG-004 — Сильные тормоза отклика UI и контекстного меню в трее при почти нулевом CPU
- **Версия**: `0.0.1`
- **Крейты**: `crates/platform-windows`, `crates/app`, `crates/mod-widgets`
- **Воспроизведение**: запуск `asrbox.exe` на Windows, правый клик по значку в системном трее — меню открывается с большой задержкой (или только после движения мыши по окнам приложения), анимация дока дёргается при движении курсора.
- **Локализация**:
  1. `crates/platform-windows/src/window.rs` (`host_wnd_proc` + `push_event` + `run_event_loop`): `explorer.exe` доставляет `WM_TRAYICON` через не-очередной `SendMessage`/`SendNotifyMessage` прямо во время сна `GetMessageW`; `push_event` клал событие в `EVENT_QUEUE`, но не будил `GetMessageW`, поэтому цикл обработки событий спал до любого следующего `PostMessage`.
  2. `crates/platform-windows/src/tray.rs` и `widget_proc.rs`: после `TrackPopupMenuEx` отсутствовал обязательный `PostMessageW(hwnd, WM_NULL, 0, 0)` (Microsoft KB135788), а `WM_TRAYICON` срабатывал и на `WM_RBUTTONUP`, и на `WM_CONTEXTMENU`.
  3. `crates/app/src/docks.rs` (`step_dock`): `set_animation_timer` вызывался на каждый `WM_MOUSEMOVE`, сбрасывая 16-мс таймер Windows на каждый пиксель движения мыши.
  4. `crates/mod-widgets/src/calendar.rs` (`on_pointer_move`): на каждый пиксель движения мыши пересобирались строки календаря `build_display_rows`, даже когда наведённый элемент не менялся.
- **Попытка 1**: добавлено немедленное пробуждение `GetMessageW` через `PostMessageW(host_hwnd, WM_WAKE_EVENTS)` в `push_event`, добавлен `PostMessageW(WM_NULL)` после всех `TrackPopupMenuEx`, убран дубль `WM_CONTEXTMENU`, кэширован `timer_active` в `DockRuntime` и `shown_row_count` в `CalendarWidget`.
- **Статус**: правка готова, ожидает проверки CI (`windows-latest`) и ручной проверки на Windows.

### BUG-005 — Не открывается `widgets.cfg` из меню и не обновляется календарь после ручного сохранения
- **Версия**: `0.0.1`
- **Крейты**: `crates/mod-widgets`, `crates/platform-windows`
- **Локализация**:
  1. `crates/mod-widgets/src/calendar.rs` (`open_config_file`): вызывал `ShellExecuteW("open", "...widgets.cfg")` (на Windows для `.cfg` нет ассоциации -> `SE_ERR_NOASSOC`) и предварительно перезаписывал существующий `widgets.cfg` на диске.
  2. `crates/mod-widgets/src/storage.rs` (`parse_widget_config` / `parse_calendar_source`): не снимал UTF-8 BOM, кавычки и не принимал 2-частный формат `calendar=id|URL` или `url=URL`.
  3. `crates/platform-windows/src/http.rs` (`winhttp_get`): использовал устаревший `WINHTTP_ACCESS_TYPE_DEFAULT_PROXY` без таймаутов и скрывал реальный Win32 код ошибки.
- **Попытка 1**: открытие `.cfg` переведено на `notepad.exe` без перезаписи существующего файла; парсер `widgets.cfg` расширен и покрыт unit-тестами (BOM, кавычки, 1/2/3 части, `url=`); WinHTTP переведён на `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` + таймауты + реальные коды ошибок; добавлена очистка устаревшего кэша при `↻` и статус «Загрузка календаря...».
- **Дополнительная диагностика (скриншот пользователя + лог `mod-debug`)**: по логу `sources=0` и содержимому `widgets.cfg` установлено, что пользователь вставил рабочие URL прямо в строки-примеры, начинающиеся с `# calendar=...`, поэтому парсер пропускал их как комментарии.
- **Попытка 2**: добавлена автодиагностика закомментированных реальных `.ics` ссылок (`commented_calendar_hint` в `mod-widgets/src/storage.rs`), оранжевое предупреждение в подвале виджета, кнопка «Включить (убрать #)» и нативное поле ввода URL прямо во вкладке «Google Календарь» единого окна настроек (`mod-settings`).
- **Статус**: правка готова, ожидает проверки CI (`windows-latest`) и ручной проверки на Windows.

(Закрытые `BUG-001`, `BUG-002` перенесены в `docs/archive/bugs-closed.md`.)

---

## Известные ограничения среды

- **ENV-001**: Код пишется в Google AI Studio Build без локального Windows-раннера; компиляция, тесты, хуки и UI проверяются только через GitHub Actions (`windows-latest`) и ручной запуск на Windows.
