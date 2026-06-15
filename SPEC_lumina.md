# Lumina — спецификация v0.2

Просмотрщик фотографий для Windows, написанный на Rust с GPU-рендерингом через wgpu.

---

## 1. Обзор и цели

| Атрибут | Значение |
|---|---|
| Язык | Rust (edition 2021) |
| Рендеринг | wgpu (DirectX 12 backend на Windows) |
| Windowing | winit |
| Цель | Замена встроенного просмотрщика Windows 11 с поддержкой RAW из коробки |
| Платформа v1 | Windows 10/11 x64 |

Ключевые принципы: нулевые зависимости от Windows Imaging Component, всё декодирование — в процессе через нативные Rust-крейты, холодный старт < 300 мс.

---

## 2. Архитектура

```
lumina/
├── src/
│   ├── main.rs               # точка входа, winit event loop
│   ├── app.rs                # App state, главный update/render цикл
│   ├── renderer/
│   │   ├── mod.rs
│   │   ├── context.rs        # wgpu Device, Queue, Surface
│   │   ├── texture.rs        # загрузка текстур, mipmap
│   │   ├── pipeline.rs       # render pipeline (quad + blit shader)
│   │   └── ui.rs             # примитивы UI (панели, карусель, иконки)
│   ├── decoder/
│   │   ├── mod.rs            # trait Decoder + роутер по расширению
│   │   ├── standard.rs       # JPEG/PNG/WebP/TIFF/BMP/GIF через image-rs
│   │   ├── raw.rs            # RAW через rawler или libraw-sys
│   │   ├── heic.rs           # HEIC через libheif-rs
│   │   └── formats.rs        # enum Format, список поддерживаемых расширений
│   ├── catalog/
│   │   ├── mod.rs
│   │   ├── folder.rs         # сканирование папки, сортировка, watcher
│   │   └── thumbnail.rs      # генерация/кэш миниатюр (sled или sqlite)
│   ├── exif/
│   │   ├── mod.rs
│   │   ├── reader.rs         # чтение через kamadak-exif
│   │   └── writer.rs         # запись/удаление тегов через little-exiftool FFI
│   ├── input/
│   │   ├── mod.rs
│   │   ├── keyboard.rs
│   │   ├── mouse.rs          # zoom, pan, drag
│   │   └── touch.rs          # свайп (трекпад/тач через winit Touch events)
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs         # расчёт layout (titlebar, viewer, divider, bottom bar)
│   │   ├── carousel.rs       # состояние карусели, scroll, выбор
│   │   ├── exif_popup.rs     # popup редактора EXIF
│   │   ├── slideshow.rs      # оверлейный тулбар slideshow
│   │   └── theme.rs          # ThemePalette, resolve system theme
│   └── config.rs             # чтение/запись config.toml (serde + toml)
└── assets/
    ├── shaders/
    │   ├── blit.wgsl         # финальный blit текстуры на экран
    │   └── ui.wgsl           # рендер UI-примитивов
    └── icons/                # SVG → растеризованные PNG при сборке
```

---

## 3. Поддерживаемые форматы

### 3.1 Стандартные

| Формат | Крейт | Чтение | Запись |
|---|---|---|---|
| JPEG / JFIF | `image` | ✓ | — |
| PNG | `image` | ✓ | — |
| WebP | `image` (feature webp) | ✓ | — |
| TIFF | `image` | ✓ | — |
| BMP | `image` | ✓ | — |
| GIF (статик) | `image` | ✓ | — |
| AVIF | `image` (feature avif) | ✓ | — |
| HEIC / HEIF | `libheif-rs` | ✓ | — |

### 3.2 RAW камер

Приоритет — декодировать embedded JPEG preview для быстрого показа, затем async декодировать полный RAW.

| Камера | Формат | Крейт |
|---|---|---|
| Fujifilm | RAF | `rawler` |
| Canon | CR2, CR3 | `rawler` / `libraw-sys` |
| Nikon | NEF, NRW | `rawler` |
| Sony | ARW, SRF | `rawler` |
| Panasonic | RW2 | `rawler` |
| Olympus/OM | ORF | `rawler` |
| Pentax | PEF, DNG | `rawler` |
| Adobe | DNG | `rawler` |
| Leica | RWL, DNG | `rawler` |
| Phase One | IIQ | `libraw-sys` (fallback) |

Крейт `rawler` — предпочтительный (pure Rust). `libraw-sys` — FFI-обёртка над LibRaw как fallback для экзотики.

---

## 4. UI и компоненты

### 4.1 Titlebar

```
[●][●][●]   DSC_4821.RAF · Fujifilm X-T5 · Lumina   [пусто]
```

- Кастомный (frameless window), нативные кнопки управления окном отрисовываются вручную
- По центру: `имя_файла · модель_камеры_из_EXIF · Lumina`
- Поддержка drag окна за titlebar

### 4.2 Viewer (главная область)

- Фон: `#111113` (тёмный нейтральный, не чистый чёрный)
- Фотография рендерится как textured quad через wgpu
- Поведение масштаба:

| Действие | Результат |
|---|---|
| Колёсико вверх/вниз | Zoom in/out с центром под курсором |
| Double-click | Переключение fit-to-window ↔ 100% |
| Ctrl + 0 | Fit to window |
| Ctrl + 1 | 100% (1:1) |
| Drag (при zoom > fit) | Pan; курсор меняется на grab/grabbing |

- Zoom range: 5% – 3200%
- Плавная анимация zoom (easing, ~150 мс)
- Стрелки навигации по краям (появляются при hover, скрываются через 2 с)

### 4.3 Divider bar

Горизонтальная полоска между viewer и bottom bar:

```
карусель  [════]  ⌄
```

- Клик по ней скрывает/показывает bottom bar (анимация slide, ~200 мс)
- Иконка шеврона вращается при скрытии
- Слева — метка "карусель" (fadout при скрытии)

### 4.4 Bottom bar

Состоит из трёх зон:

#### Зона слева — мета-информация файла

Ширина ~110 px, выровнено по левому краю карусели.

```
ФОРМАТ
RAF · RAW

РАЗМЕР
40.2 MP
7728 × 5200
38.4 МБ
```

Обновляется при смене фото. Все данные — из заголовка файла, не из EXIF.

#### Зона центр — карусель миниатюр

- Горизонтальный скролл, скрыт scrollbar
- Миниатюры: 62 × 64 px, border-radius 4px
- Активная миниатюра: белая рамка 1.5px
- Бейдж формата (RAF, CR2 и т.д.) в правом нижнем углу для RAW-файлов
- Скролл мышью по карусели — прокрутка карусели (не zoom)
- Свайп трекпадом — прокрутка карусели
- Клик — открыть фото
- Ленивая загрузка миниатюр: сначала embedded preview из RAW, потом полный рендер
- Кэш миниатюр: sled (embedded key-value DB), ключ = `path + mtime`

#### Зона справа — кнопки действий

Ширина ~76 px, две иконки:

| Иконка | Действие |
|---|---|
| `ti-maximize` | Fullscreen / выход из fullscreen (F или F11) |
| `ti-info-circle` | Открыть EXIF popup |

В fullscreen дополнительно появляется тулбар slideshow (см. §13). Кнопка запуска slideshow — `ti-player-play` — в этом тулбаре, не в bottom bar.

### 4.5 EXIF Popup

Открывается поверх viewer (не модальное окно ОС, а wgpu-rendered overlay).

```
┌─────────────────────────────────────────┐
│ EXIF — DSC_4821.RAF                [✕] │
├─────────────────────────────────────────┤
│ Make                    Fujifilm    [✎] │
│ Model                   X-T5        [✎] │
│ ExposureTime            1/500       [✎] │
│ FNumber                 f/4.0       [✎] │
│ ISO                     400         [✎] │
│ DateTimeOriginal        2024-...    [✎] │
│ GPS                     —           [✎] │
│  ...                                    │
├─────────────────────────────────────────┤
│ [Удалить всё GPS]          [Сохранить]  │
└─────────────────────────────────────────┘
```

- Редактирование тегов инлайн (клик на иконку карандаша)
- Удаление отдельного тега (кнопка `[✕]` при hover)
- Кнопка "Удалить всё GPS" — batch-удаление GPS-тегов
- Запись изменений: перезапись файла с обновлённым EXIF (только для JPEG/TIFF; для RAW — отдельный sidecar XMP)
- Изменения не затрагивают пиксельные данные (non-destructive для EXIF)

---

## 5. Навигация — полный список

| Действие | Результат |
|---|---|
| `→` / `←` | Следующее / предыдущее фото |
| `Home` / `End` | Первое / последнее в папке |
| Клик на миниатюру | Открыть фото |
| Свайп горизонталь (трекпад) | Следующее / предыдущее фото |
| Свайп по карусели (трекпад) | Прокрутка карусели |
| `F` или `F11` | Fullscreen |
| `Esc` | Выход из fullscreen / закрыть popup |
| `R` | Поворот по часовой стрелке (90°) |
| `Shift+R` | Поворот против часовой стрелки (90°) |
| `H` | Горизонтальное отражение |
| `V` | Вертикальное отражение |
| `I` | Открыть/закрыть EXIF popup |
| `Ctrl+C` | Скопировать фото в буфер |
| `Delete` | Удалить файл (с подтверждением) |
| `Ctrl+Z` | Отменить поворот/отражение |
| `Space` | Пауза / возобновить slideshow (только в fullscreen slideshow) |

---

## 6. Поворот и отражение (non-destructive)

- Трансформации хранятся в памяти как `Transform { rotation: u8, flip_h: bool, flip_v: bool }`
- Для JPEG применяется lossless JFIF rotation (через `mozjpeg` или jpegtran FFI)
- Для RAW — трансформация пишется в sidecar `.xmp`
- EXIF-тег `Orientation` обновляется соответственно
- Горячие клавиши и кнопки в titlebar (TBD в v1.1)

---

## 7. Производительность и загрузка

### Стратегия загрузки

```
1. Открытие файла
   └─► Быстро: декодировать embedded JPEG preview (есть в большинстве RAW)
       └─► Показать в viewer немедленно
   └─► Async: полное декодирование RAW в отдельном потоке (tokio или rayon)
       └─► Заменить текстуру по готовности, без перерисовки UI

2. Предзагрузка
   └─► Декодировать следующие 2 и предыдущие 2 фото в фоне (LRU кэш)

3. Миниатюры
   └─► При первом открытии папки: параллельная генерация (rayon thread pool)
   └─► Кэш: sled по ключу (абс. путь + mtime)
```

### Целевые показатели

| Метрика | Цель |
|---|---|
| Холодный старт до первого фото | < 300 мс |
| Переключение между JPEG | < 50 мс |
| Показ preview RAW | < 200 мс |
| Полный RAW decode (X-T5 40 MP) | < 2 с (фон) |
| Использование RAM в покое | < 150 МБ |

---

## 8. Зависимости (Cargo.toml)

```toml
[dependencies]
winit          = "0.30"
wgpu           = { version = "22", features = ["dx12"] }
image          = { version = "0.25", features = ["webp", "avif", "tiff"] }
rawler         = "0.6"               # pure-Rust RAW decoder
libheif-rs     = "0.18"             # HEIC/HEIF
kamadak-exif   = "0.5"             # EXIF чтение
little-exiftool = "0.2"            # EXIF запись (FFI к ExifTool)
sled           = "0.34"            # thumbnail cache
tokio          = { version = "1", features = ["rt-multi-thread"] }
rayon          = "1.10"
bytemuck       = "1.16"            # wgpu buffer casting
glam           = "0.28"            # матрицы трансформаций
notify         = "6"               # folder watcher
image-rs-exif  = { optional = true } 

natord         = "1.0"             # натуральная сортировка файлов
serde          = { version = "1", features = ["derive"] }
toml           = "0.8"             # config.toml

[build-dependencies]
resvg          = "0.43"            # SVG → PNG для иконок при сборке
```

> `libraw-sys` добавить как optional fallback feature `raw-libraw` для форматов, не поддерживаемых rawler.

---

## 9. Shader (blit.wgsl)

Минимальный шейдер для отрисовки фото:

```wgsl
// blit.wgsl
struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOut {
    var quad = array<vec2<f32>, 4>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0),
        vec2(-1.0,  1.0), vec2(1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 4>(
        vec2(0.0, 1.0), vec2(1.0, 1.0),
        vec2(0.0, 0.0), vec2(1.0, 0.0),
    );
    var out: VertexOut;
    out.pos = transform * vec4(quad[vid], 0.0, 1.0);
    out.uv = uvs[vid];
    return out;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> transform: mat4x4<f32>;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
```

Матрица `transform` кодирует pan + zoom + rotation.

---

## 10. App state (псевдокод)

```rust
struct AppState {
    // каталог
    folder: FolderCatalog,      // список файлов в папке, сортировка

    // viewer
    view_transform: ViewTransform,  // zoom, pan, rotation
    texture_current: Option<wgpu::Texture>,
    texture_prefetch: LruCache<usize, wgpu::Texture>,

    // UI
    carousel_visible: bool,
    carousel_scroll: f32,
    exif_popup_open: bool,
    exif_data: Option<ExifMap>,
    fullscreen: bool,

    // slideshow
    slideshow: SlideshowState,

    // тема
    theme: ActiveTheme,     // Dark | Light (уже resolved из System)
    palette: ThemePalette,

    // async
    decode_task: Option<JoinHandle<DecodedImage>>,
}

struct ViewTransform {
    zoom: f32,          // 0.05 .. 32.0
    pan: Vec2,          // смещение в нормализованных координатах
    rotation: u8,       // 0, 90, 180, 270
    flip_h: bool,
    flip_v: bool,
}
```

---

## 11. Регистрация как просмотрщик по умолчанию

При установке (NSIS или WiX installer):

- Регистрация в реестре `HKCU\Software\Classes` для всех поддерживаемых расширений
- ProgID: `Lumina.PhotoViewer`
- Команда запуска: `lumina.exe "%1"`
- Иконка: `lumina.exe,0`
- Прописывается в `OpenWithProgids` для каждого расширения
- Появляется в "Выбрать приложение по умолчанию" в Windows 11

---

## 12. Фазы разработки

### v0.1 — Viewer core
- [ ] winit окно + wgpu context
- [ ] Загрузка и показ JPEG/PNG
- [ ] Zoom (колёсико) и pan (drag)
- [ ] Double-click fit/100%
- [ ] Навигация стрелками

### v0.2 — RAW и форматы
- [ ] rawler интеграция (RAF, NEF, ARW, CR2)
- [ ] HEIC через libheif-rs
- [ ] Embedded preview для мгновенного показа
- [ ] Async полное декодирование RAW

### v0.3 — UI
- [ ] Titlebar с именем файла и камерой
- [ ] Карусель миниатюр
- [ ] Divider toggle (скрыть/показать карусель)
- [ ] Мета-панель слева (формат, разрешение, размер)
- [ ] Кнопки fullscreen и EXIF справа

### v0.4 — EXIF и трансформации
- [ ] Чтение EXIF (kamadak-exif)
- [ ] EXIF popup с редактированием
- [ ] Запись EXIF / XMP sidecar для RAW
- [ ] Поворот и отражение (non-destructive)

### v0.5 — Полировка
- [ ] Кэш миниатюр (sled)
- [ ] Предзагрузка ±2 фото
- [ ] Folder watcher (notify)
- [ ] Свайп трекпадом (winit Touch events)
- [ ] Installer + регистрация в реестре

### v0.6 — Slideshow, темизация, multi-monitor
- [ ] Slideshow режим (см. §13)
- [ ] Темизация (см. §14)
- [ ] Multi-monitor fullscreen (см. §15)
- [ ] Сортировка каталога (см. §16)

---

## 13. Slideshow

Slideshow доступен **только в полноэкранном режиме**. Активируется кнопкой в оверлейном тулбаре полноэкранного режима (появляется при движении мыши, скрывается через 3 с).

### UI в fullscreen

```
╔══════════════════════════════════════════════════════╗
║                                                      ║
║                     [фото]                           ║
║                                                      ║
║  ─────────────────────────────────────────────────  ║
║  ◀  ▶  ⏸  🔀   ════════●══════  ×1  [00:05]  [✕]  ║
╚══════════════════════════════════════════════════════╝
```

Тулбар прижат к низу fullscreen-окна. Элементы слева направо:

| Элемент | Описание |
|---|---|
| `◀` / `▶` | Предыдущее / следующее вручную |
| `⏸` / `▶` | Пауза / возобновить slideshow |
| `🔀` | Случайный порядок on/off |
| Прогресс-бар | Таймер текущего слайда; кликабелен (перемотка) |
| `×1` | Скорость: ×0.5 / ×1 / ×2 / ×3; цикличный клик |
| `[00:05]` | Интервал между слайдами; клик открывает picker (3 / 5 / 10 / 15 / 30 с) |
| `[✕]` | Выйти из slideshow (остаться в fullscreen) |

### Поведение

- При запуске slideshow карусель и divider скрываются
- Переход между фото — анимация fade (200 мс), не слайд, чтобы не путать с ручной навигацией
- `Esc` — выход из fullscreen (slideshow при этом останавливается)
- `Space` — пауза/возобновить
- `→` / `←` — вручную вперёд/назад (пауза не снимается)
- Случайный порядок: Fisher-Yates shuffle по текущему списку каталога; повторы исключены до полного прохода
- Интервал и скорость сохраняются в настройках приложения (см. §14)

### State

```rust
struct SlideshowState {
    active: bool,
    paused: bool,
    shuffle: bool,
    interval_secs: f32,       // 3.0 / 5.0 / 10.0 / 15.0 / 30.0
    speed_multiplier: f32,    // 0.5 / 1.0 / 2.0 / 3.0
    elapsed: f32,             // секунд с последней смены
    shuffle_queue: Vec<usize>,
    shuffle_pos: usize,
}
```

---

## 14. Темизация

### Режимы

| Режим | Описание |
|---|---|
| `System` | Следует теме Windows (светлая/тёмная); **по умолчанию** |
| `Dark` | Всегда тёмная |
| `Light` | Всегда светлая |

Переключение: через меню настроек (кнопка `⚙` в titlebar, TBD в v0.6). Выбор сохраняется в конфиге.

### Определение системной темы

```rust
// winit WindowEvent::ThemeChanged или начальный запрос
fn get_system_theme(window: &winit::window::Window) -> Theme {
    match window.theme() {
        Some(winit::window::Theme::Dark) => Theme::Dark,
        _ => Theme::Light,
    }
}
```

При изменении системной темы во время работы (`WindowEvent::ThemeChanged`) — немедленный пересчёт палитры без перезапуска.

### Цветовые токены

Все цвета в шейдерах и UI-примитивах параметризованы через `ThemePalette`. Никаких хардкоженных hex в коде рендерера.

```rust
struct ThemePalette {
    bg_primary:     [f32; 4],  // фон viewer
    bg_surface:     [f32; 4],  // titlebar, bottom bar
    bg_elevated:    [f32; 4],  // popup, карточки
    text_primary:   [f32; 4],
    text_secondary: [f32; 4],
    text_muted:     [f32; 4],
    border:         [f32; 4],
    accent:         [f32; 4],  // активная миниатюра, прогресс-бар
    thumb_active_border: [f32; 4],
}

impl ThemePalette {
    fn dark() -> Self { /* ... */ }
    fn light() -> Self { /* ... */ }
}
```

### Светлая тема — ключевые значения

| Токен | Hex |
|---|---|
| `bg_primary` | `#F2F2F2` |
| `bg_surface` | `#FFFFFF` |
| `bg_elevated` | `#FAFAFA` |
| `text_primary` | `#1A1A1A` |
| `text_secondary` | `#5A5A5A` |
| `border` | `rgba(0,0,0,0.1)` |
| `accent` | `#0078D4` (Windows синий) |

### Тёмная тема — ключевые значения

| Токен | Hex |
|---|---|
| `bg_primary` | `#111113` |
| `bg_surface` | `#1C1C1E` |
| `bg_elevated` | `#232325` |
| `text_primary` | `rgba(255,255,255,0.88)` |
| `text_secondary` | `rgba(255,255,255,0.50)` |
| `border` | `rgba(255,255,255,0.08)` |
| `accent` | `#4FC3F7` |

### Иконки

Иконки растеризуются при сборке в двух вариантах (`icons/dark/` и `icons/light/`). Выбор варианта — по активной теме.

### Конфиг

```toml
# %APPDATA%\Lumina\config.toml
[appearance]
theme = "system"   # "system" | "dark" | "light"

[slideshow]
interval_secs = 5.0
speed_multiplier = 1.0
shuffle = false
```

---

## 15. Поддержка нескольких мониторов

### Fullscreen

Fullscreen активируется **на том мониторе, на котором находится окно** в момент нажатия `F` / `F11` / кнопки.

```rust
fn enter_fullscreen(window: &winit::window::Window) {
    // winit автоматически использует текущий монитор окна
    window.set_fullscreen(Some(Fullscreen::Borderless(
        window.current_monitor()
    )));
}
```

При перетаскивании окна на другой монитор и повторном входе в fullscreen — используется новый монитор.

### DPI и масштабирование

- Все размеры UI в логических пикселях; физические пиксели = `logical * scale_factor`
- `WindowEvent::ScaleFactorChanged` — пересчитать layout и перегенерировать UI-текстуры
- Миниатюры в кэше хранятся в физических пикселях под конкретный scale factor; при изменении DPI — инвалидация кэша для текущей папки

### Конфигурация окна при запуске

Позиция и размер окна сохраняются в конфиге (в логических пикселях). При запуске — восстановление на том же мониторе если он доступен, иначе — primary monitor.

```toml
[window]
x = 200
y = 100
width = 1280
height = 800
monitor_name = "DELL U2723D"   # display name из winit
maximized = false
```

---

## 16. Сортировка каталога

### Принцип

Порядок файлов в карусели и при навигации стрелками **соответствует порядку сортировки папки**, в которой открыт файл. По умолчанию — по имени файла (натуральная сортировка, как в Проводнике Windows).

### Доступные режимы сортировки

| Режим | Ключ | Описание |
|---|---|---|
| По имени ↑ | `name_asc` | Натуральная сортировка A→Z, 1→10 (**по умолчанию**) |
| По имени ↓ | `name_desc` | Обратная |
| По дате съёмки ↑ | `exif_date_asc` | `DateTimeOriginal` из EXIF; файлы без EXIF — в конец |
| По дате съёмки ↓ | `exif_date_desc` | Обратная |
| По дате изменения ↑ | `mtime_asc` | `mtime` из файловой системы |
| По дате изменения ↓ | `mtime_desc` | Обратная |
| По размеру файла ↑ | `size_asc` | Байты |
| По размеру файла ↓ | `size_desc` | Обратная |

### Натуральная сортировка по имени

Реализуется вручную или через крейт `natord`. Пример: `IMG_2.jpg < IMG_10.jpg < IMG_20.jpg` (в отличие от лексикографической, где `IMG_10 < IMG_2`).

### UI переключения

Переключатель сортировки — в контекстном меню правого клика на карусели (v0.6). В тулбаре не выносится, чтобы не загромождать.

### Состояние

```rust
#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    NameAsc, NameDesc,
    ExifDateAsc, ExifDateDesc,
    MtimeAsc, MtimeDesc,
    SizeAsc, SizeDesc,
}

struct FolderCatalog {
    all_files: Vec<FileEntry>,   // исходный список из FS
    sorted: Vec<usize>,          // индексы в all_files, в текущем порядке
    sort_mode: SortMode,
    current_sorted_pos: usize,   // позиция в sorted[]
}
```

При смене режима сортировки — пересортировать `sorted`, найти новую позицию текущего файла по пути, обновить карусель без перезагрузки фото.

### Сохранение

Последний выбранный режим сортировки сохраняется в конфиге как глобальный (не per-folder).

```toml
[catalog]
sort_mode = "name_asc"
```

---

## 17. Видео (TODO)

> **Отложено.** Поддержка видеофайлов с камер (MOV, MP4, MTS) запланирована как отдельный трекер после стабилизации v1.0.

Предполагаемый стек: `ffmpeg-sys-next` (FFI) или `gstreamer` (если допустима зависимость от GStreamer runtime). Видеофайлы в текущей версии **отображаются в карусели** с иконкой-заглушкой и при клике показывают уведомление "Видео пока не поддерживается".

Расширения для распознавания (без воспроизведения): `.mov`, `.mp4`, `.mts`, `.m2ts`, `.avi`, `.mkv`.
