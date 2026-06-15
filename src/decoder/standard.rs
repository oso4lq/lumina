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
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(DecodedImage { rgba: rgba.into_raw(), width, height })
    }
}
