# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Lumina — просмотрщик фотографий для Windows на Rust + wgpu. Полная спецификация — `SPEC_lumina.md`,
дорожная карта по фазам — `ROADMAP.md`. Документация и общение по проекту ведутся на русском.

## Команды

```bash
cargo run                       # запустить viewer (системный диалог выбора файла)
cargo run -- "путь\к\фото.jpg"  # открыть конкретный файл (CLI-арг имеет приоритет над диалогом)
cargo build                     # сборка
cargo test                      # юнит-тесты (RAW/HEIC-тесты помечены #[ignore] — нужны реальные файлы)
cargo test heic_decodes -- --ignored   # запустить конкретный ignore-тест на реальном образце
cargo run --bin make_fixtures   # перегенерировать tests/fixtures/red_2x3.{png,jpg}
cargo run --bin smoke_libheif   # проверить линковку libheif
```

`default-run = "lumina"` в `Cargo.toml` — поэтому `cargo run` запускает viewer, а не `make_fixtures`.
Профиль `dev` собирается с `opt-level = 1` (а зависимости — `3`), иначе image-декод в debug мучительно медленный.

### Окружение сборки (Windows / MSVC)

HEIC через `libheif-rs` требует нативных зависимостей. User-env переменные (уже выставлены через `setx`):
`VCPKG_ROOT=~/vcpkg`, `LIBCLANG_PATH=C:\Program Files\LLVM\bin` (bindgen), `VCPKGRS_DYNAMIC=1`.
Для **запуска** каталог `~/vcpkg/installed/x64-windows/bin` (`heif.dll`/`libde265.dll`) должен быть в `PATH`
либо DLL лежать рядом с exe.

## Архитектура

Один бинарь, без потоков-воркеров вручную: всё крутится вокруг winit event loop в `app.rs`.

### Декодеры (`src/decoder/`)
Трейт `Decoder` с тремя методами: `supports(ext)`, `decode_preview` (быстрое встроенное превью или `None`),
`decode_full`. Три реализации:
- **`StandardDecoder`** — JPEG/PNG/BMP/GIF/TIFF/WebP через крейт `image`. Превью нет (одна стадия).
- **`HeicDecoder`** — HEIC/HEIF через `libheif-rs` (FFI). Одна стадия; копирует с учётом stride в плотный RGBA8.
- **`RawDecoder`** — RAW (RAF/NEF/ARW/CR2/DNG/…) через `rawler`. Двухстадийный: превью = встроенный JPEG камеры
  (мгновенно, цветоточно), full = полный develop.

`decoder_for(ext)` — роутер с приоритетом Raw → Heic → Standard. Все декодеры возвращают `DecodedImage`
(плотный `rgba: Vec<u8>` + размеры).

**RAW и цвет (важный нюанс, см. `decode_full` в `raw.rs`):** для не-байеровских CFA (Fuji X-Trans, паттерн крупнее 2×2)
rawler 0.7.2 демозаит некорректно → зеленца, поэтому для них используется встроенный JPEG камеры; для байеровских —
полноценный `RawDevelop`. Тип сенсора определяется дешёвым зондом `raw_image(.., dummy=true)`.
Известное ограничение: Lightroom HDR DNG (float JPEG XL) rawler не декодирует — показывается только встроенное превью.

### Конвейер декодирования (`app.rs`)
`load_current()` спавнит декод на `rayon::spawn`; результаты возвращаются в event loop через
`EventLoopProxy::send_event(UserEvent::Decoded { generation, stage, result })`.
- **Generation counter** (`state.generation`): инкрементится на каждую загрузку; результаты с устаревшим
  `generation` игнорируются в `user_event` — так быстрая навигация не показывает старые кадры.
- **Две стадии:** сначала прилетает `Preview` (если есть), потом `Full`. При первом кадре генерации вид
  инициализируется (fit), при подмене preview→full `rescale_for_new_image` сохраняет экранный размер.

### Рендер (`src/renderer/`)
- **`GpuContext`** (`context.rs`) — wgpu surface/device/queue/config. Выбирает sRGB-формат поверхности,
  `PresentMode::Mailbox` при наличии (иначе Fifo), `desired_maximum_frame_latency = 1` — минимум лага ввода.
- **`BlitPipeline`** (`pipeline.rs`) — рисует одну текстуру в открытый pass (`draw`) с GPU-viewport'ом
  viewer-региона (triangle-strip из 4 вершин), шейдер `assets/shaders/blit.wgsl`. Текстура грузится как
  `Rgba8UnormSrgb`. Трансформ вида передаётся uniform-матрицей `Mat4`.

### UI (`src/ui/` + `src/renderer/ui_pipeline.rs`, `text.rs`)
Чистое ядро `src/ui/`: `theme` (палитра/размеры, srgb→linear), `layout` (раскладка
titlebar/кнопок/viewer в физ. px), `hit` (курсор→регион: кнопки/caption/края resize),
`scene` (UiState → список `DrawCmd`). Всё юнит-тестируется, без GPU.
Рендер: `UiPipeline` — инстансовый SDF rounded-rect (`assets/shaders/ui.wgsl`);
`TextLayer` — обёртка glyphon (текст + глифы кнопок из Segoe MDL2 Assets).
`Renderer::render` рисует фото в viewer-viewport (под titlebar), затем UI, затем текст.
Гард пропускает блит фото, если окно ниже titlebar (вырожденный размер) — иначе wgpu падает на set_viewport.

**v0.3b — bottom bar:** `ui::layout` считает зоны divider/мета/карусель/кнопки (`compute(win, scale,
bottom_factor, fullscreen)`); `ui::hit` — регионы bottom bar + `hit_thumbnail`; `ui::scene` эмитит хром
bottom bar и `FileMeta`/`meta_lines`. Миниатюры: `ThumbnailStore` (`src/thumbnail.rs` — окно запроса/LRU/
поколение/cover-кроп, юнит-тесты) + `ThumbnailLayer` (`src/renderer/thumbnail.rs` — текстура на миниатюру,
SDF-скругление, `assets/shaders/thumb.wgsl`). Декод миниатюр — `decode_preview` иначе `decode_full` →
ресайз через `image`, на rayon, лениво in-memory (sled-кэш — v0.5). Иконки действий — Tabler
(`assets/fonts/tabler-icons.ttf`); кнопки окна — Segoe MDL2. Fullscreen — `winit` borderless; нативный
флаг отключает caption/resize в `WM_NCHITTEST`.

### Платформа (`src/platform/windows.rs`)
Нативный frameless: субклассинг wndproc, `WM_NCCALCSIZE` убирает caption (сохраняя
WS_THICKFRAME → resize/Aero Snap/тень), `WM_NCHITTEST` мапит регионы из `ui::hit` в HT-коды
(caption-drag, края resize). Кнопки окна обрабатываются в client area через winit.
`set_scale` передаёт scale_factor в wndproc (нет доступа к winit-состоянию из FFI).

### Вид (`src/view.rs`)
`ViewTransform` — вся математика zoom/pan/fit и анимации, **без зависимостей от GPU или winit** (потому хорошо
покрыт юнит-тестами). `matrix(win, img)` строит ortho-проекцию с центрированием, pan и zoom.
`zoom_at` сохраняет точку под курсором; `clamp_pan` не даёт полей по краям; `min_zoom` = fit-zoom (фото не
делается меньше окна). Поля `rotation/flip_*` — задел под v0.4. Анимация zoom — ease-out-cubic, 0.15 с.

### Каталог (`src/catalog/`)
`FolderCatalog` — список поддерживаемых файлов папки открытого файла, натуральная сортировка (`natord`),
навигация prev/next/first/last. Фильтрует по `decoder::supported(ext)`.

### Ввод (`src/input/`)
Чистые функции `on_wheel`/`on_nav_key`, возвращающие `InputOutcome { redraw, navigate }` — тоже без сайд-эффектов,
тестируемо. Сам диспетчинг событий (drag, double-click → fit/100%, Ctrl+0/Ctrl+1, стрелки) — в `app.rs::window_event`.

### Ошибки (`src/error.rs`)
`LuminaError` (thiserror) + `Result<T>`. Декодеры мапят строковые ошибки сторонних крейтов в варианты `Raw`/`Heic`.

## Конвенции

- Стиль слоёв: «чистое ядро» (`view`, `input`, `catalog`, трейт `Decoder`) тестируется юнитами; GPU/winit/FFI —
  тонкие обёртки без логики. Сохраняй это разделение при добавлении фич.
- Комментарии в коде — на русском, как и весь проект.
- Дизайны фаз — `docs/superpowers/specs/`, планы реализации — `docs/superpowers/plans/`. Перед крупной фичей
  сверяйся со `SPEC_lumina.md` и обновляй `ROADMAP.md`.
