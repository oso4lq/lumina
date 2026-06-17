# Очистка IFD1-дублей при записи EXIF — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** При записи EXIF-тега очищать его устаревший IFD1-дубль (thumbnail-IFD), чтобы правка
(напр. Artist на Fuji RAF) была видна при перечитывании, а не перебивалась stale-значением из IFD1.

**Architecture:** Write-side фикс в одной чистой функции `exif::tags::edits_to_args`: к каждой
правке группы `EXIF` (`Set`/`Delete`) дописывается `-IFD1:{tag}=` (удаление thumbnail-копии).
IFD0 становится единственным авторитетным. Решение generic: для не-EXIF групп и форматов без IFD1
дополнительный аргумент не добавляется / является безопасным no-op-удалением.

**Tech Stack:** Rust, внешний `exiftool` (subprocess). Дизайн:
`docs/superpowers/specs/2026-06-17-lumina-exif-ifd1-duplicate-clear-design.md`.

## Global Constraints

- Крейт `lumina` **бинарный**: `cargo test --lib` падает. Тесты — `cargo test --bin lumina <фильтр>`,
  сборка — `cargo build`.
- Bash-инструмент НЕ видит cargo/env сборки. В начале КАЖДОГО bash-вызова с cargo выставлять inline:
  `export PATH="$HOME/.cargo/bin:$HOME/vcpkg/installed/x64-windows/bin:$PATH" VCPKG_ROOT="$HOME/vcpkg" LIBCLANG_PATH="C:\\Program Files\\LLVM\\bin" VCPKGRS_DYNAMIC=1`
- Коммиты завершать трейлером: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`
- Работа в ветке `v0.4b-exif`.
- **exiftool** для интеграционного теста: dev-shim `target/debug/exiftool.exe` (exiftool 13.59) уже на месте.
  Юнит-тесты exiftool не требуют; интеграционный помечен `#[ignore]`.
- **Только группа `EXIF`** получает IFD1-очистку (IFD1 — EXIF/TIFF-концепт; XMP/IPTC/GPS дублей в IFD1
  не имеют). Применяется к `Set` и `Delete`. `DeleteAllGps` не меняется.
- **Контекст (формато-зависимость, для понимания теста):** на реальном Fuji RAF exiftool на дубле
  IFD0/IFD1 при чтении `-json -G` отдаёт **IFD1** (баг); на крошечной JPG-фикстуре — **IFD0**. Поэтому
  интеграционный тест на JPG проверяет **механику фикса** (IFD1-дубль физически удалён после записи),
  а не «чтение вернуло новое значение» (на JPG это прошло бы и без фикса).

---

## Файловая структура

- **Изменяется** `src/exif/tags.rs` — `edits_to_args` (логика IFD1-очистки) + юнит-тесты (новые + правка
  одного существующего).
- **Изменяется** `src/exif/write.rs` — правка существующего теста `edit_args_backup_is_plain` (его ожидаемый
  вывод для EXIF-правки меняется) + новый `#[ignore]` интеграционный тест.
- **Изменяется** `ROADMAP.md` — заметка о фиксе.

---

## Task 1: `edits_to_args` — очистка IFD1-дубля для EXIF-правок

**Files:**
- Modify: `src/exif/tags.rs` (функция `edits_to_args` + модуль `tests`)
- Modify: `src/exif/write.rs` (тест `edit_args_backup_is_plain`)

**Interfaces:**
- Consumes: `TagEdit { Set{group,tag,value}, Delete{group,tag}, DeleteAllGps }` (без изменений).
- Produces: `edits_to_args(&[TagEdit]) -> Vec<String>` — сигнатура без изменений, но теперь EXIF-правка
  (`Set`/`Delete`) даёт **два** аргумента: основной + `-IFD1:{tag}=`. Вызывающий `write::edit_args`
  (дописывает `-overwrite_original` в конец плоского вектора) не требует изменений.

- [ ] **Step 1: Написать падающие юнит-тесты**

В `#[cfg(test)] mod tests` в `src/exif/tags.rs` добавить:
```rust
    #[test]
    fn edits_to_args_exif_set_clears_ifd1_dup() {
        let edits = vec![TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Jane".into() }];
        assert_eq!(
            edits_to_args(&edits),
            vec!["-EXIF:Artist=Jane".to_string(), "-IFD1:Artist=".to_string()]
        );
    }

    #[test]
    fn edits_to_args_exif_delete_clears_ifd1_dup() {
        let edits = vec![TagEdit::Delete { group: "EXIF".into(), tag: "Artist".into() }];
        assert_eq!(
            edits_to_args(&edits),
            vec!["-EXIF:Artist=".to_string(), "-IFD1:Artist=".to_string()]
        );
    }

    #[test]
    fn edits_to_args_non_exif_groups_unchanged() {
        // XMP/IPTC/GPS дублей в IFD1 не имеют — один аргумент на правку
        let edits = vec![
            TagEdit::Set { group: "XMP".into(), tag: "Rating".into(), value: "5".into() },
            TagEdit::Delete { group: "IPTC".into(), tag: "Keywords".into() },
        ];
        assert_eq!(
            edits_to_args(&edits),
            vec!["-XMP:Rating=5".to_string(), "-IPTC:Keywords=".to_string()]
        );
    }
```

Также **обновить** существующий тест `edits_to_args_set_delete_gps`: блок ожидаемого вывода
```rust
        assert_eq!(
            args,
            vec![
                "-EXIF:Artist=Jane".to_string(),
                "-XMP:Rating=".to_string(),
                "-gps:all=".to_string(),
            ]
        );
```
заменить на (EXIF-правка теперь добавляет IFD1-очистку):
```rust
        assert_eq!(
            args,
            vec![
                "-EXIF:Artist=Jane".to_string(),
                "-IFD1:Artist=".to_string(),
                "-XMP:Rating=".to_string(),
                "-gps:all=".to_string(),
            ]
        );
```

- [ ] **Step 2: Запустить — убедиться, что падают**

Run: `cargo test --bin lumina tags::tests::edits_to_args`
Expected: FAIL — новые тесты и обновлённый `edits_to_args_set_delete_gps` не сходятся (IFD1-аргумент ещё не добавляется).

- [ ] **Step 3: Реализовать IFD1-очистку**

В `src/exif/tags.rs` функцию `edits_to_args` **заменить целиком** на:
```rust
/// Набор правок → аргументы exiftool (без пути; путь добавляет write::write_edits).
/// `Set` → `-Group:Tag=value`; `Delete` → `-Group:Tag=`; `DeleteAllGps` → `-gps:all=`.
/// Для группы `EXIF` дополнительно очищается IFD1-дубль (`-IFD1:Tag=`): иначе exiftool при
/// чтении `-json -G` на дублированном теге (напр. Fuji зеркалит Artist в thumbnail-IFD) отдаёт
/// устаревшее значение из IFD1, и правка IFD0 «не видна». Для других групп / форматов без IFD1
/// это безопасный no-op (XMP/IPTC/GPS дублей в IFD1 не имеют; отсутствующий тег — удаление-no-op).
pub fn edits_to_args(edits: &[TagEdit]) -> Vec<String> {
    let mut args = Vec::new();
    for e in edits {
        match e {
            TagEdit::Set { group, tag, value } => {
                args.push(format!("-{group}:{tag}={value}"));
                if group == "EXIF" {
                    args.push(format!("-IFD1:{tag}="));
                }
            }
            TagEdit::Delete { group, tag } => {
                args.push(format!("-{group}:{tag}="));
                if group == "EXIF" {
                    args.push(format!("-IFD1:{tag}="));
                }
            }
            TagEdit::DeleteAllGps => args.push("-gps:all=".to_string()),
        }
    }
    args
}
```

- [ ] **Step 4: Обновить тест в `write.rs`**

В `src/exif/write.rs`, тест `edit_args_backup_is_plain`, строку
```rust
        assert_eq!(edit_args(&edits, WriteMode::Backup), vec!["-EXIF:Artist=Jane".to_string()]);
```
заменить на (Backup-режим, EXIF-правка теперь включает IFD1-очистку, без `-overwrite_original`):
```rust
        assert_eq!(
            edit_args(&edits, WriteMode::Backup),
            vec!["-EXIF:Artist=Jane".to_string(), "-IFD1:Artist=".to_string()]
        );
```
> Тест `edit_args_overwrite_adds_flag` использует группу `XMP` (`Delete XMP Rating`) — он **не меняется**
> (IFD1-очистка только для EXIF): остаётся `["-XMP:Rating=", "-overwrite_original"]`.

- [ ] **Step 5: Запустить — убедиться, что проходят**

Run: `cargo test --bin lumina tags::tests`
Expected: PASS (новые 3 + обновлённый + прежние).

Run: `cargo test --bin lumina write::tests::edit_args`
Expected: PASS (оба `edit_args`-теста: backup обновлён, overwrite без изменений).

- [ ] **Step 6: Полная сборка и прогон**

Run: `cargo build`
Expected: компилируется, без новых warnings.

Run: `cargo test --bin lumina`
Expected: PASS — все юнит-тесты; `#[ignore]` пропущены.

- [ ] **Step 7: Commit**

```bash
git add src/exif/tags.rs src/exif/write.rs
git commit -m "fix(exif): очищать IFD1-дубль EXIF-тега при записи (правка Artist на Fuji была не видна)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: интеграционный тест (IFD1-дубль удалён после записи) + ROADMAP

**Files:**
- Modify: `src/exif/write.rs` (модуль `tests` — новый `#[ignore]`)
- Modify: `ROADMAP.md`

- [ ] **Step 1: Написать `#[ignore]` интеграционный тест**

В `#[cfg(test)] mod tests` в `src/exif/write.rs` добавить (рядом с прочими `#[ignore]`):
```rust
    #[test]
    #[ignore]
    fn write_clears_ifd1_artist_duplicate() {
        // Воспроизводим дубль: IFD0:Artist + IFD1:Artist, затем правим через write_edits
        // (Set EXIF:Artist) и проверяем, что IFD1-копия физически удалена, а IFD0 = новое значение.
        // На JPG exiftool создаёт обе копии (на RAF на чтении IFD1 перебивал бы IFD0 — суть бага).
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red_2x3.jpg");
        let tmp = std::env::temp_dir().join("lumina_ifd1_dup_test.jpg");
        std::fs::copy(&src, &tmp).expect("копия фикстуры");

        // подготовка: создать дубль Artist в IFD0 и IFD1 напрямую через exiftool
        let prep = std::process::Command::new(crate::exif::read::exiftool_path())
            .args(["-IFD0:Artist=Old", "-IFD1:Artist=Stale", "-overwrite_original"])
            .arg("--")
            .arg(&tmp)
            .output()
            .expect("подготовка дубля");
        assert!(prep.status.success(), "prep: {}", String::from_utf8_lossy(&prep.stderr));

        // правка через продакшн-путь
        write_edits(
            &tmp,
            &[TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "New".into() }],
            WriteMode::Overwrite,
        )
        .expect("запись");

        // проверка: -a -G1 — IFD0:Artist=New, IFD1:Artist отсутствует
        let out = std::process::Command::new(crate::exif::read::exiftool_path())
            .args(["-a", "-G1", "-s3", "-Artist"])
            .arg("--")
            .arg(&tmp)
            .output()
            .expect("чтение -a -G1");
        let dump = String::from_utf8_lossy(&out.stdout);
        // после фикса остаётся ровно одна строка Artist (IFD0=New); IFD1-дубль удалён
        let lines: Vec<&str> = dump.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines, vec!["New"], "ожидался только IFD0:Artist=New, получено: {dump:?}");

        let _ = std::fs::remove_file(&tmp);
    }
```
> Тест читает напрямую через exiftool `-a -G1` (показывает обе IFD-копии раздельно) — `read_tags`
> (`-json -G`) их схлопывает и не позволяет увидеть, удалён ли IFD1-дубль.

- [ ] **Step 2: Запустить интеграционный тест**

Run: `cargo test --bin lumina write::tests::write_clears_ifd1_artist_duplicate -- --ignored`
Expected: PASS (дубль создан, после `write_edits` остался только `IFD0:Artist=New`).

- [ ] **Step 3: (опц.) прогнать все интеграционные write-тесты**

Run: `cargo test --bin lumina write:: -- --ignored`
Expected: PASS (прежние 3 + новый = 4).

- [ ] **Step 4: ROADMAP**

В `ROADMAP.md`, в блоке «Прогресс v0.4d (детально)», после строки про интеграцию в `app.rs`
добавить пункт списка:
```markdown
- [x] Фикс read-back правок EXIF на форматах с IFD1-дублями (Fuji зеркалит Artist в thumbnail-IFD):
      `edits_to_args` очищает `-IFD1:{tag}=` при записи EXIF-тега — иначе exiftool `-json -G` отдавал
      устаревший IFD1, и правка IFD0 была не видна ([дизайн](docs/superpowers/specs/2026-06-17-lumina-exif-ifd1-duplicate-clear-design.md))
```
И обновить счётчик тестов в абзаце ниже: было «10 новых в v0.4d … +2 новых `#[ignore]`» — учесть
3 новых юнит-теста (`tags::edits_to_args_*`) и 1 новый `#[ignore]` (`write_clears_ifd1_artist_duplicate`),
т.е. «13 новых в v0.4d … +3 `#[ignore]`». (Проверить фактический счётчик `cargo test --bin lumina` и
указать точно.)

- [ ] **Step 5: Commit**

```bash
git add src/exif/write.rs ROADMAP.md
git commit -m "test(exif): #[ignore]-тест очистки IFD1-дубля + ROADMAP" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 6: Ручная приёмка** (пользователь, на копии Fuji RAF — оригиналы не трогать)

`cargo run -- "путь\к\копии.RAF"` → открыть EXIF popup → отредактировать `Artist` → Save (обычный режим)
→ закрыть/переоткрыть popup → значение Artist = новое (а не камерное). Рядом `*.RAF_original`, `.xmp` нет.

---

## Self-review (выполнен при написании)

- **Покрытие спека:** write-side очистка IFD1 для EXIF Set/Delete → Task 1 Step 3; generic/безопасность
  (не-EXIF без изменений) → юнит-тест `edits_to_args_non_exif_groups_unchanged`; интеграционная проверка
  «дубль удалён» → Task 2; граница «фикс на момент записи» — реализована самим подходом (read-путь не трогаем);
  ROADMAP → Task 2 Step 4. Read-side подход отвергнут (спек) — не реализуется.
- **Сломанные существующие тесты учтены:** `tags::edits_to_args_set_delete_gps` (Task 1 Step 1) и
  `write::edit_args_backup_is_plain` (Task 1 Step 4) — оба обновлены под новый вывод EXIF-правки;
  `write::edit_args_overwrite_adds_flag` (XMP) намеренно не трогается.
- **Согласованность типов:** сигнатура `edits_to_args` неизменна; `write::edit_args`/`write_edits`
  работают с плоским `Vec<String>` — переменное число аргументов на правку их не ломает.
- **Плейсхолдеров нет:** весь код приведён; счётчик тестов в ROADMAP помечен «проверить фактический».
