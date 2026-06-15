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
}

/// Посчитать раскладку из физического размера окна и scale_factor.
pub fn compute(win: Vec2, scale: f32) -> UiLayout {
    let bar_h = theme::TITLEBAR_HEIGHT * scale;
    let btn_w = theme::BUTTON_WIDTH * scale;

    let titlebar = Rect { x: 0.0, y: 0.0, w: win.x, h: bar_h };
    // кнопки прижаты вправо: close — крайняя правая, затем max, затем min
    let btn_close = Rect { x: win.x - btn_w, y: 0.0, w: btn_w, h: bar_h };
    let btn_max = Rect { x: win.x - 2.0 * btn_w, y: 0.0, w: btn_w, h: bar_h };
    let btn_min = Rect { x: win.x - 3.0 * btn_w, y: 0.0, w: btn_w, h: bar_h };
    // зона заголовка — между левым краем и кнопками
    let title = Rect { x: 0.0, y: 0.0, w: (win.x - 3.0 * btn_w).max(0.0), h: bar_h };
    let viewer = Rect { x: 0.0, y: bar_h, w: win.x, h: (win.y - bar_h).max(0.0) };

    UiLayout { titlebar, btn_min, btn_max, btn_close, title, viewer }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titlebar_and_viewer_split() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0);
        assert_eq!(l.titlebar, Rect { x: 0.0, y: 0.0, w: 1280.0, h: 32.0 });
        assert_eq!(l.viewer, Rect { x: 0.0, y: 32.0, w: 1280.0, h: 768.0 });
    }

    #[test]
    fn buttons_right_aligned_close_last() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0);
        assert_eq!(l.btn_close.x, 1280.0 - 46.0);
        assert_eq!(l.btn_max.x, 1280.0 - 92.0);
        assert_eq!(l.btn_min.x, 1280.0 - 138.0);
        assert_eq!(l.btn_close.w, 46.0);
    }

    #[test]
    fn scale_factor_doubles_sizes() {
        let l = compute(Vec2::new(2560.0, 1600.0), 2.0);
        assert_eq!(l.titlebar.h, 64.0);
        assert_eq!(l.btn_close.w, 92.0);
        assert_eq!(l.viewer.y, 64.0);
    }

    #[test]
    fn rect_contains() {
        let r = Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 };
        assert!(r.contains(Vec2::new(15.0, 15.0)));
        assert!(!r.contains(Vec2::new(30.0, 15.0))); // правая граница исключена
        assert!(!r.contains(Vec2::new(5.0, 15.0)));
    }
}
