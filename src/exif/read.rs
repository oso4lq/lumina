//! Обёртка: чтение всех EXIF/метаданных файла через exiftool (subprocess).
//! Логики нет — формирование команды и передача stdout в tags::parse.
use crate::error::{LuminaError, Result};
use crate::exif::tags::{self, ExifTags};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// Запустить exiftool, передав аргументы (опции + путь) через UTF-8 argfile на stdin (`-@ -`).
///
/// Это ЕДИНСТВЕННЫЙ надёжный способ передать Unicode-пути (кириллица и пр.) официальному
/// standalone-exiftool на Windows: через argv не-ASCII символы мангелятся в `?` — exiftool
/// видит подстановочные знаки и отвечает «Wildcards don't work…»/«No matching files». Argfile
/// же читается как UTF-8; директива `-charset filename=utf8` сообщает, что имена файлов — в UTF-8.
/// Каждый аргумент — отдельная строка; `--` завершает опции перед путём (защита от имён, начинающихся с `-`).
pub fn run_exiftool_argfile(args: &[&str], path: &Path) -> Result<std::process::Output> {
    // Формат argfile: одна строка = один аргумент. Встроенный перевод строки в аргументе
    // (напр. многострочное значение тега, введённое пользователем) мог бы инжектировать лишние
    // опции exiftool — поэтому вырезаем CR/LF из аргументов и отвергаем путь с переводом строки
    // (на Windows такие имена файлов невозможны, но защищаемся явно).
    let mut content = String::from("-charset\nfilename=utf8\n");
    for a in args {
        let safe = a.replace(['\n', '\r'], "");
        content.push_str(&safe);
        content.push('\n');
    }
    content.push_str("--\n");
    let path_str = path.to_string_lossy();
    if path_str.contains(['\n', '\r']) {
        return Err(LuminaError::Exif(path.to_path_buf(), "путь содержит перевод строки".into()));
    }
    content.push_str(&path_str);
    content.push('\n');

    let mut child = Command::new(exiftool_path())
        .args(["-@", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LuminaError::Exif(path.to_path_buf(), format!("запуск exiftool: {e}")))?;
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(content.as_bytes())
        .map_err(|e| LuminaError::Exif(path.to_path_buf(), format!("stdin exiftool: {e}")))?;
    child
        .wait_with_output()
        .map_err(|e| LuminaError::Exif(path.to_path_buf(), format!("exiftool: {e}")))
}

/// Прочитать все теги файла. Группированный JSON exiftool → ExifTags.
pub fn read_tags(path: &Path) -> Result<ExifTags> {
    let out = run_exiftool_argfile(&["-json", "-G", "-struct"], path)?;
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

    // Регресс: чтение по пути с не-ASCII символами (кириллица). На standalone-exiftool это
    // работает только через UTF-8 argfile (run_exiftool_argfile) — argv мангелит Unicode.
    #[test]
    #[ignore]
    fn read_unicode_path_ok() {
        let base = std::env::temp_dir().join("lumina_unicode_read_test");
        let dir = base.join("Заброшенная деревня");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("DSCF7536.jpg");
        std::fs::copy(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red_2x3.jpg"),
            &f,
        )
        .unwrap();
        let tags = read_tags(&f).expect("чтение по кириллическому пути не должно падать");
        assert!(!tags.groups.is_empty(), "ожидались группы тегов");
        // FileType должен прочитаться
        assert_eq!(tags::get(&tags, "File", "FileType").as_deref(), Some("JPEG"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
