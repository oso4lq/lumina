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
}
