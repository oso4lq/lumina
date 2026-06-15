//! Построение списка draw-команд titlebar из состояния. Чистое, без GPU.

use crate::ui::hit::Region;
use crate::ui::layout::{Rect, UiLayout};
use crate::ui::theme::{self, ThemePalette};

/// Глифы Segoe MDL2 Assets для кнопок окна.
pub const GLYPH_MINIMIZE: char = '\u{E921}'; // ChromeMinimize
pub const GLYPH_MAXIMIZE: char = '\u{E922}'; // ChromeMaximize
pub const GLYPH_RESTORE: char = '\u{E923}';  // ChromeRestore
pub const GLYPH_CLOSE: char = '\u{E8BB}';    // ChromeClose

/// Семейство шрифта для глифов кнопок окна.
pub const ICON_FONT_FAMILY: &str = "Segoe MDL2 Assets";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
}

#[derive(Clone, Debug)]
pub enum DrawCmd {
    Rect { rect: Rect, color: [f32; 4], radius: f32 },
    Text { rect: Rect, text: String, size: f32, color: [f32; 4], align: Align },
    Icon { rect: Rect, glyph: char, size: f32, color: [f32; 4] },
}

/// Состояние UI для рендера titlebar.
pub struct UiState {
    pub title: String,
    pub hovered: Region,
    pub maximized: bool,
}

impl UiState {
    pub fn new() -> Self {
        Self { title: String::new(), hovered: Region::None, maximized: false }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Построить draw-команды titlebar. `scale` — для перевода кеглей в физ. px.
pub fn build(
    state: &UiState,
    layout: &UiLayout,
    theme: &ThemePalette,
    scale: f32,
) -> Vec<DrawCmd> {
    let mut cmds = Vec::new();

    // Фон titlebar
    cmds.push(DrawCmd::Rect { rect: layout.titlebar, color: theme.bg_surface, radius: 0.0 });

    // Hover-подложки кнопок
    if state.hovered == Region::Minimize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_min, color: theme.button_hover, radius: 0.0 });
    }
    if state.hovered == Region::Maximize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_max, color: theme.button_hover, radius: 0.0 });
    }
    if state.hovered == Region::Close {
        cmds.push(DrawCmd::Rect {
            rect: layout.btn_close,
            color: theme.button_close_hover,
            radius: 0.0,
        });
    }

    // Заголовок по центру
    cmds.push(DrawCmd::Text {
        rect: layout.title,
        text: state.title.clone(),
        size: theme::TITLE_FONT_SIZE * scale,
        color: theme.text_primary,
        align: Align::Center,
    });

    // Глифы кнопок
    let icon = theme::ICON_FONT_SIZE * scale;
    cmds.push(DrawCmd::Icon {
        rect: layout.btn_min,
        glyph: GLYPH_MINIMIZE,
        size: icon,
        color: theme.text_primary,
    });
    cmds.push(DrawCmd::Icon {
        rect: layout.btn_max,
        glyph: if state.maximized { GLYPH_RESTORE } else { GLYPH_MAXIMIZE },
        size: icon,
        color: theme.text_primary,
    });
    cmds.push(DrawCmd::Icon {
        rect: layout.btn_close,
        glyph: GLYPH_CLOSE,
        size: icon,
        color: theme.text_primary,
    });

    cmds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::compute;
    use glam::Vec2;

    fn fixture(hovered: Region, maximized: bool, title: &str) -> Vec<DrawCmd> {
        let layout = compute(Vec2::new(1280.0, 800.0), 1.0);
        let theme = ThemePalette::dark();
        let state = UiState { title: title.into(), hovered, maximized };
        build(&state, &layout, &theme, 1.0)
    }

    #[test]
    fn always_has_bg_title_and_three_icons() {
        let cmds = fixture(Region::None, false, "a.jpg · Lumina");
        let icons = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { .. })).count();
        let texts = cmds.iter().filter(|c| matches!(c, DrawCmd::Text { .. })).count();
        assert_eq!(icons, 3);
        assert_eq!(texts, 1);
        // первая команда — фон titlebar
        assert!(matches!(cmds[0], DrawCmd::Rect { radius, .. } if radius == 0.0));
    }

    #[test]
    fn close_hover_adds_red_backplate() {
        let none = fixture(Region::None, false, "x");
        let hover = fixture(Region::Close, false, "x");
        assert_eq!(hover.len(), none.len() + 1); // добавилась подложка
    }

    #[test]
    fn maximized_uses_restore_glyph() {
        let cmds = fixture(Region::None, true, "x");
        let has_restore = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_RESTORE));
        assert!(has_restore);
    }

    #[test]
    fn title_text_centered() {
        let cmds = fixture(Region::None, false, "hello");
        let t = cmds.iter().find_map(|c| match c {
            DrawCmd::Text { align, text, .. } => Some((*align, text.clone())),
            _ => None,
        });
        assert_eq!(t, Some((Align::Center, "hello".to_string())));
    }
}
