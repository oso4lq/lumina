use crate::error::Result;
use std::path::Path;

mod standard;
pub use standard::StandardDecoder;

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub trait Decoder {
    /// Поддерживает ли расширение (в нижнем регистре, без точки).
    fn supports(ext: &str) -> bool
    where
        Self: Sized;

    fn decode(&self, path: &Path) -> Result<DecodedImage>;
}

/// Расширение файла в нижнем регистре без точки, либо пустая строка.
pub fn ext_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn supports_common_extensions() {
        assert!(StandardDecoder::supports("png"));
        assert!(StandardDecoder::supports("jpg"));
        assert!(StandardDecoder::supports("jpeg"));
        assert!(!StandardDecoder::supports("raf"));
        assert!(!StandardDecoder::supports("txt"));
    }

    #[test]
    fn decodes_png_to_rgba() {
        let d = StandardDecoder;
        let img = d.decode(&fixture("red_2x3.png")).unwrap();
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 3);
        assert_eq!(img.rgba.len(), 2 * 3 * 4); // RGBA8
        assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]); // первый пиксель красный
    }

    #[test]
    fn ext_lower_works() {
        assert_eq!(ext_lower(Path::new("A.JPG")), "jpg");
        assert_eq!(ext_lower(Path::new("noext")), "");
    }
}
