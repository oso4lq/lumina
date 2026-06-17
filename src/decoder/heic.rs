use super::{Decoder, DecodedImage};
use crate::error::{LuminaError, Result};
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use std::path::Path;

pub(crate) const EXTS: &[&str] = &["heic", "heif"];

pub struct HeicDecoder;

impl Decoder for HeicDecoder {
    fn supports(ext: &str) -> bool {
        EXTS.contains(&ext)
    }

    fn decode_preview(&self, _path: &Path) -> Result<Option<DecodedImage>> {
        Ok(None) // одна стадия: полный декод HEIC и так быстрый
    }

    fn decode_full(&self, path: &Path) -> Result<DecodedImage> {
        let path_str = path
            .to_str()
            .ok_or_else(|| LuminaError::Heic(path.to_path_buf(), "не-UTF8 путь".into()))?;
        let lib = LibHeif::new();
        let ctx = HeifContext::read_from_file(path_str)
            .map_err(|e| LuminaError::Heic(path.to_path_buf(), e.to_string()))?;
        let handle = ctx
            .primary_image_handle()
            .map_err(|e| LuminaError::Heic(path.to_path_buf(), e.to_string()))?;
        let image = lib
            .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
            .map_err(|e| LuminaError::Heic(path.to_path_buf(), e.to_string()))?;

        let width = image.width();
        let height = image.height();
        let planes = image.planes();
        let iv = planes
            .interleaved
            .ok_or_else(|| LuminaError::Heic(path.to_path_buf(), "нет interleaved-плоскости".into()))?;

        // Скопировать с учётом stride в плотный RGBA8.
        let w = width as usize;
        let h = height as usize;
        let stride = iv.stride;
        let src = iv.data;
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            let start = y * stride;
            rgba.extend_from_slice(&src[start..start + w * 4]);
        }

        Ok(DecodedImage { rgba, width, height })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_heic_ext() {
        assert!(HeicDecoder::supports("heic"));
        assert!(HeicDecoder::supports("heif"));
        assert!(!HeicDecoder::supports("jpg"));
    }

    // Требует реальный образец tests/fixtures/sample.heic — запускать вручную:
    // cargo test heic_decodes -- --ignored
    #[test]
    #[ignore]
    fn heic_decodes_sample() {
        let d = HeicDecoder;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.heic");
        let img = d.decode_full(&path).unwrap();
        assert!(img.width > 0 && img.height > 0);
        assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
    }

    // Приёмка: портретный HEIC (снят вертикально) должен выйти портретным —
    // libheif применяет irot/imir сам, повторно НЕ ориентируем.
    // cargo test heic_portrait_is_upright -- --ignored
    #[test]
    #[ignore]
    fn heic_portrait_is_upright() {
        let d = HeicDecoder;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/portrait.heic");
        let img = d.decode_full(&path).unwrap();
        assert!(img.height > img.width, "ожидался портрет: {}×{}", img.width, img.height);
    }
}
