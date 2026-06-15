//! Палитра и размерные константы UI. Чистые данные, без GPU.
//! Цвета хранятся в ЛИНЕЙНОМ пространстве (поверхность sRGB конвертирует на запись).

/// Высота titlebar в логических пикселях.
pub const TITLEBAR_HEIGHT: f32 = 32.0;
/// Ширина одной кнопки управления окном (логические px).
pub const BUTTON_WIDTH: f32 = 46.0;
/// Кегль заголовка (логические px).
pub const TITLE_FONT_SIZE: f32 = 13.0;
/// Кегль глифа кнопки окна (логические px).
pub const ICON_FONT_SIZE: f32 = 10.0;

/// sRGB-компонента (0..=255) → линейная (0..1).
pub fn srgb_to_linear(c: u8) -> f32 {
    let s = c as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Линейный RGBA из sRGB-hex компонент + alpha 0..1.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> [f32; 4] {
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), a]
}

#[derive(Clone, Copy)]
pub struct ThemePalette {
    pub bg_viewer: [f32; 4],
    pub bg_surface: [f32; 4],
    pub text_primary: [f32; 4],
    pub text_secondary: [f32; 4],
    pub button_hover: [f32; 4],
    pub button_close_hover: [f32; 4],
    pub accent: [f32; 4],
}

impl ThemePalette {
    pub fn dark() -> Self {
        Self {
            bg_viewer: rgba(0x11, 0x11, 0x13, 1.0),
            bg_surface: rgba(0x1b, 0x1b, 0x1e, 1.0),
            text_primary: rgba(0xea, 0xea, 0xec, 1.0),
            text_secondary: rgba(0x8a, 0x8a, 0x90, 1.0),
            button_hover: rgba(0xff, 0xff, 0xff, 0.10),
            button_close_hover: rgba(0xe8, 0x11, 0x23, 1.0),
            accent: rgba(0x4a, 0x9e, 0xff, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_endpoints() {
        assert_eq!(srgb_to_linear(0), 0.0);
        assert!((srgb_to_linear(255) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_is_monotonic() {
        assert!(srgb_to_linear(64) < srgb_to_linear(128));
        assert!(srgb_to_linear(128) < srgb_to_linear(200));
    }

    #[test]
    fn dark_palette_alpha() {
        let p = ThemePalette::dark();
        assert_eq!(p.bg_viewer[3], 1.0);
        assert!(p.button_hover[3] < 1.0); // hover полупрозрачный
    }
}
