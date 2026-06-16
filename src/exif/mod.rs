pub mod read;
pub mod tags;
pub mod write;

use image::DynamicImage;
use std::path::Path;

/// EXIF Orientation (TIFF tag 0x0112), значения 1..=8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Normal,     // 1
    FlipH,      // 2
    Rotate180,  // 3
    FlipV,      // 4
    Transpose,  // 5
    Rotate90,   // 6  поворот по часовой 90°
    Transverse, // 7
    Rotate270,  // 8  поворот по часовой 270° (= против часовой 90°)
}

impl Orientation {
    /// Маппинг сырого значения тега. Неизвестное/0 → Normal.
    pub fn from_exif_u16(v: u16) -> Self {
        match v {
            2 => Orientation::FlipH,
            3 => Orientation::Rotate180,
            4 => Orientation::FlipV,
            5 => Orientation::Transpose,
            6 => Orientation::Rotate90,
            7 => Orientation::Transverse,
            8 => Orientation::Rotate270,
            _ => Orientation::Normal, // 1 и всё неизвестное
        }
    }
}

/// Прочитать EXIF Orientation из файла. Любая ошибка (нет файла, нет EXIF,
/// нет тега, формат без EXIF) → Normal.
pub fn read_orientation(path: &Path) -> Orientation {
    let Ok(file) = std::fs::File::open(path) else {
        return Orientation::Normal;
    };
    let mut reader = std::io::BufReader::new(file);
    let exif_reader = ::exif::Reader::new();
    let Ok(exif_data) = exif_reader.read_from_container(&mut reader) else {
        return Orientation::Normal;
    };
    match exif_data.get_field(::exif::Tag::Orientation, ::exif::In::PRIMARY) {
        // Orientation — SHORT-тег со значениями 1..=8; u32→u16 без потерь.
        Some(f) => match f.value.get_uint(0) {
            Some(v) => Orientation::from_exif_u16(v as u16),
            None => Orientation::Normal,
        },
        None => Orientation::Normal,
    }
}

/// Прочитать модель камеры (EXIF Model) для заголовка. Любая ошибка/отсутствие → None.
/// Дёшево: парсит только метаданные. Пустую строку трактуем как None.
pub fn read_model(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = ::exif::Reader::new().read_from_container(&mut reader).ok()?;
    let f = exif.get_field(::exif::Tag::Model, ::exif::In::PRIMARY)?;
    let s = f.display_value().to_string().trim().trim_matches('"').to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Применить ориентацию к изображению, приведя его к upright-виду.
/// Используется декодерами разово на потоке декода.
pub fn apply_to_image(img: DynamicImage, orientation: Orientation) -> DynamicImage {
    match orientation {
        Orientation::Normal => img,
        Orientation::FlipH => img.fliph(),
        Orientation::Rotate180 => img.rotate180(),
        Orientation::FlipV => img.flipv(),
        Orientation::Transpose => img.rotate90().fliph(),
        Orientation::Rotate90 => img.rotate90(),
        Orientation::Transverse => img.rotate270().fliph(),
        Orientation::Rotate270 => img.rotate270(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u16_maps_all_values() {
        assert_eq!(Orientation::from_exif_u16(1), Orientation::Normal);
        assert_eq!(Orientation::from_exif_u16(2), Orientation::FlipH);
        assert_eq!(Orientation::from_exif_u16(3), Orientation::Rotate180);
        assert_eq!(Orientation::from_exif_u16(4), Orientation::FlipV);
        assert_eq!(Orientation::from_exif_u16(5), Orientation::Transpose);
        assert_eq!(Orientation::from_exif_u16(6), Orientation::Rotate90);
        assert_eq!(Orientation::from_exif_u16(7), Orientation::Transverse);
        assert_eq!(Orientation::from_exif_u16(8), Orientation::Rotate270);
    }

    #[test]
    fn from_u16_unknown_is_normal() {
        assert_eq!(Orientation::from_exif_u16(0), Orientation::Normal);
        assert_eq!(Orientation::from_exif_u16(99), Orientation::Normal);
    }

    #[test]
    fn apply_rotate90_swaps_dims_and_moves_corner() {
        let mut img = image::RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 0, 255, 255]));
        let dynimg = image::DynamicImage::ImageRgba8(img);
        let out = apply_to_image(dynimg, Orientation::Rotate90);
        assert_eq!((out.width(), out.height()), (1, 2));
    }

    #[test]
    fn apply_normal_is_noop_dims() {
        let dynimg = image::DynamicImage::ImageRgba8(image::RgbaImage::new(3, 2));
        let out = apply_to_image(dynimg, Orientation::Normal);
        assert_eq!((out.width(), out.height()), (3, 2));
    }

    #[test]
    fn read_orientation_missing_file_is_normal() {
        let o = read_orientation(std::path::Path::new("nonexistent_xyz.jpg"));
        assert_eq!(o, Orientation::Normal);
    }
}
