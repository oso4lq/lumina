use super::{Decoder, DecodedImage};
use crate::error::{LuminaError, Result};
use rawler::get_decoder;
use rawler::imgop::develop::RawDevelop;
use rawler::rawsource::RawSource;
use std::path::Path;

const EXTS: &[&str] = &[
    "raf", "nef", "nrw", "arw", "srf", "cr2", "cr3", "rw2", "orf", "pef", "dng", "rwl", "iiq",
];

pub struct RawDecoder;

impl Decoder for RawDecoder {
    fn supports(ext: &str) -> bool {
        EXTS.contains(&ext)
    }

    fn decode_preview(&self, path: &Path) -> Result<Option<DecodedImage>> {
        let source = RawSource::new(path)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let decoder = get_decoder(&source)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let preview = decoder
            .preview_image(&source, &Default::default())
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        Ok(preview.map(|dynimg| {
            let rgba = dynimg.to_rgba8();
            let (width, height) = (rgba.width(), rgba.height());
            DecodedImage { rgba: rgba.into_raw(), width, height }
        }))
    }

    fn decode_full(&self, path: &Path) -> Result<DecodedImage> {
        let source = RawSource::new(path)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let decoder = get_decoder(&source)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let raw = decoder
            .raw_image(&source, &Default::default(), false)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let intermediate = RawDevelop::default()
            .develop_intermediate(&raw)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let dynimg = intermediate
            .to_dynamic_image()
            .ok_or_else(|| LuminaError::Raw(path.to_path_buf(), "develop вернул None".into()))?;
        let rgba = dynimg.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        Ok(DecodedImage { rgba: rgba.into_raw(), width, height })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_raw_exts() {
        assert!(RawDecoder::supports("raf"));
        assert!(RawDecoder::supports("nef"));
        assert!(RawDecoder::supports("cr2"));
        assert!(!RawDecoder::supports("jpg"));
        assert!(!RawDecoder::supports("heic"));
    }

    #[test]
    #[ignore]
    fn raw_full_develops_sample() {
        let d = RawDecoder;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.raf");
        let img = d.decode_full(&path).unwrap();
        assert!(img.width > 0 && img.height > 0);
        assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
    }

    #[test]
    #[ignore]
    fn raw_preview_sample() {
        let d = RawDecoder;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.raf");
        assert!(d.decode_preview(&path).unwrap().is_some());
    }
}
