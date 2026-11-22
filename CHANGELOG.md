# CHANGELOG.md — История изменений `asrbox`

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), нумерация версий следует [Semantic Versioning](https://semver.org/lang/ru/).

## [0.0.1] - 2026-11-22

### Добавлено
- Корневой `.gitignore` с исключением `target/`, `data/`, `*.log` и локальных файлов настроек.
- Корневой `Cargo.toml` (`[workspace]`, `resolver = "2"`, `[workspace.package]` с `version = "0.0.1"` и `edition = "2021"`, lints workspace: `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`).
- Каркас 11 крейтов в `crates/` (`core`, `platform-api`, `platform-windows`, `mod-keyboard`, `mod-dock`, `mod-speech`, `mod-webdav`, `mod-monitor`, `mod-screenshot`, `mod-huawei`, `app`) с минимальными `Cargo.toml`, пустыми `src/lib.rs` (в `crates/app` — `src/main.rs` с пустой `main`) и файлами `MAP.md`.
- Служебный крейт `xtask` с командой `cargo run -p xtask -- atlas-check` и модульными тестами без использования `unwrap()`/`expect()`.
- Проектная документация в корне: `AGENTS.md`, `SPEC.md` (включая раздел 0 и бюджеты `NF-001..NF-010`), `ATLAS.md`, `HANDOFF.yaml`, `ARCHITECTURE.md`, `DECISIONS.md`, `BUGS.md`, `CHANGELOG.md`.
- Релизный чеклист `docs/RELEASE_CHECKLIST.md`.
- Конвейер GitHub Actions `.github/workflows/ci.yml` для `windows-latest` с кешированием `Swatinem/rust-cache@v2` и проверками `fmt`, `clippy`, `atlas-check`, `test` и `build --release`.
