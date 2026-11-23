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

### BUG-002 — Падение CI на шаге `Check formatting` (`cargo fmt --check`) и импортах `windows 0.61`
- **Версия**: `0.0.1`
- **Крейты**: `crates/mod-widgets`, `crates/mod-dock`, `crates/platform-api`, `crates/platform-windows`, `crates/app`
- **Воспроизведение**: GitHub Actions `.github/workflows/ci.yml` → шаг `Check formatting` (`cargo fmt --check`), run `45121511537` / job `137516111111`.
- **Лог**: `exit code 1` (568 строк diff; порядок `use` Edition 2021, перенос `let ... =`, `chain_width = 60`). При кросс-проверке `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` также выявлены: неверное имя фичи `Win32_Graphics_Dwmapi` (нужно `Win32_Graphics_Dwm`), отсутствующая фича `Win32_Security` для `CreateMutexW`, пути импорта `ClientToScreen` (`Graphics::Gdi`) и `WM_MOUSELEAVE` (`0x02A3`), а также 2 линта Rust 1.100 (`chunks_exact_to_as_chunks` в `icons.rs`, `field_reassign_with_default` в тесте `storage.rs`).
- **Попытка 1**: выполнен `cargo fmt`, исправлены фичи/импорты `windows 0.61` и 2 предупреждения Clippy 1.100. Локально пройдены `cargo fmt --check`, `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings`, `cargo xtask atlas-check`, `cargo test --workspace` (20/20) и `--selftest`.
- **Статус**: правка готова, ожидает подтверждения CI (`windows-latest`).

(Закрытый `BUG-001` перенесён в `docs/archive/bugs-closed.md`.)

---

## Известные ограничения среды

- **ENV-001**: Код пишется в Google AI Studio Build без локального Windows-раннера; компиляция, тесты, хуки и UI проверяются только через GitHub Actions (`windows-latest`) и ручной запуск на Windows.
