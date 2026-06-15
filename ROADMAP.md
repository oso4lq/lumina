# Lumina — дорожная карта разработки

Просмотрщик фотографий для Windows на Rust + wgpu. Полная спецификация — `SPEC_lumina.md`.
Дизайны по фазам — `docs/superpowers/specs/`. Планы реализации — `docs/superpowers/plans/`.

## Статусы

🟢 готово · 🟡 в работе · ⚪ не начато

## Стадии

| Фаза | Статус | Описание | Артефакты |
|---|---|---|---|
| **v0.1 — Viewer core** | 🟢 | Окно + wgpu, показ JPEG/PNG, zoom/pan, double-click fit/100%, навигация стрелками | [дизайн](docs/superpowers/specs/2026-06-15-lumina-v0.1-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-v0.1.md) |
| **v0.2 — RAW и форматы** | 🟢 | rawler (RAF/NEF/ARW/CR2), HEIC через libheif-rs, embedded preview, async-декод полного RAW | [дизайн](docs/superpowers/specs/2026-06-15-lumina-v0.2-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-v0.2.md) |
| **v0.3 — UI** | ⚪ | Кастомный frameless titlebar, карусель миниатюр, divider toggle, мета-панель, кнопки fullscreen/EXIF | — |
| **v0.4 — EXIF и трансформации** | ⚪ | Чтение EXIF (kamadak-exif), EXIF popup с редактированием, запись/XMP sidecar, повороты/отражения | — |
| **v0.5 — Полировка** | ⚪ | Кэш миниатюр (sled), префетч ±2, folder watcher (notify), свайп трекпадом, installer + реестр | — |
| **v0.6 — Slideshow и пр.** | ⚪ | Slideshow, темизация (System/Dark/Light), multi-monitor fullscreen, режимы сортировки каталога | — |

## Прогресс v0.1 (детально)

Пункты из §12 спеки:

- [x] winit-окно + wgpu-контекст
- [x] Загрузка и показ JPEG/PNG
- [x] Zoom (колёсико) и pan (drag)
- [x] Double-click fit ↔ 100%
- [x] Навигация стрелками

Прогресс по задачам плана отслеживается в `docs/superpowers/plans/2026-06-15-lumina-v0.1.md`.

## Прогресс v0.2 (детально)

- [x] rawler интеграция (RAF, NEF, ARW, CR2, …)
- [x] HEIC через libheif-rs
- [x] Embedded preview для мгновенного показа
- [x] Async полное декодирование RAW (двухстадийный конвейер preview→full)
- [x] Точный цвет RAW: для Fuji X-Trans — встроенный JPEG камеры (rawler демозаит X-Trans
      некорректно → зеленца); для байеровских RAW — полноценный develop ([дизайн](docs/superpowers/specs/2026-06-15-lumina-raw-accurate-color-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-raw-accurate-color.md))

Код и юнит-тесты готовы (29 тестов, RAW/HEIC-фикстуры — `#[ignore]`). Приёмка на реальных
файлах пройдена: HEIC, Fuji RAF (естественный цвет), DNG открываются.

**Известное ограничение — Lightroom HDR DNG (float JPEG XL).** Полноразмерные данные таких DNG
(напр. 7728×5152) хранятся как float-JPEG-XL тайлами — rawler 0.7.2 это не декодирует
(JPEG XL у него только для целочисленных данных). Показываем встроенное превью 1024px
(цвет верный, мягко). Истинный full-res потребовал бы libraw + Adobe DNG SDK (тяжёлая
нативная сборка на MSVC) — отложено. Детали разбора — в `docs/superpowers/specs/`.

## Установленное окружение

- 🟢 Rust 1.96.0 stable (`x86_64-pc-windows-msvc`)
- 🟢 MSVC Build Tools 14.51 + Windows SDK 10.0.26100
- 🟢 Git, winget
- 🟢 LLVM/Clang 22.1.7 (`libclang`) — bindgen для libheif-rs
- 🟢 CMake 4.3.3
- 🟢 vcpkg + libheif 1.23.0 (libde265/x265) — HEIC. Линковка проверена (`cargo run --bin smoke_libheif`)
- ⚪ ExifTool — v0.4 (запись EXIF)

> **Окружение сборки/запуска v0.2** (user-env уже выставлены через `setx`):
> `VCPKG_ROOT=~/vcpkg`, `LIBCLANG_PATH=C:\Program Files\LLVM\bin`, `VCPKGRS_DYNAMIC=1`.
> Для запуска приложения каталог `~/vcpkg/installed/x64-windows/bin` (heif.dll/libde265.dll)
> должен быть в `PATH`, либо DLL копируются рядом с exe.
