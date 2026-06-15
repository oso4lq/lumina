# Lumina — дорожная карта разработки

Просмотрщик фотографий для Windows на Rust + wgpu. Полная спецификация — `SPEC_lumina.md`.
Дизайны по фазам — `docs/superpowers/specs/`. Планы реализации — `docs/superpowers/plans/`.

## Статусы

🟢 готово · 🟡 в работе · ⚪ не начато

## Стадии

| Фаза | Статус | Описание | Артефакты |
|---|---|---|---|
| **v0.1 — Viewer core** | 🟡 | Окно + wgpu, показ JPEG/PNG, zoom/pan, double-click fit/100%, навигация стрелками | [дизайн](docs/superpowers/specs/2026-06-15-lumina-v0.1-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-v0.1.md) |
| **v0.2 — RAW и форматы** | ⚪ | rawler (RAF/NEF/ARW/CR2), HEIC через libheif-rs, embedded preview, async-декод полного RAW | — |
| **v0.3 — UI** | ⚪ | Кастомный frameless titlebar, карусель миниатюр, divider toggle, мета-панель, кнопки fullscreen/EXIF | — |
| **v0.4 — EXIF и трансформации** | ⚪ | Чтение EXIF (kamadak-exif), EXIF popup с редактированием, запись/XMP sidecar, повороты/отражения | — |
| **v0.5 — Полировка** | ⚪ | Кэш миниатюр (sled), префетч ±2, folder watcher (notify), свайп трекпадом, installer + реестр | — |
| **v0.6 — Slideshow и пр.** | ⚪ | Slideshow, темизация (System/Dark/Light), multi-monitor fullscreen, режимы сортировки каталога | — |

## Прогресс v0.1 (детально)

Пункты из §12 спеки:

- [ ] winit-окно + wgpu-контекст
- [ ] Загрузка и показ JPEG/PNG
- [ ] Zoom (колёсико) и pan (drag)
- [ ] Double-click fit ↔ 100%
- [ ] Навигация стрелками

Прогресс по задачам плана отслеживается в `docs/superpowers/plans/2026-06-15-lumina-v0.1.md`.

## Установленное окружение

- 🟢 Rust 1.96.0 stable (`x86_64-pc-windows-msvc`)
- 🟢 MSVC Build Tools 14.51 + Windows SDK 10.0.26100
- 🟢 Git
- ⚪ LLVM/Clang (`libclang`) — понадобится в v0.2 (HEIC, libraw)
- ⚪ vcpkg + libheif + CMake — v0.2
- ⚪ ExifTool — v0.4 (запись EXIF)
