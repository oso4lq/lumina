use super::{Decoder, DecodedImage};
use crate::error::{LuminaError, Result};
use rawler::get_decoder;
use rawler::imgop::develop::RawDevelop;
use rawler::rawimage::RawPhotometricInterpretation;
use rawler::rawsource::RawSource;
use std::path::Path;

const EXTS: &[&str] = &[
    "raf", "nef", "nrw", "arw", "srf", "cr2", "cr3", "rw2", "orf", "pef", "dng", "rwl", "iiq",
];

/// CFA крупнее 2×2 (например, X-Trans 6×6) rawler 0.7.2 демозаит некорректно
/// (X-Trans-данные проходят через байеровский демозаик → неверный цвет).
fn is_non_bayer_cfa(width: usize, height: usize) -> bool {
    width > 2 || height > 2
}

/// `image::DynamicImage` → наш плотный RGBA8. rawler и проект используют одну версию `image`.
fn dynamic_to_decoded(dynimg: image::DynamicImage) -> DecodedImage {
    let rgba = dynimg.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    DecodedImage { rgba: rgba.into_raw(), width, height }
}

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
        // Встроенный JPEG камеры: мгновенно и цветоточно (film simulation камеры).
        // None — у формата нет встроенного полного изображения, стадия Preview пропускается.
        let embedded = decoder
            .full_image(&source, &Default::default())
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        Ok(embedded.map(dynamic_to_decoded))
    }

    fn decode_full(&self, path: &Path) -> Result<DecodedImage> {
        let source = RawSource::new(path)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let decoder = get_decoder(&source)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;

        // Дешёвый зонд метаданных: dummy=true пропускает распаковку пикселей,
        // но заполняет CFA. Узнаём тип сенсора без тяжёлого декода.
        let probe = decoder
            .raw_image(&source, &Default::default(), true)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let non_bayer = match &probe.photometric {
            RawPhotometricInterpretation::Cfa(cfg) => {
                is_non_bayer_cfa(cfg.cfa.width, cfg.cfa.height)
            }
            _ => false,
        };

        if non_bayer {
            // X-Trans и пр.: develop rawler даёт неверный цвет → встроенный JPEG камеры.
            match decoder
                .full_image(&source, &Default::default())
                .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?
            {
                Some(dynimg) => return Ok(dynamic_to_decoded(dynimg)),
                None => log::warn!(
                    "{path:?}: non-Bayer CFA без встроенного JPEG — fallback на develop (возможна зеленца)"
                ),
            }
        }

        // Байер (или fallback): полноценный develop.
        let raw = decoder
            .raw_image(&source, &Default::default(), false)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let intermediate = RawDevelop::default()
            .develop_intermediate(&raw)
            .map_err(|e| LuminaError::Raw(path.to_path_buf(), e.to_string()))?;
        let dynimg = intermediate
            .to_dynamic_image()
            .ok_or_else(|| LuminaError::Raw(path.to_path_buf(), "develop вернул None".into()))?;
        Ok(dynamic_to_decoded(dynimg))
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

    // Требует реальный образец tests/fixtures/sample.raf — запускать вручную.
    // Превью теперь = встроенный JPEG камеры (Decoder::full_image), а не preview_image.
    #[test]
    #[ignore]
    fn raw_preview_sample() {
        let d = RawDecoder;
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.raf");
        assert!(d.decode_preview(&path).unwrap().is_some());
    }

    #[test]
    fn bayer_2x2_is_bayer() {
        assert!(!is_non_bayer_cfa(2, 2));
    }

    #[test]
    fn xtrans_6x6_is_non_bayer() {
        assert!(is_non_bayer_cfa(6, 6));
        // несимметричные/прочие крупные паттерны тоже считаем не-Байером
        assert!(is_non_bayer_cfa(2, 6));
        assert!(is_non_bayer_cfa(6, 2));
    }
}
