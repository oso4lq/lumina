//! Обёртка: запись правок тегов через exiftool (subprocess). Логики нет — только
//! формирование команды. Редактируемые форматы — in-place (exiftool делает `_original`),
//! RAW — XMP sidecar (`-o %d%f.xmp`), оригинал RAW не трогаем.
use crate::decoder::{ext_lower, Decoder, RawDecoder};
use crate::error::{LuminaError, Result};
use crate::exif::read::exiftool_path;
use crate::exif::tags::{edits_to_args, TagEdit};
use std::path::Path;
use std::process::Command;

/// Записать набор правок в файл. Пустой список — no-op (успех).
pub fn write_edits(path: &Path, edits: &[TagEdit]) -> Result<()> {
    if edits.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new(exiftool_path());
    cmd.args(edits_to_args(edits));
    // RAW → sidecar (создаёт/обновляет name.xmp); остальные форматы — in-place.
    if RawDecoder::supports(&ext_lower(path)) {
        cmd.args(["-o", "%d%f.xmp"]);
    }
    // `--` завершает разбор опций (защита от подмены флагов именем файла).
    cmd.arg("--").arg(path);
    let out = cmd
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
        write_edits(&tmp, &edits).expect("запись прошла");
        // exiftool оставляет name.jpg_original рядом
        let backup = tmp.with_file_name("lumina_write_test.jpg_original");
        assert!(backup.exists(), "ожидался _original бэкап");
        // прочитать обратно
        let tags = crate::exif::read::read_tags(&tmp).expect("чтение");
        assert_eq!(crate::exif::tags::get(&tags, "EXIF", "Artist").as_deref(), Some("Lumina Test"));
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&backup);
    }
}
