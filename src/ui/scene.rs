//! Построение списка draw-команд titlebar и bottom bar из состояния. Чистое, без GPU.

use crate::ui::hit::Region;
use crate::ui::layout::{Rect, UiLayout};
use crate::ui::theme::{self, ThemePalette};

/// Глифы Segoe MDL2 Assets для кнопок окна.
pub const GLYPH_MINIMIZE: char = '\u{E921}'; // ChromeMinimize
pub const GLYPH_MAXIMIZE: char = '\u{E922}'; // ChromeMaximize
pub const GLYPH_RESTORE: char = '\u{E923}';  // ChromeRestore
pub const GLYPH_CLOSE: char = '\u{E8BB}';    // ChromeClose

/// Семейство шрифта кнопок окна.
pub const ICON_FONT_FAMILY: &str = "Segoe MDL2 Assets";

/// Семейство шрифта иконок действий (Tabler Icons 3.44.0).
pub const TABLER_FONT_FAMILY: &str = "tabler-icons";

/// Глифы Tabler (кодпоинты из tabler-icons.css 3.44.0).
pub const GLYPH_FULLSCREEN: char = '\u{EAEA}'; // ti-maximize
pub const GLYPH_INFO: char = '\u{EAC5}';       // ti-info-circle

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconFont {
    WindowMdl2,
    Tabler,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
}

#[derive(Clone, Debug)]
pub enum DrawCmd {
    Rect { rect: Rect, color: [f32; 4], radius: f32 },
    Text { rect: Rect, text: String, size: f32, color: [f32; 4], align: Align },
    Icon { rect: Rect, glyph: char, size: f32, color: [f32; 4], font: IconFont },
}

/// Мета-информация о файле (из заголовка/ФС, не из EXIF).
#[derive(Clone, Debug, PartialEq)]
pub struct FileMeta {
    pub format_label: String, // "RAF · RAW"
    pub megapixels: f32,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

/// Строки мета-панели: (label, value).
pub fn meta_lines(m: &FileMeta) -> Vec<(String, String)> {
    vec![
        ("ФОРМАТ".to_string(), m.format_label.clone()),
        (
            "РАЗМЕР".to_string(),
            format!(
                "{:.1} MP\n{} × {}\n{}",
                m.megapixels,
                m.width,
                m.height,
                humanize_bytes(m.bytes)
            ),
        ),
    ]
}

/// Размер в байтах → человекочитаемо ("38.4 МБ").
pub fn humanize_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} МБ", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.1} КБ", b / 1024.0)
    } else {
        format!("{} Б", bytes)
    }
}

/// Состояние UI для рендера titlebar и bottom bar.
pub struct UiState {
    pub title: String,
    pub hovered: Region,
    pub maximized: bool,
    pub bottom_visible: bool, // цель toggle
    pub bottom_factor: f32,   // анимированная видимость 0..1
    pub fullscreen: bool,
    pub meta: Option<FileMeta>,
    pub thumb_count: usize,
    pub active_index: usize,
    pub scroll: f32,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            hovered: Region::None,
            maximized: false,
            bottom_visible: true,
            bottom_factor: 1.0,
            fullscreen: false,
            meta: None,
            thumb_count: 0,
            active_index: 0,
            scroll: 0.0,
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Построить draw-команды. `scale` — для перевода кеглей в физ. px.
/// Активная рамка миниатюры и бейджи — поверх миниатюр (рисуются позже текстуры в рендере).
pub fn build(
    state: &UiState,
    layout: &UiLayout,
    theme: &ThemePalette,
    scale: f32,
    thumb_rects: &[(usize, Rect)],
    raw_flags: &[bool],
) -> Vec<DrawCmd> {
    let mut cmds = Vec::new();

    // В fullscreen хрома нет — только фон viewer задаёт clear, команд не нужно.
    if state.fullscreen {
        return cmds;
    }

    // --- Titlebar (как в v0.3a) ---
    cmds.push(DrawCmd::Rect { rect: layout.titlebar, color: theme.bg_surface, radius: 0.0 });
    if state.hovered == Region::Minimize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_min, color: theme.button_hover, radius: 0.0 });
    }
    if state.hovered == Region::Maximize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_max, color: theme.button_hover, radius: 0.0 });
    }
    if state.hovered == Region::Close {
        cmds.push(DrawCmd::Rect { rect: layout.btn_close, color: theme.button_close_hover, radius: 0.0 });
    }
    cmds.push(DrawCmd::Text {
        rect: layout.title,
        text: state.title.clone(),
        size: theme::TITLE_FONT_SIZE * scale,
        color: theme.text_primary,
        align: Align::Center,
    });
    let icon = theme::ICON_FONT_SIZE * scale;
    cmds.push(DrawCmd::Icon { rect: layout.btn_min, glyph: GLYPH_MINIMIZE, size: icon, color: theme.text_primary, font: IconFont::WindowMdl2 });
    cmds.push(DrawCmd::Icon {
        rect: layout.btn_max,
        glyph: if state.maximized { GLYPH_RESTORE } else { GLYPH_MAXIMIZE },
        size: icon,
        color: theme.text_primary,
        font: IconFont::WindowMdl2,
    });
    cmds.push(DrawCmd::Icon { rect: layout.btn_close, glyph: GLYPH_CLOSE, size: icon, color: theme.text_primary, font: IconFont::WindowMdl2 });

    // --- Divider ---
    cmds.push(DrawCmd::Rect { rect: layout.divider, color: theme.bg_surface, radius: 0.0 });
    // грип по центру (маленький прямоугольник)
    let grip_w = 60.0 * scale;
    let grip_h = 3.0 * scale;
    let grip = Rect {
        x: layout.divider.x + (layout.divider.w - grip_w) * 0.5,
        y: layout.divider.y + (layout.divider.h - grip_h) * 0.5,
        w: grip_w,
        h: grip_h,
    };
    cmds.push(DrawCmd::Rect { rect: grip, color: theme.divider_grip, radius: grip_h * 0.5 });
    // метка «карусель» слева с альфой по bottom_factor
    let mut label_color = theme.text_secondary;
    label_color[3] = state.bottom_factor;
    cmds.push(DrawCmd::Text {
        rect: Rect { x: layout.divider.x + 12.0 * scale, y: layout.divider.y, w: 120.0 * scale, h: layout.divider.h },
        text: "карусель".to_string(),
        size: theme::META_LABEL_SIZE * scale,
        color: label_color,
        align: Align::Left,
    });

    // --- Bottom bar (если хоть немного видим) ---
    if state.bottom_factor > 0.0 {
        cmds.push(DrawCmd::Rect { rect: layout.bottom_bar, color: theme.bg_surface, radius: 0.0 });

        // Мета-панель
        if let Some(meta) = &state.meta {
            let mut y = layout.meta.y + 10.0 * scale;
            for (label, value) in meta_lines(meta) {
                cmds.push(DrawCmd::Text {
                    rect: Rect { x: layout.meta.x + 12.0 * scale, y, w: layout.meta.w - 16.0 * scale, h: theme::META_LABEL_SIZE * scale * 1.4 },
                    text: label,
                    size: theme::META_LABEL_SIZE * scale,
                    color: theme.text_secondary,
                    align: Align::Left,
                });
                y += theme::META_LABEL_SIZE * scale * 1.6;
                // значение может быть многострочным (\n) — glyphon разложит по высоте rect
                let lines = value.matches('\n').count() as f32 + 1.0;
                cmds.push(DrawCmd::Text {
                    rect: Rect { x: layout.meta.x + 12.0 * scale, y, w: layout.meta.w - 16.0 * scale, h: theme::META_VALUE_SIZE * scale * 1.3 * lines },
                    text: value,
                    size: theme::META_VALUE_SIZE * scale,
                    color: theme.text_primary,
                    align: Align::Left,
                });
                y += theme::META_VALUE_SIZE * scale * 1.3 * lines + 6.0 * scale;
            }
        }

        // Активная рамка и бейджи поверх миниатюр
        for (idx, r) in thumb_rects {
            if *idx == state.active_index {
                cmds.push(DrawCmd::Rect { rect: *r, color: theme.active_border, radius: theme::THUMB_RADIUS * scale });
            }
            if raw_flags.get(*idx).copied().unwrap_or(false) {
                // бейдж формата в правом нижнем углу — фон + текст
                let bw = 22.0 * scale;
                let bh = 11.0 * scale;
                let badge = Rect { x: r.x + r.w - bw - 3.0 * scale, y: r.y + r.h - bh - 3.0 * scale, w: bw, h: bh };
                cmds.push(DrawCmd::Rect { rect: badge, color: theme.badge_bg, radius: 2.0 * scale });
            }
        }
        // (текст бейджа — в Task 9, когда есть расширения файлов; здесь только фон-плашка)

        // Кнопки действий
        if state.hovered == Region::ActionFullscreen {
            cmds.push(DrawCmd::Rect { rect: layout.btn_fullscreen, color: theme.button_hover, radius: 0.0 });
        }
        if state.hovered == Region::ActionExif {
            cmds.push(DrawCmd::Rect { rect: layout.btn_exif, color: theme.button_hover, radius: 0.0 });
        }
        let ai = theme::ACTION_ICON_SIZE * scale;
        cmds.push(DrawCmd::Icon { rect: layout.btn_fullscreen, glyph: GLYPH_FULLSCREEN, size: ai, color: theme.text_primary, font: IconFont::Tabler });
        // EXIF инертна → тусклый цвет
        cmds.push(DrawCmd::Icon { rect: layout.btn_exif, glyph: GLYPH_INFO, size: ai, color: theme.text_secondary, font: IconFont::Tabler });
    }

    cmds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::compute;
    use glam::Vec2;

    fn fixture(f: impl FnOnce(&mut UiState)) -> Vec<DrawCmd> {
        let layout = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let theme = ThemePalette::dark();
        let mut state = UiState::new();
        state.title = "a.jpg · Lumina".into();
        f(&mut state);
        let thumbs = crate::ui::layout::carousel_thumb_rects(layout.carousel, state.thumb_count, state.scroll, 1.0);
        let raw: Vec<bool> = (0..state.thumb_count).map(|i| i % 2 == 0).collect();
        build(&state, &layout, &theme, 1.0, &thumbs, &raw)
    }

    #[test]
    fn fullscreen_emits_no_chrome() {
        let cmds = fixture(|s| s.fullscreen = true);
        assert!(cmds.is_empty());
    }

    #[test]
    fn titlebar_present_with_three_window_icons() {
        let cmds = fixture(|_| {});
        let win_icons = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { font: IconFont::WindowMdl2, .. })).count();
        assert_eq!(win_icons, 3);
    }

    #[test]
    fn two_action_icons_tabler() {
        let cmds = fixture(|_| {});
        let tab = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { font: IconFont::Tabler, .. })).count();
        assert_eq!(tab, 2);
    }

    #[test]
    fn exif_icon_is_dimmer_than_fullscreen() {
        let cmds = fixture(|_| {});
        let mut fs = None;
        let mut ex = None;
        for c in &cmds {
            if let DrawCmd::Icon { glyph, color, font: IconFont::Tabler, .. } = c {
                if *glyph == GLYPH_FULLSCREEN { fs = Some(*color); }
                if *glyph == GLYPH_INFO { ex = Some(*color); }
            }
        }
        // EXIF использует text_secondary (тусклее), fullscreen — text_primary
        assert_ne!(fs.unwrap(), ex.unwrap());
    }

    #[test]
    fn hidden_bottom_keeps_divider_no_bar_bg() {
        let cmds = fixture(|s| { s.bottom_factor = 0.0; s.bottom_visible = false; });
        // divider фон есть, bottom bar фон — нет
        let bar_bg = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { rect, .. } if rect.h == 84.0));
        assert!(!bar_bg);
        // метка «карусель» присутствует (divider всегда)
        let label = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "карусель"));
        assert!(label);
    }

    #[test]
    fn active_thumb_border_present_when_visible() {
        let cmds = fixture(|s| { s.thumb_count = 10; s.active_index = 0; });
        let theme = ThemePalette::dark();
        let border = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { color, .. } if *color == theme.active_border));
        assert!(border);
    }

    #[test]
    fn meta_lines_format() {
        let m = FileMeta { format_label: "RAF · RAW".into(), megapixels: 40.23, width: 7728, height: 5200, bytes: 40_265_318 };
        let lines = meta_lines(&m);
        assert_eq!(lines[0], ("ФОРМАТ".to_string(), "RAF · RAW".to_string()));
        assert!(lines[1].1.contains("40.2 MP"));
        assert!(lines[1].1.contains("7728 × 5200"));
        assert!(lines[1].1.contains("МБ"));
    }

    #[test]
    fn humanize_bytes_units() {
        assert_eq!(humanize_bytes(512), "512 Б");
        assert!(humanize_bytes(2048).contains("КБ"));
        assert!(humanize_bytes(5 * 1024 * 1024).contains("МБ"));
    }
}
