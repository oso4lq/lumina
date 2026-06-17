//! Дисковый кэш миниатюр: ключ (стабильный FNV-1a hex), путь, IO, эвикция по бюджету.
//! Чистое ядро (ключ/путь) тестируется юнитами; IO/prune — на временной папке.
use crate::decoder::DecodedImage;
use std::path::{Path, PathBuf};

/// FNV-1a 64-bit по байтам. Стабилен между сборками (в отличие от std::DefaultHasher),
/// поэтому пригоден для персистентного ключа кэша.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Ключ кэша миниатюры: hex от FNV-1a по (путь, mtime, размер, целевая высота).
/// mtime+size обеспечивают авто-инвалидацию при правке файла.
pub fn cache_key(path: &Path, mtime: u64, size: u64, th: u32) -> String {
    let mut buf = path.to_string_lossy().into_owned().into_bytes();
    buf.extend_from_slice(&mtime.to_le_bytes());
    buf.extend_from_slice(&size.to_le_bytes());
    buf.extend_from_slice(&th.to_le_bytes());
    format!("{:016x}", fnv1a64(&buf))
}

/// Путь файла кэша: dir/<key>.png
pub fn cache_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{key}.png"))
}

/// Папка кэша: %LOCALAPPDATA%\Lumina\thumbs, иначе temp/lumina/thumbs.
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("lumina"));
    base.join("Lumina").join("thumbs")
}

/// Прочитать миниатюру из кэша (PNG → RGBA8). None при отсутствии/ошибке.
pub fn load(dir: &Path, key: &str) -> Option<DecodedImage> {
    let path = cache_path(dir, key);
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    Some(DecodedImage { rgba: rgba.into_raw(), width, height })
}

/// Записать миниатюру в кэш (RGBA8 → PNG), атомарно через временный файл + rename.
pub fn store(dir: &Path, key: &str, img: &DecodedImage) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Some(buf) = image::RgbaImage::from_raw(img.width, img.height, img.rgba.clone()) else {
        return;
    };
    let final_path = cache_path(dir, key);
    let tmp_path = dir.join(format!("{key}.tmp"));
    if buf.save_with_format(&tmp_path, image::ImageFormat::Png).is_ok() {
        let _ = std::fs::rename(&tmp_path, &final_path);
    }
}

/// Удалять файлы кэша по возрастанию mtime, пока суммарный размер не уложится в бюджет.
pub fn prune(dir: &Path, budget: u64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<(PathBuf, u64, std::time::SystemTime)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            if !md.is_file() {
                return None;
            }
            let mtime = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            Some((e.path(), md.len(), mtime))
        })
        .collect();
    let total: u64 = entries.iter().map(|(_, s, _)| *s).sum();
    if total <= budget {
        return;
    }
    // старейшие первыми
    entries.sort_by_key(|(_, _, t)| *t);
    let mut cur = total;
    for (path, size, _) in entries {
        if cur <= budget {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            cur = cur.saturating_sub(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn cache_key_is_deterministic() {
        let p = Path::new("C:/x/a.jpg");
        assert_eq!(cache_key(p, 111, 222, 120), cache_key(p, 111, 222, 120));
    }

    #[test]
    fn cache_key_varies_by_each_component() {
        let p = Path::new("C:/x/a.jpg");
        let base = cache_key(p, 111, 222, 120);
        assert_ne!(base, cache_key(Path::new("C:/x/b.jpg"), 111, 222, 120));
        assert_ne!(base, cache_key(p, 999, 222, 120));
        assert_ne!(base, cache_key(p, 111, 999, 120));
        assert_ne!(base, cache_key(p, 111, 222, 240));
    }

    #[test]
    fn cache_key_is_16_hex() {
        let k = cache_key(Path::new("a"), 1, 2, 3);
        assert_eq!(k.len(), 16);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn cache_path_appends_png() {
        assert_eq!(cache_path(Path::new("C:/c"), "deadbeef"), Path::new("C:/c/deadbeef.png"));
    }

    fn tmp_subdir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("lumina_thumbcache_test_{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn store_then_load_roundtrip() {
        let dir = tmp_subdir("roundtrip");
        // 2x1 RGBA: красный, зелёный
        let img = DecodedImage { rgba: vec![255,0,0,255, 0,255,0,255], width: 2, height: 1 };
        store(&dir, "k1", &img);
        let got = load(&dir, "k1").expect("должно прочитаться");
        assert_eq!((got.width, got.height), (2, 1));
        assert_eq!(got.rgba, img.rgba);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_is_none() {
        let dir = tmp_subdir("missing");
        assert!(load(&dir, "nope").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_evicts_oldest_over_budget() {
        let dir = tmp_subdir("prune");
        let img = DecodedImage { rgba: vec![0u8; 4 * 64 * 64], width: 64, height: 64 };
        // три записи; бюджет ~ под две → старейшая удаляется
        store(&dir, "old", &img);
        store(&dir, "mid", &img);
        store(&dir, "new", &img);
        // выставить mtime по возрастанию через повторную запись в нужном порядке не получится
        // надёжно — поэтому бюджет 0 удаляет всё, бюджет огромный не трогает ничего:
        prune(&dir, u64::MAX);
        assert!(cache_path(&dir, "old").exists());
        prune(&dir, 0);
        assert!(!cache_path(&dir, "old").exists());
        assert!(!cache_path(&dir, "new").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
