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
pub const GLYPH_ROTATE_CW: char = '\u{EB15}';  // ti-rotate-clockwise
pub const GLYPH_PLAY: char = '\u{ED46}';       // ti-player-play
pub const GLYPH_FS_EXIT: char = '\u{EA29}';    // ti-arrows-minimize (выход из fullscreen)
pub const GLYPH_CHEVRON_LEFT: char = '\u{EA60}';  // ti-chevron-left
pub const GLYPH_CHEVRON_RIGHT: char = '\u{EA61}'; // ti-chevron-right

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

/// Слой прямоугольника относительно миниатюр карусели.
/// `Bg` рисуется ДО миниатюр (подложки), `Overlay` — ПОСЛЕ (рамка/бейджи поверх фото).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RectLayer {
    Bg,
    Overlay,
}

#[derive(Clone, Debug)]
pub enum DrawCmd {
    Rect { rect: Rect, color: [f32; 4], radius: f32, layer: RectLayer },
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

/// Строки мета-панели (без заголовков, в столбик): формат, разрешение, размер файла.
/// Напр.: ["JPG", "1920×1280px", "12.4MB"].
pub fn meta_lines(m: &FileMeta) -> Vec<String> {
    vec![
        m.format_label.clone(),
        format!("{}×{}px", m.width, m.height),
        humanize_bytes(m.bytes),
    ]
}

/// Размер в байтах → человекочитаемо ("12.4MB").
pub fn humanize_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1}MB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.1}KB", b / 1024.0)
    } else {
        format!("{}B", bytes)
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
    /// Прозрачность оверлейного тулбара fullscreen (0..1): 1 при движении курсора,
    /// плавно гаснет до 0 после простоя. Вне fullscreen — 0.
    pub fs_overlay: f32,
    pub meta: Option<FileMeta>,
    pub thumb_count: usize,
    pub active_index: usize,
    pub scroll: f32,
    /// Прозрачность экранных стрелок [prev, next] (0..1): к 1 при hover, иначе к 0.
    pub nav_alpha: [f32; 2],
    /// Можно ли листать prev/next (нет первого/последнего — стрелка скрыта и инертна).
    pub can_prev: bool,
    pub can_next: bool,
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
            fs_overlay: 0.0,
            meta: None,
            thumb_count: 0,
            active_index: 0,
            scroll: 0.0,
            nav_alpha: [0.0, 0.0],
            can_prev: false,
            can_next: false,
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

    // В fullscreen хрома нет, но есть оверлейный тулбар: [play] [выход] справа-сверху.
    // Прозрачность всего тулбара — fs_overlay (показ при движении, плавное гашение).
    if state.fullscreen {
        let a = state.fs_overlay.clamp(0.0, 1.0);
        if a <= 0.01 {
            return cmds; // оверлей погашен — ничего не рисуем (и хит-тест в app заблокирован)
        }
        let ai = theme::ACTION_ICON_SIZE * scale;
        for (rect, glyph, region) in [
            (layout.btn_fs_play, GLYPH_PLAY, Region::SlideshowPlay),
            (layout.btn_fs_exit, GLYPH_FS_EXIT, Region::FullscreenExit),
        ] {
            // полупрозрачная подложка (ярче при hover), умноженная на прозрачность тулбара
            let mut bg = theme.overlay_bg;
            if state.hovered == region {
                bg[3] = (bg[3] + 0.25).min(1.0);
            }
            bg[3] *= a;
            let mut ic = theme.text_primary;
            ic[3] *= a;
            cmds.push(DrawCmd::Rect { rect, color: bg, radius: 6.0 * scale, layer: RectLayer::Bg });
            cmds.push(DrawCmd::Icon { rect, glyph, size: ai, color: ic, font: IconFont::Tabler });
        }
        return cmds;
    }

    // --- Titlebar (как в v0.3a) ---
    cmds.push(DrawCmd::Rect { rect: layout.titlebar, color: theme.bg_surface, radius: 0.0, layer: RectLayer::Bg });
    if state.hovered == Region::Minimize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_min, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
    }
    if state.hovered == Region::Maximize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_max, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
    }
    if state.hovered == Region::Close {
        cmds.push(DrawCmd::Rect { rect: layout.btn_close, color: theme.button_close_hover, radius: 0.0, layer: RectLayer::Bg });
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
    cmds.push(DrawCmd::Rect { rect: layout.divider, color: theme.bg_surface, radius: 0.0, layer: RectLayer::Bg });
    // грип по центру (маленький прямоугольник)
    let grip_w = 60.0 * scale;
    let grip_h = 3.0 * scale;
    let grip = Rect {
        x: layout.divider.x + (layout.divider.w - grip_w) * 0.5,
        y: layout.divider.y + (layout.divider.h - grip_h) * 0.5,
        w: grip_w,
        h: grip_h,
    };
    cmds.push(DrawCmd::Rect { rect: grip, color: theme.divider_grip, radius: grip_h * 0.5, layer: RectLayer::Bg });

    // --- Bottom bar (если хоть немного видим) ---
    if state.bottom_factor > 0.0 {
        cmds.push(DrawCmd::Rect { rect: layout.bottom_bar, color: theme.bg_surface, radius: 0.0, layer: RectLayer::Bg });

        // Мета-панель: формат / разрешение / размер — в столбик, без заголовков.
        if let Some(meta) = &state.meta {
            let line_h = theme::META_VALUE_SIZE * scale * 1.55;
            let mut y = layout.meta.y + 12.0 * scale;
            for value in meta_lines(meta) {
                cmds.push(DrawCmd::Text {
                    rect: Rect { x: layout.meta.x + 12.0 * scale, y, w: layout.meta.w - 14.0 * scale, h: line_h },
                    text: value,
                    size: theme::META_VALUE_SIZE * scale,
                    color: theme.text_primary,
                    align: Align::Left,
                });
                y += line_h;
            }
        }

        // Активная рамка (контур, не заливка — иначе перекрыла бы фото) и бейджи поверх миниатюр
        for (idx, r) in thumb_rects {
            if *idx == state.active_index {
                let t = theme::THUMB_BORDER * scale;
                let c = theme.active_border;
                // четыре тонких прямоугольника по краям миниатюры
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x, y: r.y, w: r.w, h: t }, color: c, radius: 0.0, layer: RectLayer::Overlay });
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x, y: r.y + r.h - t, w: r.w, h: t }, color: c, radius: 0.0, layer: RectLayer::Overlay });
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x, y: r.y, w: t, h: r.h }, color: c, radius: 0.0, layer: RectLayer::Overlay });
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x + r.w - t, y: r.y, w: t, h: r.h }, color: c, radius: 0.0, layer: RectLayer::Overlay });
            }
            if raw_flags.get(*idx).copied().unwrap_or(false) {
                // бейдж формата в правом нижнем углу — фон + текст
                let bw = 22.0 * scale;
                let bh = 11.0 * scale;
                let badge = Rect { x: r.x + r.w - bw - 3.0 * scale, y: r.y + r.h - bh - 3.0 * scale, w: bw, h: bh };
                cmds.push(DrawCmd::Rect { rect: badge, color: theme.badge_bg, radius: 2.0 * scale, layer: RectLayer::Overlay });
            }
        }
        // (текст бейджа — в Task 9, когда есть расширения файлов; здесь только фон-плашка)

        // Кнопки действий: [поворот] [fullscreen] [инфо].
        if state.hovered == Region::ActionRotate {
            cmds.push(DrawCmd::Rect { rect: layout.btn_rotate, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
        }
        if state.hovered == Region::ActionFullscreen {
            cmds.push(DrawCmd::Rect { rect: layout.btn_fullscreen, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
        }
        if state.hovered == Region::ActionExif {
            cmds.push(DrawCmd::Rect { rect: layout.btn_exif, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
        }
        let ai = theme::ACTION_ICON_SIZE * scale;
        // Поворот и EXIF инертны (v0.4) → тусклый цвет; fullscreen активен → яркий.
        cmds.push(DrawCmd::Icon { rect: layout.btn_rotate, glyph: GLYPH_ROTATE_CW, size: ai, color: theme.text_secondary, font: IconFont::Tabler });
        cmds.push(DrawCmd::Icon { rect: layout.btn_fullscreen, glyph: GLYPH_FULLSCREEN, size: ai, color: theme.text_primary, font: IconFont::Tabler });
        cmds.push(DrawCmd::Icon { rect: layout.btn_exif, glyph: GLYPH_INFO, size: ai, color: theme.text_secondary, font: IconFont::Tabler });
    }

    // --- Экранные стрелки навигации (поверх фото; проявляются на hover) ---
    let chev = theme::NAV_CHEVRON_SIZE * scale;
    for (i, (rect, can, glyph)) in [
        (layout.nav_prev, state.can_prev, GLYPH_CHEVRON_LEFT),
        (layout.nav_next, state.can_next, GLYPH_CHEVRON_RIGHT),
    ]
    .into_iter()
    .enumerate()
    {
        let a = state.nav_alpha[i].clamp(0.0, 1.0);
        if !can || a <= 0.01 {
            continue;
        }
        let mut bg = theme.overlay_bg;
        bg[3] *= a;
        let mut ic = theme.text_primary;
        ic[3] *= a;
        cmds.push(DrawCmd::Rect { rect, color: bg, radius: 8.0 * scale, layer: RectLayer::Bg });
        cmds.push(DrawCmd::Icon { rect, glyph, size: chev, color: ic, font: IconFont::Tabler });
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
        let aspects = vec![1.5_f32; state.thumb_count];
        let thumbs = crate::ui::layout::carousel_thumb_rects(layout.carousel, &aspects, state.scroll, 1.0);
        let raw: Vec<bool> = (0..state.thumb_count).map(|i| i % 2 == 0).collect();
        build(&state, &layout, &theme, 1.0, &thumbs, &raw)
    }

    #[test]
    fn nav_arrow_shown_on_hover_when_can_navigate() {
        let cmds = fixture(|s| { s.can_next = true; s.nav_alpha = [0.0, 1.0]; });
        let has_right = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_CHEVRON_RIGHT));
        assert!(has_right);
    }

    #[test]
    fn nav_arrow_hidden_when_cannot_navigate() {
        let cmds = fixture(|s| { s.can_next = false; s.nav_alpha = [0.0, 1.0]; });
        let has_right = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_CHEVRON_RIGHT));
        assert!(!has_right);
    }

    #[test]
    fn nav_arrow_hidden_when_alpha_zero() {
        let cmds = fixture(|s| { s.can_prev = true; s.nav_alpha = [0.0, 0.0]; });
        let has_left = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_CHEVRON_LEFT));
        assert!(!has_left);
    }

    #[test]
    fn fullscreen_emits_only_overlay() {
        let cmds = fixture(|s| { s.fullscreen = true; s.fs_overlay = 1.0; });
        // нет хрома (titlebar bg высотой 32 отсутствует)
        let has_titlebar = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { rect, .. } if rect.h == 32.0));
        assert!(!has_titlebar);
        // ровно 2 оверлейных глифа: play + выход
        let icons: Vec<char> = cmds.iter().filter_map(|c| match c {
            DrawCmd::Icon { glyph, font: IconFont::Tabler, .. } => Some(*glyph),
            _ => None,
        }).collect();
        assert_eq!(icons.len(), 2);
        assert!(icons.contains(&GLYPH_PLAY));
        assert!(icons.contains(&GLYPH_FS_EXIT));
    }

    #[test]
    fn titlebar_present_with_three_window_icons() {
        let cmds = fixture(|_| {});
        let win_icons = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { font: IconFont::WindowMdl2, .. })).count();
        assert_eq!(win_icons, 3);
    }

    #[test]
    fn three_action_icons_tabler() {
        let cmds = fixture(|_| {});
        let tab = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { font: IconFont::Tabler, .. })).count();
        assert_eq!(tab, 3); // поворот + fullscreen + инфо
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
        // bottom bar фон (высота 84) отсутствует
        let bar_bg = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { rect, .. } if rect.h == 84.0));
        assert!(!bar_bg);
        // divider всегда виден — присутствует грип (его цвет)
        let theme = ThemePalette::dark();
        let grip = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { color, .. } if *color == theme.divider_grip));
        assert!(grip);
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
        assert_eq!(lines[0], "RAF · RAW");
        assert_eq!(lines[1], "7728×5200px");
        assert!(lines[2].contains("MB"));
    }

    #[test]
    fn humanize_bytes_units() {
        assert_eq!(humanize_bytes(512), "512B");
        assert!(humanize_bytes(2048).contains("KB"));
        assert!(humanize_bytes(5 * 1024 * 1024).contains("MB"));
    }
}
