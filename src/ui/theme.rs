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

/// Высота divider-полоски (логические px).
pub const DIVIDER_HEIGHT: f32 = 22.0;
/// Высота bottom bar при полной видимости (логические px).
pub const BOTTOM_BAR_HEIGHT: f32 = 84.0;
/// Ширина левой мета-зоны bottom bar (логические px).
pub const META_WIDTH: f32 = 132.0;
/// Ширина правой зоны кнопок действий (логические px). 3 кнопки: поворот/fullscreen/инфо.
pub const ACTIONS_WIDTH: f32 = 114.0;
/// Ширина (толщина) экранной стрелки навигации — тонкая полоса у края viewer (логические px).
pub const NAV_ARROW_W: f32 = 44.0;
/// Кегль глифа-шеврона стрелки (логические px).
pub const NAV_CHEVRON_SIZE: f32 = 20.0;

/// EXIF popup (логические px).
pub const POPUP_MAX_W: f32 = 560.0;
pub const POPUP_MAX_H: f32 = 620.0;
pub const POPUP_MARGIN: f32 = 40.0;     // минимальный зазор до краёв окна
pub const POPUP_HEADER_H: f32 = 40.0;
pub const POPUP_SEARCH_H: f32 = 34.0;
pub const POPUP_ROW_H: f32 = 26.0;      // строка тега
pub const POPUP_GROUP_H: f32 = 22.0;    // заголовок группы
pub const POPUP_PAD: f32 = 12.0;        // внутренний горизонтальный отступ
pub const POPUP_TITLE_SIZE: f32 = 14.0;
pub const POPUP_ROW_SIZE: f32 = 12.0;
pub const POPUP_GROUP_SIZE: f32 = 11.0;
pub const POPUP_RADIUS: f32 = 10.0;
pub const POPUP_CARET_W: f32 = 1.5;       // ширина каретки поиска (логические px)
pub const POPUP_CARET_BLINK: f32 = 0.53;  // полупериод мигания каретки (сек)
/// Футер popup (логические px) — кнопки Сохранить/Отменить всегда внизу карточки.
pub const POPUP_FOOTER_H: f32 = 46.0;
/// Размер иконок-действий строки (✎/✕) и кнопок футера — текст (логические px).
pub const POPUP_ACTION_ICON: f32 = 13.0;
pub const POPUP_BTN_SIZE: f32 = 12.0;   // кегль текста кнопок футера

/// Полупрозрачное затемнение фона под popup (линейное пространство, alpha не зависит от srgb).
pub const POPUP_DIM: [f32; 4] = [0.0, 0.0, 0.0, 0.55];

/// Размер кнопки оверлейного тулбара fullscreen (логические px).
pub const FS_BUTTON: f32 = 44.0;
/// Отступ оверлея fullscreen от края монитора и зазор между его кнопками (логические px).
pub const FS_OVERLAY_PAD: f32 = 16.0;
pub const FS_OVERLAY_GAP: f32 = 8.0;
/// Высота миниатюры карусели (логические px). Ширина — по аспекту фото.
pub const THUMB_H: f32 = 64.0;
/// Аспект-плейсхолдер до загрузки (ширина = высота × AR) и пределы аспекта,
/// чтобы панорамы/узкие кадры не ломали ленту.
pub const THUMB_DEFAULT_AR: f32 = 1.5;
pub const THUMB_MIN_AR: f32 = 0.5;
pub const THUMB_MAX_AR: f32 = 2.4;
/// Толщина рамки активной миниатюры (логические px).
pub const THUMB_BORDER: f32 = 2.0;
/// Скругление миниатюры и зазор между ними (логические px).
pub const THUMB_RADIUS: f32 = 4.0;
pub const THUMB_GAP: f32 = 6.0;
/// Горизонтальный внутренний отступ карусели (логические px).
pub const CAROUSEL_PAD: f32 = 10.0;
/// Кегли текстов bottom bar (логические px).
pub const META_LABEL_SIZE: f32 = 9.0;
pub const META_VALUE_SIZE: f32 = 11.0;
pub const ACTION_ICON_SIZE: f32 = 16.0;
pub const BADGE_FONT_SIZE: f32 = 7.0;

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
    pub badge_bg: [f32; 4],
    pub divider_grip: [f32; 4],
    pub thumb_placeholder: [f32; 4],
    pub active_border: [f32; 4],
    pub overlay_bg: [f32; 4],
    /// Плашка-подложка заголовка группы в EXIF popup (светлее карточки).
    pub popup_group_bg: [f32; 4],
    /// Непрозрачная подложка поля поиска (инпут-бокс; непрозрачна, чтобы рамка фокуса была чистой).
    pub popup_field_bg: [f32; 4],
    /// Подсветка выделения текста в поле поиска.
    pub selection_bg: [f32; 4],
    /// Акцент активной кнопки Save (фон).
    pub save_bg: [f32; 4],
    /// Опасное действие (delete/✕) — цвет иконки.
    pub danger: [f32; 4],
    /// Маркер изменённого значения (pending Set) — цвет текста.
    pub pending_mark: [f32; 4],
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
            badge_bg: rgba(0x00, 0x00, 0x00, 0.70),
            divider_grip: rgba(0x3a, 0x3a, 0x40, 1.0),
            thumb_placeholder: rgba(0x26, 0x26, 0x2c, 1.0),
            active_border: rgba(0xff, 0xff, 0xff, 1.0),
            overlay_bg: rgba(0x00, 0x00, 0x00, 0.45),
            popup_group_bg: rgba(0xff, 0xff, 0xff, 0.08),
            popup_field_bg: rgba(0x2a, 0x2a, 0x30, 1.0),
            selection_bg: rgba(0x4a, 0x9e, 0xff, 0.35),
            save_bg: rgba(0x2f, 0x6f, 0xd6, 1.0),
            danger: rgba(0xe0, 0x5a, 0x4a, 1.0),
            pending_mark: rgba(0x6f, 0xb0, 0x6f, 1.0),
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

    #[test]
    fn bottom_bar_constants_positive() {
        assert!(BOTTOM_BAR_HEIGHT > THUMB_H); // миниатюра помещается + отступы
        assert!(DIVIDER_HEIGHT > 0.0);
        let p = ThemePalette::dark();
        assert!(p.badge_bg[3] < 1.0); // бейдж полупрозрачный
        assert_eq!(p.active_border, [1.0, 1.0, 1.0, 1.0]);
    }
}
