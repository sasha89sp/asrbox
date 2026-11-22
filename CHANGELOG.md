# CHANGELOG.md — История изменений `asrbox`

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), нумерация версий следует [Semantic Versioning](https://semver.org/lang/ru/).

## [0.0.1] - 2026-11-22

### Добавлено
- Корневой `.gitignore` с исключением `target/`, `data/`, `*.log` и локальных файлов настроек.
- Корневой `Cargo.toml` (`[workspace]`, `resolver = "2"`, `[workspace.package]` с `version = "0.0.1"` и `edition = "2021"`, lints `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`).
- Конфигурация `.cargo/config.toml` с алиасом `cargo xtask`.
- Каркас 11 крейтов в `crates/` (`core`, `platform-api`, `platform-windows`, `mod-keyboard`, `mod-dock`, `mod-speech`, `mod-webdav`, `mod-monitor`, `mod-screenshot`, `mod-huawei`, `app`) с минимальными `Cargo.toml`, пустыми `src/lib.rs` (в `crates/app` — `src/main.rs` с пустой `main`) и картами `MAP.md`.
- Утилита `xtask` (`cargo xtask atlas-check` / `cargo run -p xtask -- atlas-check`) без внешних зависимостей и без `unwrap()`/`expect()`.
- Полный пакет проектной документации по руководству: `AGENTS.md`, `SPEC.md` (с бюджетами `NF-001..NF-010`), `ATLAS.md`, `HANDOFF.yaml`, `ARCHITECTURE.md`, `DECISIONS.md`, `BUGS.md`, `CHANGELOG.md`.
- Каталоги и документы сопровождения: `specs/README.md`, `docs/RELEASE_CHECKLIST.md`, `docs/LICENSES.md`, `docs/archive/bugs-closed.md`.
- Конвейеры GitHub Actions: `.github/workflows/ci.yml` (`windows-latest`, `Swatinem/rust-cache@v2`, `fmt`, `clippy`, `xtask atlas-check`, `test`, `build --release`, проверка лимита `< 25 МБ`, смоук `--selftest`, выгрузка артефакта) и `.github/workflows/release.yml` (проверка тега `v*`, переименование `asrbox-vX.Y.Z.exe`, SHA-256, GitHub Releases).
