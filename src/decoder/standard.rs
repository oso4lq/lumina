use super::{Decoder, DecodedImage};
use crate::error::{LuminaError, Result};
use std::path::Path;

const EXTS: &[&str] = &["jpg", "jpeg", "jfif", "png", "bmp", "gif", "tiff", "tif", "webp"];

pub struct StandardDecoder;

impl Decoder for StandardDecoder {
    fn supports(ext: &str) -> bool {
        EXTS.contains(&ext)
    }

    fn decode_preview(&self, _path: &Path) -> Result<Option<DecodedImage>> {
        Ok(None)
    }

    fn decode_full(&self, path: &Path) -> Result<DecodedImage> {
        let img = image::open(path)
            .map_err(|e| LuminaError::Decode(path.to_path_buf(), e))?;
        // EXIF Orientation: image сам не ориентирует — приводим к upright здесь.
        let orientation = crate::exif::read_orientation(path);
        let img = crate::exif::apply_to_image(img, orientation);
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(DecodedImage { rgba: rgba.into_raw(), width, height })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
    }

    #[test]
    fn applies_orientation_6_rotates_landscape_to_portrait() {
        let d = StandardDecoder;
        let img = d.decode_full(&fixture("oriented_3x2_or6.jpg")).unwrap();
        assert_eq!((img.width, img.height), (2, 3));
    }
}
