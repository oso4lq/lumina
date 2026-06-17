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
| **v0.4b — EXIF popup/запись** | 🟢 | Часть 1 (чтение): EXIF popup-просмотрщик (полный браузер тегов exiftool, группировка/поиск/скролл), модель камеры в заголовке. Часть 2 (запись): инлайн-редактирование тегов записываемых групп (EXIF/XMP/IPTC/GPS), запись через exiftool (in-place + `_original`, RAW → XMP sidecar), «Удалить всё GPS», футер Save/Отменить всё, подтверждение несохранённых изменений, clipboard (Ctrl+C/V/X). Бандл exiftool → перенесён в v0.5 (вместе с инсталлятором) | [дизайн ч.1](docs/superpowers/specs/2026-06-16-lumina-v0.4b-exif-design.md) · [дизайн ч.2](docs/superpowers/specs/2026-06-16-lumina-v0.4b-exif-write-design.md) · [план ч.1](docs/superpowers/plans/2026-06-16-lumina-v0.4b-exif-read.md) · [план ч.2](docs/superpowers/plans/2026-06-16-lumina-v0.4b-exif-write.md) |
| **v0.4d — Починка записи EXIF + режимы сохранения** | 🟢 | Починка записи RAW: **in-place вместо XMP sidecar** (sidecar из v0.4b терял правки) — все форматы пишутся через exiftool in-place. Два режима: обычный (бэкап `_original`, обратимо) и необратимый (`-overwrite_original`, без бэкапа) через тоггл «Необратимо». Действие «Стереть всё» (`-all=` с сохранением Orientation/ICC). Обобщённый `ConfirmKind` (закрытие/перезапись/стирание) | [дизайн](docs/superpowers/specs/2026-06-17-lumina-v0.4d-exif-write-modes-design.md) · [план](docs/superpowers/plans/2026-06-17-lumina-v0.4d-exif-write-modes.md) |
| **v0.5 — Полировка** | 🟡 | Кэш миниатюр (диск), префетч ±2, folder watcher (notify), свайп трекпадом, installer + реестр, **бандл exiftool** (standalone exe + `exiftool_files` рядом с exe — приложение работает без отдельной установки exiftool) | A: [дизайн](docs/superpowers/specs/2026-06-17-lumina-v0.5a-thumbnail-cache-prefetch-design.md) · [план](docs/superpowers/plans/2026-06-17-lumina-v0.5a-thumbnail-cache-prefetch.md) · B: [дизайн](docs/superpowers/specs/2026-06-17-lumina-v0.5b-folder-watcher-design.md) · [план](docs/superpowers/plans/2026-06-17-lumina-v0.5b-folder-watcher.md) · C: [дизайн](docs/superpowers/specs/2026-06-17-lumina-v0.5c-trackpad-swipe-design.md) · [план](docs/superpowers/plans/2026-06-17-lumina-v0.5c-trackpad-swipe.md) · D: [дизайн](docs/superpowers/specs/2026-06-17-lumina-v0.5d-distribution-design.md) · [план](docs/superpowers/plans/2026-06-17-lumina-v0.5d-distribution.md) |
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

> **#2 (запись EXIF Orientation в оригиналы):** механизм записи тегов есть (v0.4b ч.2 — `write_edits`),
> правка `EXIF:Orientation` возможна вручную; отдельная «запечь поворот в файл»-кнопка пока не реализована.

## Прогресс v0.4b (детально)

Дизайн (две части): **часть 1 — чтение** (popup-просмотрщик), **часть 2 — запись** (редактирование/sidecar).
Логика — в чистом ядре (`ui::textedit`, `exif::tags`, `ui::{layout,hit,scene}`); обёртки без логики
(`exif::read` — subprocess exiftool); `app.rs` — тонкий диспетчер. Новых GPU-пайплайнов нет — popup
рисуется существующим UI-конвейером (только новые `DrawCmd`).

**Часть 1 — чтение (готово):**

- [x] `ui::textedit` — чистое ядро однострочного редактора (буфер/каретка/выделение, индексы в символах,
      Unicode-корректно); под поиск сейчас и редактирование тегов в части 2
- [x] `exif::tags` — модель `ExifTags`/`TagGroup` + `parse(json)` разбора `exiftool -json -G`
      (детерминированный порядок: группы по `GROUP_ORDER`, неизвестные — по алфавиту; теги внутри — по алфавиту)
- [x] `exif::read` — `read_tags(path)` через subprocess exiftool (`-json -G -struct --`, разделитель `--`
      защищает от подмены флагов через имя файла); `exiftool_path()` (рядом с exe / `assets/bin` / PATH-фолбэк);
      `LuminaError::Exif`
- [x] EXIF popup: тема (`POPUP_*` + затемнение), геометрия (`popup_layout` — центр-карточка заголовок/поиск/тело),
      хит-тест (`hit_popup` → Close/Search/Body/Outside), отрисовка (`build_popup` — карточка, поиск,
      сгруппированные строки ключ/значение, скролл-клип без GPU-scissor)
- [x] `exif::read_model` (kamadak-exif) — модель камеры в системном заголовке: `имя · Модель · Lumina`
      (без Model — `имя · Lumina`)
- [x] Открытие popup: клавиша `I` (`Action::ToggleExif`, по физ. клавише) **и** кнопка «i» bottom bar;
      иконки действий поворот/EXIF — яркие (активны), как fullscreen
- [x] Интеграция в `app.rs`: `UserEvent::ExifLoaded` + async-чтение на rayon (generation-guard от устаревших
      результатов), открытие/закрытие, ввод в поиск (печать/Backspace/Delete/стрелки/Home/End/Shift-выделение/Ctrl+A),
      скролл колесом, полный гейтинг ввода при открытом popup (клавиатура/клики/колесо перехватываются — шорткаты/
      зум/навигация/карусель не срабатывают), баннер ошибки при недоступном exiftool
- [x] UX-доводка по приёмке: клип текста по rect / строк тела по `body` (нет вылезания при скролле);
      заголовки групп — светлая плашка + яркий текст; поле поиска — модель фокуса (клик в поле/в тело),
      акцентная рамка фокуса, мигающая каретка (`ControlFlow::WaitUntil`, без 60fps) и подсветка выделения
      (позиция каретки/выделения — измерением шрифтом текстового слоя); симметричные зазоры заголовок/поиск/группа

Код и юнит-тесты готовы (131 тест зелёный + 6 `#[ignore]`; ~22 новых в части 1: textedit 9, exif::tags 4,
popup layout/hit/scene 7+, input 1; +1 `#[ignore]` интеграционный на реальном файле — нужен exiftool).
Визуальная GUI-приёмка пройдена (popup поверх фото, группировка/поиск/скролл, Model в заголовке, фокус/каретка,
гейтинг, баннер ошибки). exiftool у разработчика — Perl-дистрибутив через тонкий `exiftool.exe`-shim рядом с exe.

**Часть 2 — запись (готово):**

- [x] `exif::tags` — `is_editable`/`WRITABLE_GROUPS` (EXIF/XMP/IPTC/GPS записываемы; File/Composite/ExifTool/ICC —
      read-only), `TagEdit` (Set/Delete/DeleteAllGps) + `edits_to_args` (правки → аргументы exiftool)
- [x] `exif::write` — `write_edits(path, &[TagEdit])` через subprocess exiftool: редактируемые форматы in-place
      (exiftool оставляет `_original`), RAW → XMP sidecar (`-o %d%f.xmp`, оригинал не трогаем); `--` против подмены флагов
- [x] UI-ядро: футер popup (тело уменьшается; кнопки Save/Отменить всё — `popup_footer_buttons`); единый итератор
      видимых строк `scene::popup_rows` (общая геометрия для отрисовки/хита/диспетчера) + `popup_row_actions` (✎/✕);
      `hit_popup_edit` (футер/действия строк/GPS-delete-all/бар подтверждения); отрисовка ✎/✕ по hover, инлайн-редактора,
      маркеров pending (Set — акцент, Delete/GPS — зачёркнуто), бара «Несохранённые изменения» (Сохранить/Отменить/Продолжить)
- [x] Интеграция в `app.rs`: буфер правок (`exif_pending` (group,tag)→PendingOp + `exif_pending_delete_gps`),
      инлайн-редактор (✎ → правка, Enter коммит, Esc отмена; каретка мигает), удаление (✕ toggl'ит), «Удалить всё GPS»,
      Save → `write_edits` на rayon → `ExifSaved` → перечитывание тегов + обновление заголовка при смене Model,
      «Отменить всё», подтверждение закрытия при несохранённых правках (I/Esc/✕/клик-вне), hover-строка
- [x] Буфер обмена (`arboard`): Ctrl+C/V/X в поле поиска и в инлайн-редакторе

Код и юнит-тесты готовы (141 тест зелёный + 7 `#[ignore]`; ~10 новых в части 2: exif::tags 3, layout 1,
scene 4, hit 2; +1 `#[ignore]` интеграционный `write_real_jpg_set_and_backup` — записывает тег, проверяет
`_original` и обратное чтение, прогнан на exiftool 13.59 — зелёный). Запись EXIF Orientation в оригиналы
(пункт #2 из v0.4c) технически возможна правкой тега `EXIF:Orientation`, но как отдельная «запечь поворот»-кнопка
не реализована — остаётся отдельным пунктом.
**Бандл exiftool перенесён в v0.5** (вместе с инсталлятором): пока путь резолвится рядом с exe / в PATH (dev — shim).

> **Зависимости:** часть 1 — `serde_json` (разбор JSON exiftool); часть 2 — `arboard` (буфер обмена).
> Внешний `exiftool.exe` — пока dev-требование (рядом с exe или в PATH); бандл в ассеты — v0.5.
> Новых нативных зависимостей сборки часть 2 не вводит.

## Прогресс v0.4d (детально)

Дизайн (брейншторм): XMP sidecar для RAW из v0.4b **отвергнут осознанно** — он создавал `.xmp`
рядом с файлом, но правки не возвращались в сам RAW, поэтому при перечитывании терялись. Решение —
писать in-place во все форматы (exiftool это умеет и для RAW), а обратимость дать режимом бэкапа.
Логика — в чистом ядре (`exif::write` — чистые `edit_args`/`strip_args` + тонкая обёртка `run_exiftool`;
`ui::{theme,layout,scene,hit}` — без GPU); `app.rs` — диспетчер. Новых GPU-пайплайнов нет.

- [x] `exif::write` — убрана ветка sidecar (`-o %d%f.xmp`); `WriteMode { Backup, Overwrite }`,
      чистые `edit_args(edits, mode)` (пустой список → пустой вектор; `Overwrite` добавляет
      `-overwrite_original`) и `strip_args()` (`-all=` + `-tagsfromfile @ -orientation -icc_profile
      -overwrite_original` — стирает всё, сохраняя Orientation/ICC); `write_edits(path, edits, mode)`
      и `strip_all(path)` поверх единой обёртки `run_exiftool` (с `--` против подмены флагов)
- [x] UI-ядро: `theme.danger_bg`; геометрия футера `popup_footer_toggle`/`popup_footer_strip`;
      `ConfirmKind { None, CloseWithPending, OverwriteSave, StripAll }` вместо булева `confirm_close`;
      `PopupEditState.confirm`/`overwrite_mode`; отрисовка тоггла «Необратимо» (danger-стиль во вкл.),
      кнопки «Стереть всё» (только в необратимом режиме) и трёх вариантов бара подтверждения;
      хит-тест `FooterToggle`/`FooterStrip` + обобщённые `ConfirmPrimary/Secondary/Tertiary`
- [x] Интеграция в `app.rs`: состояние `exif_confirm`/`exif_overwrite_mode`/`exif_close_after_save`;
      `exif_save(mode)` + `exif_strip_all` на rayon; тоггл режима;
      `FooterSave` в необратимом режиме при наличии правок → бар «Перезаписать», иначе обычный Backup;
      цепочка закрытия-с-правками → подтверждение перезаписи; сброс необратимого режима после успешного
      сохранения (следующее разрушительное действие — снова осознанное); Esc-приоритет редактор → бар → закрытие
- [x] Фикс read-back правок EXIF на форматах с IFD1-дублями (Fuji зеркалит Artist в thumbnail-IFD):
      `edits_to_args` очищает `-IFD1:{tag}=` при записи EXIF-тега — иначе exiftool `-json -G` отдавал
      устаревший IFD1, и правка IFD0 была не видна ([дизайн](docs/superpowers/specs/2026-06-17-lumina-exif-ifd1-duplicate-clear-design.md))

Код и юнит-тесты готовы (154 теста зелёных + 10 `#[ignore]`; 13 новых в v0.4d: write 4, tags 3, layout 1,
scene 3, hit 2; +3 новых `#[ignore]` интеграционных — `write_overwrite_no_backup`, `strip_all_removes_pii_keeps_orientation`,
`write_clears_ifd1_artist_duplicate`, прогнаны на exiftool 13.59 — зелёные). Визуальную GUI-приёмку (правка RAF in-place без `.xmp`, режимы
бэкап/необратимо, «Стереть всё», Esc на баре) подтверждает пользователь вручную.

> **Бандл exiftool — v0.5.**

## Прогресс v0.5a (детально)

Дизайн: два независимых кэш-слоя «чистое ядро + тонкая обёртка», декод остаётся на rayon.

- [x] Дисковый кэш миниатюр (`src/thumbcache.rs`): ключ FNV-1a (стабилен между сборками) по
      `path+mtime+size+th`, PNG на диске в `%LOCALAPPDATA%\Lumina\thumbs`, эвикция по бюджету 256 МБ
      (`prune` фоном при старте). Консультируется в rayon-воркере декода миниатюр ДО тяжёлого декода;
      переоткрытие папки — миниатюры из кэша, без повторного декода
- [x] Префетч ±2 (`src/prefetch.rs`): in-memory LRU декодированных кадров с байтовым бюджетом 512 МБ
      (сам ограничивается на тяжёлых RAW); заполняется при завершении `Full`, используется
      `load_current` (быстрый путь `show_image`) для мгновенного перелистывания; guard
      `prefetch_inflight` против повторных декодов соседей
      ([дизайн](docs/superpowers/specs/2026-06-17-lumina-v0.5a-thumbnail-cache-prefetch-design.md))

Код и юнит-тесты готовы (165 тестов зелёных + 10 `#[ignore]`; 11 новых в v0.5a: thumbcache 7,
prefetch 4). Визуальную приёмку (кэш переживает переоткрытие папки; мгновенная навигация по
посещённым соседям; память не растёт неограниченно) подтверждает пользователь вручную.

## Установленное окружение

- 🟢 Rust 1.96.0 stable (`x86_64-pc-windows-msvc`)
- 🟢 MSVC Build Tools 14.51 + Windows SDK 10.0.26100
- 🟢 Git, winget
- 🟢 LLVM/Clang 22.1.7 (`libclang`) — bindgen для libheif-rs
- 🟢 CMake 4.3.3
- 🟢 vcpkg + libheif 1.23.0 (libde265/x265) — HEIC. Линковка проверена (`cargo run --bin smoke_libheif`)
- 🟡 ExifTool — используется в v0.4b (чтение тегов в popup — ч.1; запись правок — ч.2); резолвится рядом с exe / в PATH (dev — shim `exiftool.exe` 13.59). Бандл в ассеты — v0.5.
  Пока dev-требование: `exiftool.exe` рядом с exe или в PATH

> **Окружение сборки/запуска v0.2** (user-env уже выставлены через `setx`):
> `VCPKG_ROOT=~/vcpkg`, `LIBCLANG_PATH=C:\Program Files\LLVM\bin`, `VCPKGRS_DYNAMIC=1`.
> Для запуска приложения каталог `~/vcpkg/installed/x64-windows/bin` (heif.dll/libde265.dll)
> должен быть в `PATH`, либо DLL копируются рядом с exe.
