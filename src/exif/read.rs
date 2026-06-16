//! Обёртка: чтение всех EXIF/метаданных файла через exiftool (subprocess).
//! Логики нет — формирование команды и передача stdout в tags::parse.
use crate::error::{LuminaError, Result};
use crate::exif::tags::{self, ExifTags};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Путь к exiftool: рядом с exe (или assets/bin), иначе `exiftool` из PATH.
pub fn exiftool_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("exiftool.exe"), dir.join("assets/bin/exiftool.exe")] {
                if cand.exists() {
                    return cand;
                }
            }
        }
    }
    PathBuf::from("exiftool") // dev-фолбэк: PATH
}

/// Прочитать все теги файла. Группированный JSON exiftool → ExifTags.
pub fn read_tags(path: &Path) -> Result<ExifTags> {
    // `--` завершает разбор опций: путь трактуется строго как позиционный аргумент,
    // даже если имя файла начинается с `-` (защита от подмены флагов).
    let out = Command::new(exiftool_path())
        .args(["-json", "-G", "-struct", "--"])
        .arg(path)
        .output()
        .map_err(|e| LuminaError::Exif(path.to_path_buf(), format!("запуск exiftool: {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(LuminaError::Exif(path.to_path_buf(), err));
    }
    let json = String::from_utf8_lossy(&out.stdout);
    Ok(tags::parse(&json))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Требует установленный exiftool. Запуск: cargo test --bin lumina read_real -- --ignored
    #[test]
    #[ignore]
    fn read_real_jpg_has_groups() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/red_2x3.jpg");
        let tags = read_tags(&path).expect("exiftool прочитал теги");
        assert!(!tags.groups.is_empty(), "ожидались группы тегов");
    }
}
