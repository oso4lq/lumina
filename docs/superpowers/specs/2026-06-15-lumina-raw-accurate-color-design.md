# Lumina — точный цвет при открытии RAW (design)

**Дата:** 2026-06-15
**Статус:** проектирование → реализация
**Контекст:** уточнение v0.2 (RAW и форматы). Базовая интеграция RAW/HEIC уже в `main`
(`docs/superpowers/specs/2026-06-15-lumina-v0.2-design.md`).

## Проблема

При открытии Fuji RAF полноразмерный develop через `rawler 0.7.2` даёт заметный
**зеленоватый оттенок**.

Причина (разобрано по исходникам rawler 0.7.2):

- Fuji использует **X-Trans** CFA (паттерн 6×6), а не Байер (2×2).
- `RawDevelop::develop_intermediate` корректно применяет баланс белого, цветовую матрицу
  камеры и sRGB-гамму (предупреждение «Skipping calibration» не возникает).
- Но в пути демозаика подключены **только байеровские** алгоритмы
  (`PPGDemosaic`, `Bilinear4Channel`). X-Trans-демозаик в крейте есть
  (`src/imgop/sensor/xtrans/`), однако `develop_intermediate` его **не вызывает**.
- В итоге X-Trans-данные демозаятся как Байер → неверная реконструкция цвета → зеленца.

Вывод: дело не в отсутствии цветовой матрицы, а в несоответствии демозаика для X-Trans.
Для **байеровских** RAW (DNG, NEF, ARW, CR2 …) develop у rawler корректен.

## Цель

Показывать цветоточную картинку для RAW при открытии, не регрессируя качество там, где
develop работает правильно. Это просмотрщик, а не редактор: эталон «реального» вида —
собственный рендер камеры.

## Решение

Fuji-декодер rawler реализует `Decoder::full_image(&source, &params) -> Result<Option<DynamicImage>>`
(`decoders/raf.rs`), который извлекает и декодирует **встроенный полноразмерный JPEG**, записанный
камерой (`read_embedded_jpeg`). Это собственный рендер камеры (верные цвета, film simulation) и он
мгновенный (декод JPEG, без develop). `preview_image` для Fuji возвращает `None`, а `full_image`
работает.

Стратегия: «develop только там, где он корректен».

### Конвейер `RawDecoder` (две стадии, без изменения trait)

1. **`decode_preview` → встроенный JPEG камеры.**
   Зовём `decoder.full_image(&source, &Default::default())`. Если `Some(dynimg)` — возвращаем
   `Some(DecodedImage)`. Для Fuji — мгновенно и цветоточно. Если декодер не отдаёт embedded
   (`None`) — стадия пропускается (как и раньше), показ начнётся со стадии Full.

2. **`decode_full` → «лучшее финальное» по типу сенсора.**
   - Дешёвый зонд: `raw_image(&source, &params, /*dummy=*/true)` → читаем CFA из
     `RawImage.photometric` (`RawPhotometricInterpretation::Cfa(cfg) → cfg.cfa.{width,height}`).
     При `dummy=true` тяжёлая распаковка пикселей пропускается, метаданные (CFA) заполняются.
   - **X-Trans** (`width > 2 || height > 2`): develop ненадёжен → возвращаем тот же
     встроенный JPEG (`full_image`). Подмена preview→full фактически no-op, цвет остаётся верным.
     Если embedded JPEG отсутствует — крайний случай: всё же делаем develop с `log::warn`.
   - **Байер** (`2×2`): `raw_image(&source, &params, /*dummy=*/false)` → `RawDevelop::default()
     .develop_intermediate()` → `to_dynamic_image()`. Здесь подмена preview→full даёт реальный
     выигрыш в качестве (точный полноразмерный develop поверх встроенного превью).

### Чистая логика (тестируемая без фикстур)

```rust
/// CFA крупнее 2×2 (например, X-Trans 6×6) rawler 0.7.2 демозаит некорректно.
fn is_non_bayer_cfa(width: usize, height: usize) -> bool {
    width > 2 || height > 2
}
```

### Вспомогательное

`DynamicImage → DecodedImage` извлекается в локальный помощник (используется и в preview, и в
full) — только примитивы `.to_rgba8().into_raw()` + `.width()/.height()`, кросс-версионно
безопасно.

## Границы изменений

Меняется **только `src/decoder/raw.rs`**.

НЕ меняются: trait `Decoder`, `src/app.rs`, `src/error.rs`, `StandardDecoder`, `HeicDecoder`.
Двухстадийная диспетчеризация (`Stage::{Preview,Full}`, generation-гейтинг) и
`ViewTransform::rescale_for_new_image` уже готовы и подходят как есть.

## Обработка ошибок

- Ошибки `full_image` / `raw_image` / `develop_intermediate` → `LuminaError::Raw(PathBuf, String)`
  (вариант уже существует).
- Отсутствие встроенного JPEG (`Ok(None)`) — не ошибка, а ветка логики:
  - в `decode_preview` → `Ok(None)` (стадия Preview пропускается);
  - в `decode_full` для X-Trans → fallback на develop с `log::warn`.

## Тестирование

- `is_non_bayer_cfa`: `(2,2) → false`, `(6,6) → true` — обычный юнит-тест.
- `supports_raw_exts` — без изменений.
- `#[ignore]`-фикстуры (требуют реальных файлов, запуск вручную):
  - `raw_preview_sample` (Fuji `sample.raf`): теперь ожидает `Some` (через `full_image`).
  - `raw_full_develops_sample`: для Fuji вернёт embedded JPEG; проверки `width>0`,
    `rgba.len() == width*height*4` остаются валидны.
- Ручная приёмка на реальных RAF/DNG (см. чек-лист).

## Чек-лист ручной приёмки

1. RAF (Fuji): открывается мгновенно, цвет естественный (без зеленцы), подмены/«прыжка» нет.
2. DNG/иной байеровский RAW: видно встроенное превью, затем бесшовная подмена на полный develop,
   цвет естественный.
3. Ориентация: у байеровских файлов превью (встроенный JPEG) и develop совпадают по ориентации
   (нет внезапного поворота при подмене). Если расходятся — зафиксировать как отдельный баг
   (повороты — фаза v0.4).
4. Навигация стрелками по папке RAW работает, цвет стабилен от кадра к кадру.
5. Битый/нестандартный RAW не роняет приложение (лог `warn`).

## Известные оговорки (вне объёма)

- **Ориентация** embedded JPEG vs develop для Байера: `image` не применяет EXIF-orientation
  автоматически, оба идут в «сенсорной» ориентации — ожидаемо согласованы; полноценные повороты —
  v0.4.
- **Двойной декод JPEG для Fuji**: JPEG декодируется в preview и снова в full (~100 мс однократно
  на кадр). Сознательный компромисс ради простоты (без изменения сигнатуры trait и `app.rs`).
- **«Честный» X-Trans-develop** (подключение X-Trans-демозаика или libraw) — отдельная задача
  на будущее, если понадобится редакторская проявка RAW.
