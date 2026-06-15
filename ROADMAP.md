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
| **v0.3a — UI-фундамент + titlebar** | 🟢 | Движок UI-примитивов (wgpu SDF-rect + glyphon-текст + Segoe MDL2-иконки), нативный frameless-фрейм (WM_NCCALCSIZE/NCHITTEST), кастомный titlebar | [дизайн](docs/superpowers/specs/2026-06-15-lumina-v0.3a-ui-foundation-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-v0.3a-ui-foundation.md) |
| **v0.3b — Bottom bar** | 🟢 | Divider toggle, мета-панель, карусель миниатюр (lazy in-memory), кнопки fullscreen/EXIF, иконки Tabler | [дизайн](docs/superpowers/specs/2026-06-15-lumina-v0.3b-bottom-bar-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-v0.3b-bottom-bar.md) |
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

## Прогресс v0.3a (детально)

- [x] Чистое ядро UI (`src/ui/`): `theme` (палитра + srgb→linear), `layout` (titlebar/кнопки/viewer
      в физ. px), `hit` (курсор→регион), `scene` (UiState → `DrawCmd`) — покрыто юнит-тестами (17 новых)
- [x] `UiPipeline` — инстансовый SDF rounded-rect (`assets/shaders/ui.wgsl`)
- [x] `TextLayer` — обёртка glyphon (текст заголовка + глифы кнопок из Segoe MDL2 Assets)
- [x] Композиция кадра: фото в viewer-viewport (под titlebar) → UI-прямоугольники → текст
- [x] Кастомный titlebar: заголовок `имя · Lumina`, кнопки min/max/close, hover (close — красный),
      глиф max↔restore
- [x] Нативный frameless (`src/platform/windows.rs`): `WM_NCCALCSIZE` убирает caption (сохраняя
      resize/Aero Snap/тень/скруглённые углы), `WM_NCHITTEST` мапит регионы в HT-коды (caption-drag, края)
- [x] Viewer-инсет вида через GPU-viewport (`ViewTransform` неизменён); zoom к курсору скорректирован
      на высоту titlebar
- [x] Гард рендера от вырожденно маленького окна (ниже titlebar)

Код и юнит-тесты готовы (46 тестов). Приёмка на реальных файлах пройдена: titlebar поверх фото,
frameless с resize/snap/тенью, кнопки окна, zoom/pan/навигация. EXIF-модель камеры в заголовке —
отложена в v0.4 (пока `имя · Lumina`). Bottom bar / карусель / divider — v0.3b.

> **Окружение сборки/запуска** идентично v0.2 (vcpkg/libclang ниже). Доп. зависимостей нативной
> сборки v0.3a не вводит: glyphon — pure-Rust, `windows`-crate тянет только заголовки Win32.

## Прогресс v0.3b (детально)

- [x] Раскладка bottom bar (`ui::layout`): зоны divider/мета/карусель/кнопки, `compute(win, scale,
      bottom_factor, fullscreen)`, `carousel_thumb_rects`/`carousel_content_width` — юнит-тесты
- [x] Divider: грип по центру, метка «карусель» с фейдом по `bottom_factor`; клик скрывает/показывает
      bottom bar с анимацией (~200 мс), фото занимает освободившееся место
- [x] Мета-панель слева: формат (RAF · RAW / JPG), MP, разрешение, размер файла — обновляется при смене фото
- [x] Карусель: lazy-миниатюры 62×64 со SDF-скруглением (`ThumbnailLayer` + `assets/shaders/thumb.wgsl`),
      активная — белая рамка, плашка бейджа у RAW, горизонтальный скролл колесом (clamp), клик открывает фото
- [x] `ThumbnailStore` (`src/thumbnail.rs`): окно запроса, LRU-эвикция, поколение, cover-кроп — юнит-тесты;
      декод на rayon (`decode_preview` иначе `decode_full` → ресайз через `image`), in-memory
- [x] Кнопки действий (Tabler Icons, `assets/fonts/tabler-icons.ttf`): fullscreen (кнопка/`F`/`F11`,
      `Esc` — выход; хром скрыт, фото на весь монитор); EXIF — hover есть, клик no-op
- [x] Нативная инертность fullscreen: `WM_NCHITTEST` возвращает HTCLIENT (окно не двигается/не тянется)
- [x] Порядок кадра: фото в viewer-инсете (titlebar + нижний хром) → миниатюры → UI-rect'ы → текст

Код и юнит-тесты готовы (70 тестов). Отложено: sled-кэш миниатюр и префетч (v0.5), EXIF popup (v0.4),
slideshow (v0.6).

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
