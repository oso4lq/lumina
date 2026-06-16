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
| **v0.3b — Bottom bar** | 🟢 | Divider toggle, мета-панель, карусель миниатюр (lazy in-memory, aspect-лента), кнопки поворот/fullscreen/EXIF, fullscreen-оверлей play/выход (авто-скрытие), иконки Tabler | [дизайн](docs/superpowers/specs/2026-06-15-lumina-v0.3b-bottom-bar-design.md) · [план](docs/superpowers/plans/2026-06-15-lumina-v0.3b-bottom-bar.md) |
| **v0.4a — Трансформации** | 🟢 | Авто-ориентация по EXIF Orientation при декоде (JPEG/TIFF/WebP + Bayer-RAW; HEIC ориентирует libheif), ручной поворот/отражение `R`/`Shift+R`/`H`/`V`/`Ctrl+Z` + кнопка, сессионная память трансформаций | [дизайн](docs/superpowers/specs/2026-06-16-lumina-v0.4a-transforms-design.md) · [план](docs/superpowers/plans/2026-06-16-lumina-v0.4a-transforms.md) |
| **v0.4c — Навигация и эргономика ввода** | 🟢 | Раскладко-независимые шорткаты (по физ. клавише), экранные стрелки ‹ › на hover (оконный режим), свайп drag-жестом при fit | [дизайн](docs/superpowers/specs/2026-06-16-lumina-v0.4c-navigation-input-design.md) · [план](docs/superpowers/plans/2026-06-16-lumina-v0.4c-navigation-input.md) |
| **v0.4b — EXIF popup/запись** | ⚪ | EXIF popup с редактированием, запись тегов / XMP sidecar (ExifTool), модель камеры в заголовке, запись EXIF Orientation в оригиналы | — |
| **v0.5 — Полировка** | ⚪ | Кэш миниатюр (sled), префетч ±2, folder watcher (notify), свайп трекпадом, installer + реестр | — |
| **v0.6 — Slideshow и пр.** | ⚪ | Slideshow (кнопка play-задел уже в fullscreen-оверлее), темизация (System/Dark/Light), multi-monitor fullscreen, режимы сортировки каталога | — |

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
- [x] Мета-панель слева: формат / разрешение (`WxHpx`) / размер файла — в столбик без заголовков, обновляется при смене фото
- [x] Карусель: плёночная лента — высота фиксирована (64px), ширина по аспекту фото (без кропа), SDF-скругление
      (`ThumbnailLayer` + `assets/shaders/thumb.wgsl`); активная — белая рамка-контур, плашка бейджа у RAW,
      горизонтальный скролл колесом (clamp), клик открывает фото
- [x] `ThumbnailStore` (`src/thumbnail.rs`): окно запроса, LRU-эвикция, поколение — юнит-тесты; декод на rayon
      (`decode_preview` иначе `decode_full` → ресайз через `image`), in-memory; троттлинг (≤4 одновременных декодов)
- [x] Кнопки действий справа (Tabler Icons, `assets/fonts/tabler-icons.ttf`): `[поворот] [fullscreen] [i]`;
      fullscreen рабочая (`F`/`F11`/`Esc`); поворот и EXIF — задел, клик no-op
- [x] Fullscreen: хром скрыт, фото на весь монитор без каймы (`WM_NCCALCSIZE` отдаёт полный клиент в fullscreen),
      нативная инертность (`WM_NCHITTEST` → HTCLIENT); оверлей справа-сверху `[play] [выход]` — показ по движению
      курсора, плавное гашение через 3 с простоя
- [x] Порядок кадра: фото в viewer-инсете → фон-подложки (`Bg`) → миниатюры (scissor по карусели) →
      overlay-rect'ы (`Overlay`: рамка/бейджи) → текст. Миниатюры — батч-инстансы (одна запись буфера, `draw 0..4, i..i+1`)
- [x] Системный заголовок окна = `имя · Lumina` (таскбар/alt-tab)

Код и юнит-тесты готовы (74 теста). **Задел под будущие фазы (кнопки есть, логики пока нет):** поворот
(`R`/`Shift+R`, v0.4), slideshow `[play]` (v0.6). Отложено: sled-кэш миниатюр и префетч (v0.5), EXIF popup (v0.4),
авто-скрытие курсора в fullscreen (v0.6).

## Прогресс v0.4a (детально)

Дизайн: два слоя поворота, разведённые по уровням —
(1) EXIF-ориентация «запекается» в пиксели в слое декодера (каждый декодер отдаёт upright);
(2) ручной поворот/отражение живёт в матрице вида (мгновенно, без передекода), стартует с identity.

- [x] `view::Transform { rotation: u16, flip_h, flip_v }` + API `ViewTransform` (`rotate_cw`/`rotate_ccw`/
      `flip_horizontal`/`flip_vertical`/`reset_transform`/`set_transform`/`transform`); `rotation` в градусах
      ∈ {0,90,180,270} (`u16`, т.к. 270 не влезает в `u8`)
- [x] Поворот/отражение в матрице вида: центрированная формула `T(win/2+pan)·R(rot)·S(±scaled)·T(-0.5)`
      (знак масштаба кодирует отражение), `effective_dims` (90°/270° свопят W↔H), rotation-aware `clamp_pan`
      и метод `fit_zoom`, `screen_quad` (углы на экране); для identity эквивалентна прежней формуле
- [x] Модуль `src/exif`: enum `Orientation` (1..8), `read_orientation(path)` через `kamadak-exif`
      (любая ошибка → Normal), `apply_to_image` через `image`-imageops (приведение к upright)
- [x] `StandardDecoder` авто-ориентирует JPEG/TIFF/WebP по EXIF Orientation перед `to_rgba8`
- [x] `RawDecoder` ориентирует ТОЛЬКО развёрнутый Bayer (develop-ветка): rawler не применяет ориентацию;
      встроенный JPEG камеры (preview/non-Bayer X-Trans) уже ориентирован — не трогаем (без двойного поворота)
- [x] HEIC — без изменений кода (libheif применяет `irot`/`imir` сам), поведение зафиксировано `#[ignore]`-тестом
- [x] Интеграция в `app.rs`: клавиши `R`/`Shift+R`/`H`/`V`/`Ctrl+Z`, оживлена кнопка поворота bottom bar,
      сессионная память `HashMap<PathBuf, Transform>` (восстановление при загрузке, очистка при смене папки),
      rotation-aware пересчёт fit во всех точках (Resized/Redraw/первый кадр/toggle_fit/set_fit_view)

Код и юнит-тесты готовы (91 тест зелёный + 5 `#[ignore]`: реальные RAW/HEIC-образцы, в т.ч. приёмка
`raw_portrait_is_upright`/`heic_portrait_is_upright`). 17 новых тестов в v0.4a (Transform API, геометрия
матрицы, exif, авто-ориентация JPEG). Память трансформаций — сессионная, на диск НЕ пишется (запись/sidecar — v0.4b).

> **Зависимость:** добавлен `kamadak-exif` (pure-Rust) — новых нативных зависимостей сборки v0.4a не вводит.
> Визуальную GUI-приёмку (портретные фото с телефона вертикально, R/H/V/Ctrl+Z, отсутствие двойного
> поворота HEIC/RAW) подтверждает пользователь вручную.

## Прогресс v0.4c (детально)

Дизайн: вся логика — в чистом ядре (`input`/`view`/`ui`), `app.rs` — тонкий диспетчер.

- [x] Раскладко-независимые шорткаты: `input::map_key(code, ctrl, shift)` матчит по физической
      позиции клавиши (`KeyCode`), а не по символу → `R`/`Shift+R`/`H`/`V`/`F`/`Ctrl+Z`/`Ctrl+0`/`Ctrl+1`
      работают на любой раскладке (кириллица К/Р/М/Ф/Я и др.). В `app.rs` диспетчинг переключён на
      `physical_key`; NamedKey (F11/Escape/стрелки/Home/End) остаются по `logical_key` (они уже layout-independent)
- [x] Экранные стрелки навигации ‹ ›: тонкие полосы у краёв viewer (`ui::layout` `nav_prev`/`nav_next`,
      нулевые в fullscreen), хит-регионы `NavPrev`/`NavNext` (`ui::hit`, крайние 6px — приоритет ресайза),
      шевроны Tabler с проявлением по hover-альфе (~0.12 с) и инертностью на краях каталога (`can_prev`/`can_next`);
      клик листает фото
- [x] Свайп-перелистывание при размере по умолчанию (fit): drag мышью двигает фото за курсором через
      транзиентное `view::swipe_offset` (в матрице, не клампится, переживает ежекадровый fit-сброс pan);
      `input::on_swipe_release(dx, viewer_w)` решает по порогу 15% ширины (влево → следующее, вправо →
      предыдущее); недотянутое/край каталога → плавный откат (~0.2 с). При зуме drag по-прежнему панорамирует
- [x] ROADMAP/дизайн/план

Код и юнит-тесты готовы (107 тестов зелёных + 5 `#[ignore]`). 16 новых тестов в v0.4c (`map_key`,
`on_swipe_release`, `swipe_offset` в матрице, геометрия/хит/отрисовка стрелок). Визуальную GUI-приёмку
(раскладки, hover-стрелки, свайп-жест) подтверждает пользователь вручную.

> **#2 (запись EXIF Orientation в оригиналы)** остаётся в v0.4b (требует ExifTool/записи тегов).

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
