//! Раскладка titlebar и viewer-области. Чистая, в ФИЗИЧЕСКИХ пикселях.

use crate::ui::theme;
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UiLayout {
    pub titlebar: Rect,
    pub btn_min: Rect,
    pub btn_max: Rect,
    pub btn_close: Rect,
    pub title: Rect,
    pub viewer: Rect,
    pub divider: Rect,
    pub bottom_bar: Rect,
    pub meta: Rect,
    pub carousel: Rect,
    pub btn_rotate: Rect,
    pub btn_fullscreen: Rect,
    pub btn_exif: Rect,
    /// Кнопки оверлейного тулбара fullscreen (нулевые вне fullscreen).
    pub btn_fs_play: Rect,
    pub btn_fs_exit: Rect,
    /// Тонкие полосы навигации у краёв viewer (нулевые в fullscreen).
    pub nav_prev: Rect,
    pub nav_next: Rect,
}

/// Пустой прямоугольник.
const ZERO: Rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };

/// Посчитать раскладку. `bottom_factor` ∈ [0,1] — анимированная видимость bottom bar.
/// В fullscreen весь хром нулевой, viewer = всё окно.
pub fn compute(win: Vec2, scale: f32, bottom_factor: f32, fullscreen: bool) -> UiLayout {
    if fullscreen {
        // Оверлейный тулбар справа-сверху: [play] [выход].
        let b = theme::FS_BUTTON * scale;
        let pad = theme::FS_OVERLAY_PAD * scale;
        let gap = theme::FS_OVERLAY_GAP * scale;
        let y = pad;
        let exit_x = win.x - pad - b;
        let play_x = exit_x - gap - b;
        return UiLayout {
            titlebar: ZERO,
            btn_min: ZERO,
            btn_max: ZERO,
            btn_close: ZERO,
            title: ZERO,
            viewer: Rect { x: 0.0, y: 0.0, w: win.x, h: win.y },
            divider: ZERO,
            bottom_bar: ZERO,
            meta: ZERO,
            carousel: ZERO,
            btn_rotate: ZERO,
            btn_fullscreen: ZERO,
            btn_exif: ZERO,
            btn_fs_play: Rect { x: play_x, y, w: b, h: b },
            btn_fs_exit: Rect { x: exit_x, y, w: b, h: b },
            nav_prev: ZERO,
            nav_next: ZERO,
        };
    }

    let bar_h = theme::TITLEBAR_HEIGHT * scale;
    let btn_w = theme::BUTTON_WIDTH * scale;

    let titlebar = Rect { x: 0.0, y: 0.0, w: win.x, h: bar_h };
    let btn_close = Rect { x: win.x - btn_w, y: 0.0, w: btn_w, h: bar_h };
    let btn_max = Rect { x: win.x - 2.0 * btn_w, y: 0.0, w: btn_w, h: bar_h };
    let btn_min = Rect { x: win.x - 3.0 * btn_w, y: 0.0, w: btn_w, h: bar_h };
    let title = Rect { x: 0.0, y: 0.0, w: (win.x - 3.0 * btn_w).max(0.0), h: bar_h };

    let div_h = theme::DIVIDER_HEIGHT * scale;
    let bottom_h = theme::BOTTOM_BAR_HEIGHT * scale * bottom_factor.clamp(0.0, 1.0);
    let bottom_bar = Rect { x: 0.0, y: win.y - bottom_h, w: win.x, h: bottom_h };
    let divider = Rect { x: 0.0, y: win.y - bottom_h - div_h, w: win.x, h: div_h };
    let viewer = Rect { x: 0.0, y: bar_h, w: win.x, h: (divider.y - bar_h).max(0.0) };

    let meta_w = theme::META_WIDTH * scale;
    let actions_w = theme::ACTIONS_WIDTH * scale;
    let meta = Rect { x: 0.0, y: bottom_bar.y, w: meta_w, h: bottom_h };
    let carousel = Rect {
        x: meta_w,
        y: bottom_bar.y,
        w: (win.x - meta_w - actions_w).max(0.0),
        h: bottom_h,
    };
    let actions_x = win.x - actions_w;
    let third = actions_w / 3.0;
    let btn_rotate = Rect { x: actions_x, y: bottom_bar.y, w: third, h: bottom_h };
    let btn_fullscreen = Rect { x: actions_x + third, y: bottom_bar.y, w: third, h: bottom_h };
    let btn_exif = Rect { x: actions_x + 2.0 * third, y: bottom_bar.y, w: third, h: bottom_h };

    let nav_w = theme::NAV_ARROW_W * scale;
    let nav_prev = Rect { x: 0.0, y: viewer.y, w: nav_w, h: viewer.h };
    let nav_next = Rect { x: win.x - nav_w, y: viewer.y, w: nav_w, h: viewer.h };

    UiLayout {
        titlebar, btn_min, btn_max, btn_close, title, viewer,
        divider, bottom_bar, meta, carousel,
        btn_rotate, btn_fullscreen, btn_exif,
        btn_fs_play: ZERO, btn_fs_exit: ZERO,
        nav_prev, nav_next,
    }
}

/// Геометрия EXIF popup (центрированная карточка). Всё в физ. px.
#[derive(Clone, Copy, Debug)]
pub struct PopupLayout {
    pub card: Rect,
    pub header: Rect,
    pub close: Rect,
    pub search: Rect,
    pub body: Rect,
    pub footer: Rect,
}

/// Посчитать раскладку popup для размера окна `win` и масштаба `scale`.
pub fn popup_layout(win: Vec2, scale: f32) -> PopupLayout {
    let margin = theme::POPUP_MARGIN * scale;
    let w = (theme::POPUP_MAX_W * scale).min((win.x - 2.0 * margin).max(0.0));
    let h = (theme::POPUP_MAX_H * scale).min((win.y - 2.0 * margin).max(0.0));
    let x = (win.x - w) * 0.5;
    let y = (win.y - h) * 0.5;
    let card = Rect { x, y, w, h };

    let hh = theme::POPUP_HEADER_H * scale;
    let sh = theme::POPUP_SEARCH_H * scale;
    let header = Rect { x, y, w, h: hh };
    let close = Rect { x: x + w - hh, y, w: hh, h: hh }; // квадрат в правом верхнем углу
    let search = Rect { x, y: y + hh, w, h: sh };
    let fh = theme::POPUP_FOOTER_H * scale;
    let body_top = y + hh + sh;
    let footer = Rect { x, y: y + h - fh, w, h: fh };
    let body = Rect { x, y: body_top, w, h: (footer.y - body_top).max(0.0) };

    PopupLayout { card, header, close, search, body, footer }
}

/// Высота одной строки тега и заголовка группы (физ. px) — для скролла/хита.
pub fn popup_row_h(scale: f32) -> f32 {
    theme::POPUP_ROW_H * scale
}
pub fn popup_group_h(scale: f32) -> f32 {
    theme::POPUP_GROUP_H * scale
}

/// Прямоугольники кнопок футера: (save, cancel). Save — правая (primary), cancel — левее.
pub fn popup_footer_buttons(p: &PopupLayout, scale: f32) -> (Rect, Rect) {
    let pad = theme::POPUP_PAD * scale;
    let bh = 28.0 * scale;
    let bw = 96.0 * scale;
    let gap = 8.0 * scale;
    let by = p.footer.y + (p.footer.h - bh) * 0.5;
    let save = Rect { x: p.footer.x + p.footer.w - pad - bw, y: by, w: bw, h: bh };
    let cancel = Rect { x: save.x - gap - bw, y: by, w: bw, h: bh };
    (save, cancel)
}

/// Кликабельная зона чекбокс-тоггла «Необратимо» (левый край футера): бокс + подпись.
pub fn popup_footer_toggle(p: &PopupLayout, scale: f32) -> Rect {
    let pad = theme::POPUP_PAD * scale;
    let h = 20.0 * scale;
    Rect { x: p.footer.x + pad, y: p.footer.y + (p.footer.h - h) * 0.5, w: 150.0 * scale, h }
}

/// Кнопка «Стереть всё» — слева от cancel (показывается только в необратимом режиме).
pub fn popup_footer_strip(p: &PopupLayout, scale: f32) -> Rect {
    let (_save, cancel) = popup_footer_buttons(p, scale);
    let bw = 96.0 * scale;
    let gap = 8.0 * scale;
    Rect { x: cancel.x - gap - bw, y: cancel.y, w: bw, h: cancel.h }
}

/// Физическая ширина миниатюры по аспекту фото (высота фиксирована).
/// `ar` ≤ 0 трактуется как плейсхолдер (фото ещё не загружено).
pub fn thumb_width(ar: f32, scale: f32) -> f32 {
    let ar = if ar > 0.0 { ar } else { theme::THUMB_DEFAULT_AR };
    let ar = ar.clamp(theme::THUMB_MIN_AR, theme::THUMB_MAX_AR);
    theme::THUMB_H * ar * scale
}

/// Прямоугольники видимых миниатюр карусели: (индекс, rect в физ. px).
/// `aspects[i]` — аспект (ширина/высота) i-й миниатюры (≤0 = плейсхолдер).
/// Количество миниатюр = `aspects.len()`. `scroll` — смещение в физ. px.
pub fn carousel_thumb_rects(
    carousel: Rect,
    aspects: &[f32],
    scroll: f32,
    scale: f32,
) -> Vec<(usize, Rect)> {
    let th = theme::THUMB_H * scale;
    let gap = theme::THUMB_GAP * scale;
    let pad = theme::CAROUSEL_PAD * scale;
    let y = carousel.y + (carousel.h - th) * 0.5;
    let mut out = Vec::new();
    let mut x = carousel.x + pad - scroll;
    for (i, &ar) in aspects.iter().enumerate() {
        let tw = thumb_width(ar, scale);
        // включаем только пересекающие зону карусели по горизонтали
        if x + tw > carousel.x && x < carousel.x + carousel.w {
            out.push((i, Rect { x, y, w: tw, h: th }));
        }
        x += tw + gap;
    }
    out
}

/// Полная ширина содержимого карусели (для clamp скролла), физ. px.
pub fn carousel_content_width(aspects: &[f32], scale: f32) -> f32 {
    let n = aspects.len();
    if n == 0 {
        return 0.0;
    }
    let gap = theme::THUMB_GAP * scale;
    let pad = theme::CAROUSEL_PAD * scale;
    let sum: f32 = aspects.iter().map(|&ar| thumb_width(ar, scale)).sum();
    pad * 2.0 + sum + (n - 1) as f32 * gap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titlebar_and_viewer_split_with_bottom() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        assert_eq!(l.titlebar, Rect { x: 0.0, y: 0.0, w: 1280.0, h: 32.0 });
        // bottom bar 84, divider 22 снизу; viewer между titlebar(32) и divider
        assert_eq!(l.bottom_bar, Rect { x: 0.0, y: 800.0 - 84.0, w: 1280.0, h: 84.0 });
        assert_eq!(l.divider, Rect { x: 0.0, y: 800.0 - 84.0 - 22.0, w: 1280.0, h: 22.0 });
        assert_eq!(l.viewer, Rect { x: 0.0, y: 32.0, w: 1280.0, h: 800.0 - 32.0 - 84.0 - 22.0 });
    }

    #[test]
    fn bottom_hidden_extends_viewer_keeps_divider() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 0.0, false);
        assert_eq!(l.bottom_bar.h, 0.0);
        // divider прижат к низу
        assert_eq!(l.divider.y, 800.0 - 22.0);
        // viewer тянется до divider
        assert_eq!(l.viewer.h, 800.0 - 32.0 - 22.0);
    }

    #[test]
    fn fullscreen_zeroes_chrome() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, true);
        assert_eq!(l.titlebar, ZERO);
        assert_eq!(l.divider, ZERO);
        assert_eq!(l.bottom_bar, ZERO);
        assert_eq!(l.viewer, Rect { x: 0.0, y: 0.0, w: 1280.0, h: 800.0 });
    }

    #[test]
    fn nav_arrows_geometry_windowed() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        assert_eq!(l.nav_prev.x, 0.0);
        assert_eq!(l.nav_prev.w, 44.0);
        assert_eq!(l.nav_prev.y, l.viewer.y);
        assert_eq!(l.nav_prev.h, l.viewer.h);
        assert_eq!(l.nav_next.x, 1280.0 - 44.0);
        assert_eq!(l.nav_next.w, 44.0);
        assert_eq!(l.nav_next.y, l.viewer.y);
        assert_eq!(l.nav_next.h, l.viewer.h);
    }

    #[test]
    fn nav_arrows_zero_in_fullscreen() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, true);
        assert_eq!(l.nav_prev, ZERO);
        assert_eq!(l.nav_next, ZERO);
    }

    #[test]
    fn bottom_zones_layout() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        assert_eq!(l.meta.w, 132.0);
        assert_eq!(l.meta.x, 0.0);
        assert_eq!(l.carousel.x, 132.0);
        assert_eq!(l.carousel.w, 1280.0 - 132.0 - 114.0);
        // три кнопки по 38 в правой зоне 114: поворот/fullscreen/инфо
        assert_eq!(l.btn_rotate.x, 1280.0 - 114.0);
        assert_eq!(l.btn_rotate.w, 38.0);
        assert_eq!(l.btn_fullscreen.x, 1280.0 - 76.0);
        assert_eq!(l.btn_exif.x, 1280.0 - 38.0);
        // оверлейные кнопки fullscreen вне fullscreen — нулевые
        assert_eq!(l.btn_fs_exit, ZERO);
        assert_eq!(l.btn_fs_play, ZERO);
    }

    #[test]
    fn fullscreen_overlay_buttons_top_right() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, true);
        // [play] [exit] справа-сверху, размер 44, отступ 16, зазор 8
        assert_eq!(l.btn_fs_exit, Rect { x: 1280.0 - 16.0 - 44.0, y: 16.0, w: 44.0, h: 44.0 });
        assert_eq!(l.btn_fs_play, Rect { x: 1280.0 - 16.0 - 44.0 - 8.0 - 44.0, y: 16.0, w: 44.0, h: 44.0 });
        // прочий хром нулевой
        assert_eq!(l.btn_rotate, ZERO);
    }

    #[test]
    fn half_factor_halves_bottom_height() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 0.5, false);
        assert_eq!(l.bottom_bar.h, 42.0);
    }

    #[test]
    fn scale_factor_doubles_sizes() {
        let l = compute(Vec2::new(2560.0, 1600.0), 2.0, 1.0, false);
        assert_eq!(l.titlebar.h, 64.0);
        assert_eq!(l.bottom_bar.h, 168.0);
        assert_eq!(l.divider.h, 44.0);
    }

    #[test]
    fn carousel_thumbs_positions_and_visibility() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let aspects = vec![1.5_f32; 100]; // дефолт-аспект → ширина 64*1.5 = 96
        let rects = carousel_thumb_rects(l.carousel, &aspects, 0.0, 1.0);
        // первая миниатюра: x = carousel.x + pad
        assert_eq!(rects[0].0, 0);
        assert_eq!(rects[0].1.x, l.carousel.x + 10.0);
        assert_eq!(rects[0].1.w, 96.0);
        assert_eq!(rects[0].1.h, 64.0);
        // далеко за правым краем — не входят (видимых заметно меньше 100)
        assert!(rects.len() < 100);
    }

    #[test]
    fn carousel_scroll_shifts_left() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let aspects = vec![1.5_f32; 100];
        let a = carousel_thumb_rects(l.carousel, &aspects, 0.0, 1.0);
        let b = carousel_thumb_rects(l.carousel, &aspects, 102.0, 1.0); // tw+gap = 96+6
        // при скролле на один шаг индекс 1 встаёт примерно туда, где был индекс 0
        assert!((b.iter().find(|(i, _)| *i == 1).unwrap().1.x - a[0].1.x).abs() < 0.01);
    }

    #[test]
    fn variable_width_by_aspect() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        // ландшафт (2.0) шире портрета (0.6)
        let aspects = vec![2.0_f32, 0.6_f32];
        let rects = carousel_thumb_rects(l.carousel, &aspects, 0.0, 1.0);
        assert_eq!(rects[0].1.w, 128.0); // 64 * 2.0
        assert!((rects[1].1.w - 64.0 * 0.6).abs() < 0.01);
        // второй сдвинут на ширину первого + gap
        assert!((rects[1].1.x - (rects[0].1.x + 128.0 + 6.0)).abs() < 0.01);
    }

    #[test]
    fn content_width_grows_with_count() {
        assert_eq!(carousel_content_width(&[], 1.0), 0.0);
        let w1 = carousel_content_width(&[1.5], 1.0);
        let w2 = carousel_content_width(&[1.5, 1.5], 1.0);
        assert!(w2 > w1);
        // 1 миниатюра: pad*2 + tw = 20 + 96 = 116
        assert_eq!(w1, 116.0);
    }

    #[test]
    fn popup_centered_and_zoned() {
        let win = Vec2::new(1280.0, 800.0);
        let p = popup_layout(win, 1.0);
        // карточка центрирована
        assert!((p.card.x + p.card.w * 0.5 - 640.0).abs() < 0.5);
        assert!((p.card.y + p.card.h * 0.5 - 400.0).abs() < 0.5);
        // заголовок сверху карточки
        assert_eq!(p.header.y, p.card.y);
        assert_eq!(p.header.h, 40.0);
        // поиск под заголовком
        assert_eq!(p.search.y, p.card.y + 40.0);
        assert_eq!(p.search.h, 34.0);
        // тело под поиском, до верха футера
        assert_eq!(p.body.y, p.card.y + 40.0 + 34.0);
        assert!((p.body.y + p.body.h - p.footer.y).abs() < 0.5);
        // close-кнопка в правом верхнем углу карточки
        assert!((p.close.x + p.close.w - (p.card.x + p.card.w)).abs() < 0.5);
    }

    #[test]
    fn popup_has_footer_and_body_shrinks() {
        let win = Vec2::new(1280.0, 800.0);
        let p = popup_layout(win, 1.0);
        // футер внизу карточки
        assert!((p.footer.y + p.footer.h - (p.card.y + p.card.h)).abs() < 0.5);
        assert_eq!(p.footer.h, 46.0);
        // тело заканчивается на верхе футера
        assert!((p.body.y + p.body.h - p.footer.y).abs() < 0.5);
        // кнопки футера внутри футера, save правее cancel
        let (save, cancel) = popup_footer_buttons(&p, 1.0);
        assert!(save.x > cancel.x);
        assert!(save.y >= p.footer.y - 0.5 && save.y + save.h <= p.footer.y + p.footer.h + 0.5);
    }

    #[test]
    fn popup_clamps_to_small_window() {
        let win = Vec2::new(400.0, 300.0);
        let p = popup_layout(win, 1.0);
        // карточка влезает в окно с зазором POPUP_MARGIN(40) по краям
        assert!(p.card.x >= 39.0);
        assert!(p.card.w <= 400.0 - 2.0 * 40.0 + 0.5);
        assert!(p.card.h <= 300.0 - 2.0 * 40.0 + 0.5);
    }

    #[test]
    fn popup_footer_toggle_and_strip_geometry() {
        let win = Vec2::new(1280.0, 800.0);
        let p = popup_layout(win, 1.0);
        let tog = popup_footer_toggle(&p, 1.0);
        let strip = popup_footer_strip(&p, 1.0);
        let (_save, cancel) = popup_footer_buttons(&p, 1.0);
        // тоггл у левого края футера
        assert!((tog.x - (p.footer.x + 12.0)).abs() < 0.5);
        // strip слева от cancel и правее тоггла (без наложения)
        assert!(strip.x < cancel.x);
        assert!(strip.x > tog.x + tog.w);
        // внутри футера по вертикали
        assert!(strip.y >= p.footer.y - 0.5 && strip.y + strip.h <= p.footer.y + p.footer.h + 0.5);
    }

    #[test]
    fn rect_contains() {
        let r = Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 };
        assert!(r.contains(Vec2::new(15.0, 15.0)));
        assert!(!r.contains(Vec2::new(30.0, 15.0)));
        assert!(!r.contains(Vec2::new(5.0, 15.0)));
    }
}
