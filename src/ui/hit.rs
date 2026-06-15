//! Hit-тест курсора по регионам окна. Чистый, в физических пикселях.

use crate::ui::layout::UiLayout;
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    None,
    Caption,
    Minimize,
    Maximize,
    Close,
    Resize(Edge),
    Divider,
    Carousel,
    Thumbnail(usize),
    ActionFullscreen,
    ActionExif,
}

/// Ширина зоны захвата для ресайза у краёв окна (логические px).
pub const RESIZE_BORDER: f32 = 6.0;

/// Определить регион под курсором.
/// `win` — физический размер окна, `scale` — scale_factor.
pub fn hit(layout: &UiLayout, win: Vec2, cursor: Vec2, scale: f32) -> Region {
    let b = RESIZE_BORDER * scale;
    let left = cursor.x < b;
    let right = cursor.x >= win.x - b;
    let top = cursor.y < b;
    let bottom = cursor.y >= win.y - b;

    // Края/углы имеют приоритет над всем остальным.
    match (left, right, top, bottom) {
        (true, _, true, _) => return Region::Resize(Edge::TopLeft),
        (_, true, true, _) => return Region::Resize(Edge::TopRight),
        (true, _, _, true) => return Region::Resize(Edge::BottomLeft),
        (_, true, _, true) => return Region::Resize(Edge::BottomRight),
        (true, _, _, _) => return Region::Resize(Edge::Left),
        (_, true, _, _) => return Region::Resize(Edge::Right),
        (_, _, true, _) => return Region::Resize(Edge::Top),
        (_, _, _, true) => return Region::Resize(Edge::Bottom),
        _ => {}
    }

    if layout.btn_close.contains(cursor) {
        return Region::Close;
    }
    if layout.btn_max.contains(cursor) {
        return Region::Maximize;
    }
    if layout.btn_min.contains(cursor) {
        return Region::Minimize;
    }
    if layout.titlebar.contains(cursor) {
        return Region::Caption;
    }
    if layout.btn_fullscreen.contains(cursor) {
        return Region::ActionFullscreen;
    }
    if layout.btn_exif.contains(cursor) {
        return Region::ActionExif;
    }
    if layout.divider.contains(cursor) {
        return Region::Divider;
    }
    if layout.carousel.contains(cursor) {
        return Region::Carousel;
    }
    Region::None
}

/// Индекс миниатюры под курсором, если попал по одной из видимых.
pub fn hit_thumbnail(
    carousel: crate::ui::layout::Rect,
    count: usize,
    scroll: f32,
    scale: f32,
    cursor: Vec2,
) -> Option<usize> {
    for (i, r) in crate::ui::layout::carousel_thumb_rects(carousel, count, scroll, scale) {
        if r.contains(cursor) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::compute;

    fn layout() -> UiLayout {
        compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false)
    }

    #[test]
    fn close_button() {
        let win = Vec2::new(1280.0, 800.0);
        // центр close-кнопки: x ≈ 1280-23, y=16
        let r = hit(&layout(), win, Vec2::new(1280.0 - 23.0, 16.0), 1.0);
        assert_eq!(r, Region::Close);
    }

    #[test]
    fn caption_drag_area() {
        let win = Vec2::new(1280.0, 800.0);
        // середина titlebar, далеко от кнопок и краёв
        let r = hit(&layout(), win, Vec2::new(400.0, 16.0), 1.0);
        assert_eq!(r, Region::Caption);
    }

    #[test]
    fn viewer_is_none() {
        let win = Vec2::new(1280.0, 800.0);
        let r = hit(&layout(), win, Vec2::new(640.0, 400.0), 1.0);
        assert_eq!(r, Region::None);
    }

    #[test]
    fn left_edge_resize() {
        let win = Vec2::new(1280.0, 800.0);
        let r = hit(&layout(), win, Vec2::new(2.0, 400.0), 1.0);
        assert_eq!(r, Region::Resize(Edge::Left));
    }

    #[test]
    fn top_left_corner_resize() {
        let win = Vec2::new(1280.0, 800.0);
        let r = hit(&layout(), win, Vec2::new(1.0, 1.0), 1.0);
        assert_eq!(r, Region::Resize(Edge::TopLeft));
    }

    #[test]
    fn edge_priority_over_button() {
        // верхний-правый угол перекрывает close-кнопку → ресайз, не Close
        let win = Vec2::new(1280.0, 800.0);
        let r = hit(&layout(), win, Vec2::new(1279.0, 1.0), 1.0);
        assert_eq!(r, Region::Resize(Edge::TopRight));
    }

    #[test]
    fn fullscreen_button_region() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        // центр кнопки fullscreen
        let c = Vec2::new(l.btn_fullscreen.x + 19.0, l.btn_fullscreen.y + 42.0);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), c, 1.0), Region::ActionFullscreen);
    }

    #[test]
    fn exif_button_region() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let c = Vec2::new(l.btn_exif.x + 19.0, l.btn_exif.y + 42.0);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), c, 1.0), Region::ActionExif);
    }

    #[test]
    fn divider_region() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let c = Vec2::new(400.0, l.divider.y + 11.0);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), c, 1.0), Region::Divider);
    }

    #[test]
    fn carousel_region() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let c = Vec2::new(l.carousel.x + 200.0, l.carousel.y + 42.0);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), c, 1.0), Region::Carousel);
    }

    #[test]
    fn thumbnail_hit_by_index() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        // центр первой миниатюры
        let first = crate::ui::layout::carousel_thumb_rects(l.carousel, 100, 0.0, 1.0)[0].1;
        let c = Vec2::new(first.x + 31.0, first.y + 32.0);
        assert_eq!(hit_thumbnail(l.carousel, 100, 0.0, 1.0, c), Some(0));
        // мимо карусели
        assert_eq!(hit_thumbnail(l.carousel, 100, 0.0, 1.0, Vec2::new(0.0, 0.0)), None);
    }

    #[test]
    fn viewer_still_none() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), Vec2::new(640.0, 300.0), 1.0), Region::None);
    }
}
