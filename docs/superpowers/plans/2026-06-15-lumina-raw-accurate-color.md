# Точный цвет при открытии RAW — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Убрать зеленоватый оттенок при открытии Fuji RAF, показывая встроенный JPEG камеры для X-Trans и оставляя rawler-develop только для байеровских RAW.

**Architecture:** Меняется только `src/decoder/raw.rs`. `decode_preview` отдаёт встроенный JPEG камеры (`Decoder::full_image`). `decode_full` дёшево зондирует тип сенсора (`raw_image(dummy=true)` → CFA): для X-Trans (CFA крупнее 2×2) возвращает встроенный JPEG, для Байера — полноценный `RawDevelop`. Двухстадийная диспетчеризация и подмена вида в `app.rs`/`view.rs` уже готовы и не меняются.

**Tech Stack:** Rust 2021, rawler 0.7.2 (`get_decoder`, `RawSource`, `Decoder::{full_image, raw_image}`, `RawDevelop`, `RawPhotometricInterpretation::Cfa(CFAConfig{cfa: CFA})`), image 0.25 (общая версия с rawler).

**Дизайн:** `docs/superpowers/specs/2026-06-15-lumina-raw-accurate-color-design.md`

---

## Окружение исполнителя (важно)

- В Bash-инструменте cargo/rustc НЕ в PATH. В начале КАЖДОГО bash-вызова добавлять:
  `export PATH="$HOME/.cargo/bin:$PATH"`.
- Рабочая директория: `D:/0. workat/Lumina`. Ветка `main`.
- Git без глобального user — коммит:
  `git -c user.name='Lumina Dev' -c user.email='mikhail.zorin@gomining.com' commit -m "..."`.
- Реальные RAW-фикстуры отсутствуют (`tests/fixtures/sample.raf`), `#[ignore]`-тесты не запускать.

## Структура файлов

| Файл | Что делаем | Ответственность |
|---|---|---|
| `src/decoder/raw.rs` | Modify | помощник `is_non_bayer_cfa`, помощник `dynamic_to_decoded`, переработка `decode_preview`/`decode_full`, обновление `#[ignore]`-тестов |

Порядок: чистый помощник детекции (TDD) → переработка декода с зелёным билдом.

---

## Task 1: Чистый признак не-Байера `is_non_bayer_cfa` (TDD)

**Files:**
- Modify: `src/decoder/raw.rs`

- [ ] **Step 1: Написать падающий тест**

В `src/decoder/raw.rs` в блоке `#[cfg(test)] mod tests` добавить:
```rust
    #[test]
    fn bayer_2x2_is_bayer() {
        assert!(!is_non_bayer_cfa(2, 2));
    }

    #[test]
    fn xtrans_6x6_is_non_bayer() {
        assert!(is_non_bayer_cfa(6, 6));
        // несимметричные/прочие крупные паттерны тоже считаем не-Байером
        assert!(is_non_bayer_cfa(2, 6));
        assert!(is_non_bayer_cfa(6, 2));
    }
```

- [ ] **Step 2: Запустить — не компилируется**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test raw 2>&1 | head -20`
Expected: ошибка «cannot find function `is_non_bayer_cfa`».

- [ ] **Step 3: Реализовать помощник**

В `src/decoder/raw.rs` на уровне модуля (после `const EXTS`/`pub struct RawDecoder`, вне `impl`) добавить:
```rust
/// CFA крупнее 2×2 (например, X-Trans 6×6) rawler 0.7.2 демозаит некорректно
/// (X-Trans-данные проходят через байеровский демозаик → неверный цвет).
fn is_non_bayer_cfa(width: usize, height: usize) -> bool {
    width > 2 || height > 2
}
```

- [ ] **Step 4: Запустить — проходит**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test raw 2>&1 | tail -10`
Expected: `bayer_2x2_is_bayer`, `xtrans_6x6_is_non_bayer`, `supports_raw_exts` зелёные; develop/preview — ignored.

- [ ] **Step 5: Commit**

```bash
git add src/decoder/raw.rs
git -c user.name='Lumina Dev' -c user.email='mikhail.zorin@gomining.com' commit -m "feat: is_non_bayer_cfa — признак X-Trans/не-Байер CFA"
```

---

## Task 2: Встроенный JPEG для X-Trans, develop для Байера

**Files:**
- Modify: `src/decoder/raw.rs`

- [ ] **Step 1: Обновить импорты и добавить помощник `dynamic_to_decoded`**

В `src/decoder/raw.rs` заменить верхний блок импортов (строки с `use ...`) на:
```rust
use super::{Decoder, DecodedImage};
use crate::error::{LuminaError, Result};
use rawler::get_decoder;
use rawler::imgop::develop::RawDevelop;
use rawler::rawimage::RawPhotometricInterpretation;
use rawler::rawsource::RawSource;
use std::path::Path;
```

И добавить на уровне модуля (рядом с `is_non_bayer_cfa`) помощник конвертации.
`image` в дереве одной версии (0.25) с rawler, поэтому имя `image::DynamicImage` безопасно:
```rust
/// `image::DynamicImage` → наш плотный RGBA8. rawler и проект используют одну версию `image`.
fn dynamic_to_decoded(dynimg: image::DynamicImage) -> DecodedImage {
    let rgba = dynimg.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    DecodedImage { rgba: rgba.into_raw(), width, height }
}
```

- [ ] **Step 2: Переработать `decode_preview` (встроенный JPEG камеры)**

В `src/decoder/raw.rs` заменить метод `decode_preview` на:
```rust
    fn decode_preview(&self, path: &Path) -> Result<Option<DecodedImage>> {
        let source = RawSource::new(path)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let decoder = get_decoder(&source)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        // Встроенный JPEG камеры: мгновенно и цветоточно (film simulation камеры).
        // None — у формата нет встроенного полного изображения, стадия Preview пропускается.
        let embedded = decoder
            .full_image(&source, &Default::default())
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        Ok(embedded.map(dynamic_to_decoded))
    }
```

- [ ] **Step 3: Переработать `decode_full` (зонд CFA → ветка по сенсору)**

В `src/decoder/raw.rs` заменить метод `decode_full` на:
```rust
    fn decode_full(&self, path: &Path) -> Result<DecodedImage> {
        let source = RawSource::new(path)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let decoder = get_decoder(&source)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;

        // Дешёвый зонд метаданных: dummy=true пропускает распаковку пикселей,
        // но заполняет CFA. Узнаём тип сенсора без тяжёлого декода.
        let probe = decoder
            .raw_image(&source, &Default::default(), true)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let non_bayer = match &probe.photometric {
            RawPhotometricInterpretation::Cfa(cfg) => {
                is_non_bayer_cfa(cfg.cfa.width, cfg.cfa.height)
            }
            _ => false,
        };

        if non_bayer {
            // X-Trans и пр.: develop rawler даёт неверный цвет → встроенный JPEG камеры.
            match decoder
                .full_image(&source, &Default::default())
                .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?
            {
                Some(dynimg) => return Ok(dynamic_to_decoded(dynimg)),
                None => log::warn!(
                    "{path:?}: non-Bayer CFA без встроенного JPEG — fallback на develop (возможна зеленца)"
                ),
            }
        }

        // Байер (или fallback): полноценный develop.
        let raw = decoder
            .raw_image(&source, &Default::default(), false)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let intermediate = RawDevelop::default()
            .develop_intermediate(&raw)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let dynimg = intermediate
            .to_dynamic_image()
            .ok_or_else(|| LuminaError::Raw(path.to_path_buf(), "develop вернул None".into()))?;
        Ok(dynamic_to_decoded(dynimg))
    }
```

- [ ] **Step 4: Обновить комментарий ignored-теста preview**

В `src/decoder/raw.rs` тест `raw_preview_sample` оставить как есть по коду, но обновить
комментарий над ним, чтобы отражал источник превью:
```rust
    // Требует реальный образец tests/fixtures/sample.raf — запускать вручную.
    // Превью теперь = встроенный JPEG камеры (Decoder::full_image), а не preview_image.
    #[test]
    #[ignore]
    fn raw_preview_sample() {
        let d = RawDecoder;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.raf");
        // у Fuji RAF встроенный JPEG есть всегда → Some
        assert!(d.decode_preview(&path).unwrap().is_some());
    }
```

- [ ] **Step 5: Проверить сборку и тесты**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo build 2>&1 | tail -10`
Expected: компилируется. Если `rawler::rawimage::RawPhotometricInterpretation` не резолвится —
сверить путь: `find ~/.cargo -path '*rawler-0.7*/src/rawimage.rs'`; тип объявлен там как
`pub enum RawPhotometricInterpretation { Cfa(CFAConfig), .. }`, `CFAConfig { pub cfa: CFA }`,
`CFA { pub width: usize, pub height: usize }`. Если `full_image` имеет другую сигнатуру —
сверить с `decoders/mod.rs` (по умолчанию `fn full_image(&self, &RawSource, &RawDecodeParams)
-> Result<Option<DynamicImage>>`).

Run: `cargo test 2>&1 | tail -15`
Expected: все обычные тесты зелёные (decoder + catalog + view + input), RAW/HEIC — ignored.

- [ ] **Step 6: Commit**

```bash
git add src/decoder/raw.rs
git -c user.name='Lumina Dev' -c user.email='mikhail.zorin@gomining.com' commit -m "feat: точный цвет RAW — встроенный JPEG для X-Trans, develop для Байера"
```

---

## Task 3: Ручная приёмка (опц., требует реальных файлов)

**Files:** —

- [ ] **Step 1: Открыть Fuji RAF**

Run (vcpkg-bin в PATH для рантайм-DLL libheif, оно линкуется в общий бинарь):
```bash
export PATH="$HOME/.cargo/bin:$HOME/vcpkg/installed/x64-windows/bin:$PATH"
cd "D:/0. workat/Lumina"
RUST_LOG=warn cargo run -- "D:/1. urbex/800-899/864. Недостроенный кирпичный завод/raw/DSCF7592.RAF"
```
Expected: открывается мгновенно, цвет естественный (без зеленцы), «прыжка» при подмене нет.

- [ ] **Step 2: Открыть DNG (байеровский)**

Run: `cargo run -- "D:/1. urbex/800-899/864. Недостроенный кирпичный завод/raw/DSCF7592-HDR.dng"`
Expected: видно встроенное превью, затем бесшовная подмена на полный develop, цвет естественный,
ориентация не меняется при подмене.

- [ ] **Step 3: Прогнать ignored-тесты, если есть фикстуры**

Положить `tests/fixtures/sample.raf`, затем:
Run: `cargo test raw_full_develops_sample raw_preview_sample -- --ignored 2>&1 | tail -15`
Expected: оба PASS.

---

## Самопроверка плана

**Покрытие дизайна:** встроенный JPEG в preview (Task 2 Step 2); зонд CFA + ветка X-Trans/Байер
в full (Task 2 Step 3); чистый признак `is_non_bayer_cfa` + тест (Task 1); помощник
`dynamic_to_decoded` (Task 2 Step 1); обработка отсутствия embedded JPEG как ветки логики/fallback
(Task 2 Step 3); тестирование чистой логики + ignored-фикстуры (Task 1, Task 2 Step 4);
ручная приёмка с чек-листом цвета и ориентации (Task 3). ✓

**Согласованность типов:** `is_non_bayer_cfa(usize, usize) -> bool`;
`dynamic_to_decoded(image::DynamicImage) -> DecodedImage`;
`RawPhotometricInterpretation::Cfa(CFAConfig{ cfa: CFA{ width, height } })`;
`Decoder::full_image(&RawSource, &RawDecodeParams) -> Result<Option<DynamicImage>>`;
`raw_image(&RawSource, &RawDecodeParams, dummy: bool) -> Result<RawImage>` с `pub photometric`.
trait `Decoder` проекта, `app.rs`, `error.rs` НЕ меняются. ✓

**Заметки исполнителю:**
- Не менять сигнатуру trait `Decoder`, `app.rs`, `error.rs` — это вне объёма.
- `image` в дереве одной версии с rawler (0.25) — имя `image::DynamicImage` в помощнике безопасно.
- Двойной декод JPEG для Fuji (preview + full) — сознательный компромисс, не «чинить».
