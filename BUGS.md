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
- **Воспроизведение**: GitHub Actions `.github/workflows/ci.yml`, run `45127781497` / job `137536033873` — все шаги до `Build release binary` и проверки `< 25 MB` прошли успешно, но `Smoke test (--selftest)` упал с `Write-Error: Selftest exited with code ` (пустое значение `$LASTEXITCODE`).
- **Локализация**: в release-сборке `asrbox.exe` собран с `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`. В PowerShell (`pwsh`) запуск GUI-бинарника через `& ".\target\release\asrbox.exe" --selftest` не дожидается завершения и оставляет `$LASTEXITCODE` пустым (`$null`), из-за чего `if ($LASTEXITCODE -ne 0)` всегда истинно.
- **Попытка 1**: заменён запуск в `.github/workflows/ci.yml` на `$proc = Start-Process -FilePath ".\target\release\asrbox.exe" -ArgumentList "--selftest" -Wait -PassThru -ErrorAction Stop` с проверкой `$proc.ExitCode`.
- **Статус**: правка готова, ожидает подтверждения CI (`windows-latest`).

(Закрытые `BUG-001`, `BUG-002` перенесены в `docs/archive/bugs-closed.md`.)

---

## Известные ограничения среды

- **ENV-001**: Код пишется в Google AI Studio Build без локального Windows-раннера; компиляция, тесты, хуки и UI проверяются только через GitHub Actions (`windows-latest`) и ручной запуск на Windows.
