use crate::error::Result;
use std::path::Path;

mod standard;
pub use standard::StandardDecoder;

mod heic;
pub use heic::HeicDecoder;

mod raw;
pub use raw::RawDecoder;

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

    /// Быстрое встроенное превью. `None` — у формата превью нет (одна стадия).
    fn decode_preview(&self, path: &Path) -> Result<Option<DecodedImage>>;

    /// Полное декодирование (для RAW — базовый develop).
    fn decode_full(&self, path: &Path) -> Result<DecodedImage>;
}

/// Расширение файла в нижнем регистре без точки, либо пустая строка.
pub fn ext_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Поддерживается ли расширение хоть одним декодером.
pub fn supported(ext: &str) -> bool {
    StandardDecoder::supports(ext) || RawDecoder::supports(ext) || HeicDecoder::supports(ext)
}

/// Подобрать декодер по расширению. Приоритет: Raw → Heic → Standard.
pub fn decoder_for(ext: &str) -> Option<Box<dyn Decoder + Send>> {
    if RawDecoder::supports(ext) {
        Some(Box::new(RawDecoder))
    } else if HeicDecoder::supports(ext) {
        Some(Box::new(HeicDecoder))
    } else if StandardDecoder::supports(ext) {
        Some(Box::new(StandardDecoder))
    } else {
        None
    }
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
    fn standard_has_no_preview() {
        let d = StandardDecoder;
        // у обычного формата нет отдельного превью — одна стадия
        assert!(d.decode_preview(Path::new("x.jpg")).unwrap().is_none());
    }

    #[test]
    fn decodes_png_via_full() {
        let d = StandardDecoder;
        let img = d.decode_full(&fixture("red_2x3.png")).unwrap();
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 3);
        assert_eq!(img.rgba.len(), 2 * 3 * 4);
        assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn ext_lower_works() {
        assert_eq!(ext_lower(Path::new("A.JPG")), "jpg");
        assert_eq!(ext_lower(Path::new("noext")), "");
    }

    #[test]
    fn router_supported_covers_all_families() {
        assert!(supported("jpg"));   // standard
        assert!(supported("raf"));   // raw (fuji)
        assert!(supported("nef"));   // raw (nikon)
        assert!(supported("heic"));  // heic
        assert!(!supported("txt"));
    }

    #[test]
    fn router_picks_decoder_or_none() {
        assert!(decoder_for("jpg").is_some());
        assert!(decoder_for("raf").is_some());
        assert!(decoder_for("heic").is_some());
        assert!(decoder_for("txt").is_none());
    }
}
