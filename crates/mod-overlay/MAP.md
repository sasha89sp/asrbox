# MAP.md — `crates/mod-overlay`

Краткая карта крейта игрового оверлея (`asrbox-mod-overlay`). Лимит: <= 100 строк.

- `src/lib.rs` — контроллер `GameOverlay` (трейт `Module`, режим настройки/блокировки click-through, авто-показ только в играх, тесты).
- `src/config.rs` — конфигурация `OverlayConfig`, якоря углов (`OverlayAnchor`), расчёт прямоугольника на активном мониторе, сериализация `overlay.cfg`.
- `src/draw.rs` — программная отрисовка BGRA-карточки в стиле NotePlan Dark, полос прогресса GPU/CPU/RAM, бейджа потерь пакетов и рамки настройки.
- `src/render.rs` — масштабируемая HUD-компоновка (FPS + 1% low, GPU + VRAM, CPU, RAM, NET ping + PL) под произвольный размер окна.
