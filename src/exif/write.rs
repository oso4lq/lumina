//! Обёртка: запись правок тегов через exiftool (subprocess). Логики нет — только
//! формирование команды. Все форматы (вкл. RAW) пишутся in-place; режим определяет,
//! оставлять ли восстановимый бэкап (`_original`) или перезаписывать необратимо.
use crate::error::{LuminaError, Result};
use crate::exif::tags::{edits_to_args, TagEdit};
use std::path::Path;
use std::process::Command;

/// Режим записи: оставлять ли восстановимый бэкап `_original`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteMode {
    /// Обычный: exiftool делает `_original` (обратимо). По умолчанию.
    Backup,
    /// Необратимый: `-overwrite_original`, без бэкапа.
    Overwrite,
}

/// Аргументы exiftool для набора правок (без пути). Пустой список → пустой вектор.
/// `Overwrite` добавляет `-overwrite_original`.
pub fn edit_args(edits: &[TagEdit], mode: WriteMode) -> Vec<String> {
    if edits.is_empty() {
        return Vec::new();
    }
    let mut args = edits_to_args(edits);
    if mode == WriteMode::Overwrite {
        args.push("-overwrite_original".to_string());
    }
    args
}

/// Аргументы стирания всех метаданных с сохранением Orientation + ICC (всегда необратимо).
pub fn strip_args() -> Vec<String> {
    vec![
        "-all=".to_string(),
        "-tagsfromfile".to_string(),
        "@".to_string(),
        "-orientation".to_string(),
        "-icc_profile".to_string(),
        "-overwrite_original".to_string(),
    ]
}

/// Применить набор правок in-place (все форматы). Пустой список — no-op (успех).
pub fn write_edits(path: &Path, edits: &[TagEdit], mode: WriteMode) -> Result<()> {
    let args = edit_args(edits, mode);
    if args.is_empty() {
        return Ok(());
    }
    run_exiftool(path, &args)
}

/// Стереть все метаданные (Orientation + ICC сохраняются), необратимо.
pub fn strip_all(path: &Path) -> Result<()> {
    run_exiftool(path, &strip_args())
}

/// Запустить exiftool с аргументами + `-- path`. `--` завершает разбор опций
/// (защита от подмены флагов именем файла).
fn run_exiftool(path: &Path, args: &[String]) -> Result<()> {
    let out = Command::new(crate::exif::read::exiftool_path())
        .args(args)
        .arg("--")
        .arg(path)
        .output()
        .map_err(|e| LuminaError::Exif(path.to_path_buf(), format!("запуск exiftool: {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(LuminaError::Exif(path.to_path_buf(), err));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Требует установленный exiftool. Запуск: cargo test --bin lumina write_real -- --ignored
    #[test]
    #[ignore]
    fn write_real_jpg_set_and_backup() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red_2x3.jpg");
        let tmp = std::env::temp_dir().join("lumina_write_test.jpg");
        std::fs::copy(&src, &tmp).expect("копия фикстуры");
        let edits = vec![TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Lumina Test".into() }];
        write_edits(&tmp, &edits, WriteMode::Backup).expect("запись прошла");
        // exiftool оставляет name.jpg_original рядом
        let backup = tmp.with_file_name("lumina_write_test.jpg_original");
        assert!(backup.exists(), "ожидался _original бэкап");
        // прочитать обратно
        let tags = crate::exif::read::read_tags(&tmp).expect("чтение");
        assert_eq!(crate::exif::tags::get(&tags, "EXIF", "Artist").as_deref(), Some("Lumina Test"));
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn edit_args_backup_is_plain() {
        let edits = vec![TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Jane".into() }];
        assert_eq!(edit_args(&edits, WriteMode::Backup), vec!["-EXIF:Artist=Jane".to_string()]);
    }

    #[test]
    fn edit_args_overwrite_adds_flag() {
        let edits = vec![TagEdit::Delete { group: "XMP".into(), tag: "Rating".into() }];
        assert_eq!(
            edit_args(&edits, WriteMode::Overwrite),
            vec!["-XMP:Rating=".to_string(), "-overwrite_original".to_string()]
        );
    }

    #[test]
    fn edit_args_empty_is_empty_any_mode() {
        assert!(edit_args(&[], WriteMode::Backup).is_empty());
        assert!(edit_args(&[], WriteMode::Overwrite).is_empty());
    }

    #[test]
    fn strip_args_keeps_orientation_and_icc() {
        assert_eq!(
            strip_args(),
            vec![
                "-all=".to_string(),
                "-tagsfromfile".to_string(),
                "@".to_string(),
                "-orientation".to_string(),
                "-icc_profile".to_string(),
                "-overwrite_original".to_string(),
            ]
        );
    }

    #[test]
    #[ignore]
    fn write_overwrite_no_backup() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red_2x3.jpg");
        let tmp = std::env::temp_dir().join("lumina_overwrite_test.jpg");
        std::fs::copy(&src, &tmp).expect("копия фикстуры");
        write_edits(&tmp, &[TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Over".into() }], WriteMode::Overwrite).expect("запись");
        let backup = tmp.with_file_name("lumina_overwrite_test.jpg_original");
        assert!(!backup.exists(), "в режиме Overwrite бэкапа быть не должно");
        let tags = crate::exif::read::read_tags(&tmp).expect("чтение");
        assert_eq!(crate::exif::tags::get(&tags, "EXIF", "Artist").as_deref(), Some("Over"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    #[ignore]
    fn strip_all_removes_pii_keeps_orientation() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red_2x3.jpg");
        let tmp = std::env::temp_dir().join("lumina_strip_test.jpg");
        std::fs::copy(&src, &tmp).expect("копия фикстуры");
        // подготовка: PII (Artist) + Orientation
        write_edits(
            &tmp,
            &[
                TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Secret".into() },
                TagEdit::Set { group: "EXIF".into(), tag: "Orientation".into(), value: "Rotate 90 CW".into() },
            ],
            WriteMode::Overwrite,
        )
        .expect("подготовка");
        strip_all(&tmp).expect("стирание");
        let tags = crate::exif::read::read_tags(&tmp).expect("чтение");
        assert_eq!(crate::exif::tags::get(&tags, "EXIF", "Artist"), None, "PII должен быть удалён");
        assert!(crate::exif::tags::get(&tags, "EXIF", "Orientation").is_some(), "Orientation должен остаться");
        let _ = std::fs::remove_file(&tmp);
    }
}
