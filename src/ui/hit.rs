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
    ActionRotate,
    ActionFullscreen,
    ActionExif,
    FullscreenExit,
    SlideshowPlay,
    NavPrev,
    NavNext,
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
    // Оверлейные кнопки fullscreen (нулевые вне fullscreen — contains() ложно).
    if layout.btn_fs_exit.contains(cursor) {
        return Region::FullscreenExit;
    }
    if layout.btn_fs_play.contains(cursor) {
        return Region::SlideshowPlay;
    }
    if layout.btn_rotate.contains(cursor) {
        return Region::ActionRotate;
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
    if layout.nav_prev.contains(cursor) {
        return Region::NavPrev;
    }
    if layout.nav_next.contains(cursor) {
        return Region::NavNext;
    }
    Region::None
}

/// Регион внутри открытого EXIF popup. `Outside` — клик вне карточки (закрыть).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupRegion {
    Close,
    Search,
    Body,
    Outside,
    RowEdit(usize),    // индекс в popup_rows
    RowDelete(usize),
    GpsDeleteAll(usize),
    FooterSave,
    FooterCancel,
    ConfirmSave,
    ConfirmDiscard,
    ConfirmKeep,
}

/// Хит-тест popup. Вызывается app'ом ПЕРВЫМ, когда popup открыт.
pub fn hit_popup(win: Vec2, scale: f32, cursor: Vec2) -> PopupRegion {
    let p = crate::ui::layout::popup_layout(win, scale);
    if p.close.contains(cursor) {
        return PopupRegion::Close;
    }
    if p.search.contains(cursor) {
        return PopupRegion::Search;
    }
    if p.body.contains(cursor) {
        return PopupRegion::Body;
    }
    PopupRegion::Outside
}

/// Хит-тест popup в режиме редактирования (часть 2): футер, действия строк, GPS-delete-all.
/// `confirm` — показан ли бар подтверждения закрытия (тогда футер = три кнопки confirm).
/// Приоритет: confirm/футер → действия строки → Search → Body → Close → Outside.
#[allow(clippy::too_many_arguments)]
pub fn hit_popup_edit(
    win: Vec2,
    scale: f32,
    cursor: Vec2,
    tags: &crate::exif::tags::ExifTags,
    filter: &str,
    scroll: f32,
    confirm: bool,
) -> PopupRegion {
    let p = crate::ui::layout::popup_layout(win, scale);
    // close-кнопка заголовка
    if p.close.contains(cursor) {
        return PopupRegion::Close;
    }
    // футер / подтверждение
    if p.footer.contains(cursor) {
        let (save, cancel) = crate::ui::layout::popup_footer_buttons(&p, scale);
        if confirm {
            // три кнопки: используем те же save/cancel + третья слева (Keep)
            if save.contains(cursor) {
                return PopupRegion::ConfirmSave;
            }
            if cancel.contains(cursor) {
                return PopupRegion::ConfirmDiscard;
            }
            return PopupRegion::ConfirmKeep;
        }
        if save.contains(cursor) {
            return PopupRegion::FooterSave;
        }
        if cancel.contains(cursor) {
            return PopupRegion::FooterCancel;
        }
        return PopupRegion::Body; // клики по пустому футеру не закрывают
    }
    if p.search.contains(cursor) {
        return PopupRegion::Search;
    }
    if p.body.contains(cursor) {
        // действия строк
        let rows = crate::ui::scene::popup_rows(tags, filter, scale, scroll, p.body);
        for (i, r) in rows.iter().enumerate() {
            if cursor.y < r.y || cursor.y >= r.y + r.h {
                continue;
            }
            match r.kind {
                crate::ui::scene::PopupRowKind::Group if r.group == "GPS" => {
                    let (_e, del) = crate::ui::scene::popup_row_actions(r, p.body, scale);
                    if del.contains(cursor) {
                        return PopupRegion::GpsDeleteAll(i);
                    }
                }
                crate::ui::scene::PopupRowKind::Tag if r.editable => {
                    let (edit, del) = crate::ui::scene::popup_row_actions(r, p.body, scale);
                    if edit.contains(cursor) {
                        return PopupRegion::RowEdit(i);
                    }
                    if del.contains(cursor) {
                        return PopupRegion::RowDelete(i);
                    }
                }
                _ => {}
            }
        }
        return PopupRegion::Body;
    }
    PopupRegion::Outside
}

/// Индекс миниатюры под курсором, если попал по одной из видимых.
pub fn hit_thumbnail(
    carousel: crate::ui::layout::Rect,
    aspects: &[f32],
    scroll: f32,
    scale: f32,
    cursor: Vec2,
) -> Option<usize> {
    for (i, r) in crate::ui::layout::carousel_thumb_rects(carousel, aspects, scroll, scale) {
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
        let aspects = vec![1.5_f32; 100];
        // центр первой миниатюры
        let first = crate::ui::layout::carousel_thumb_rects(l.carousel, &aspects, 0.0, 1.0)[0].1;
        let c = Vec2::new(first.x + 31.0, first.y + 32.0);
        assert_eq!(hit_thumbnail(l.carousel, &aspects, 0.0, 1.0, c), Some(0));
        // мимо карусели
        assert_eq!(hit_thumbnail(l.carousel, &aspects, 0.0, 1.0, Vec2::new(0.0, 0.0)), None);
    }

    #[test]
    fn viewer_still_none() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), Vec2::new(640.0, 300.0), 1.0), Region::None);
    }

    #[test]
    fn nav_prev_region() {
        let win = Vec2::new(1280.0, 800.0);
        let l = compute(win, 1.0, 1.0, false);
        // левая полоса, ниже titlebar, не в крайних 6px ресайза
        assert_eq!(hit(&l, win, Vec2::new(20.0, 400.0), 1.0), Region::NavPrev);
    }

    #[test]
    fn nav_next_region() {
        let win = Vec2::new(1280.0, 800.0);
        let l = compute(win, 1.0, 1.0, false);
        assert_eq!(hit(&l, win, Vec2::new(1280.0 - 20.0, 400.0), 1.0), Region::NavNext);
    }

    #[test]
    fn resize_edge_priority_over_nav() {
        let win = Vec2::new(1280.0, 800.0);
        let l = compute(win, 1.0, 1.0, false);
        // крайние 6px → ресайз, не NavPrev
        assert_eq!(hit(&l, win, Vec2::new(2.0, 400.0), 1.0), Region::Resize(Edge::Left));
    }

    #[test]
    fn rotate_button_region() {
        let l = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let c = Vec2::new(l.btn_rotate.x + 19.0, l.btn_rotate.y + 42.0);
        assert_eq!(hit(&l, Vec2::new(1280.0, 800.0), c, 1.0), Region::ActionRotate);
    }

    #[test]
    fn popup_close_search_body_outside() {
        let win = Vec2::new(1280.0, 800.0);
        let p = crate::ui::layout::popup_layout(win, 1.0);
        // центр close
        let cc = Vec2::new(p.close.x + p.close.w * 0.5, p.close.y + p.close.h * 0.5);
        assert_eq!(hit_popup(win, 1.0, cc), PopupRegion::Close);
        // центр поиска
        let cs = Vec2::new(p.search.x + 30.0, p.search.y + p.search.h * 0.5);
        assert_eq!(hit_popup(win, 1.0, cs), PopupRegion::Search);
        // центр тела
        let cb = Vec2::new(p.body.x + 30.0, p.body.y + 30.0);
        assert_eq!(hit_popup(win, 1.0, cb), PopupRegion::Body);
        // угол окна — вне карточки
        assert_eq!(hit_popup(win, 1.0, Vec2::new(5.0, 5.0)), PopupRegion::Outside);
    }

    #[test]
    fn popup_footer_save_cancel_hit() {
        let win = Vec2::new(1280.0, 800.0);
        let p = crate::ui::layout::popup_layout(win, 1.0);
        let (save, cancel) = crate::ui::layout::popup_footer_buttons(&p, 1.0);
        let sc = Vec2::new(save.x + save.w * 0.5, save.y + save.h * 0.5);
        let cc = Vec2::new(cancel.x + cancel.w * 0.5, cancel.y + cancel.h * 0.5);
        let tags = crate::exif::tags::parse(r#"[{"SourceFile":"a","EXIF:Make":"X"}]"#);
        assert_eq!(hit_popup_edit(win, 1.0, sc, &tags, "", 0.0, false), PopupRegion::FooterSave);
        assert_eq!(hit_popup_edit(win, 1.0, cc, &tags, "", 0.0, false), PopupRegion::FooterCancel);
    }

    #[test]
    fn popup_row_edit_delete_hit() {
        let win = Vec2::new(1280.0, 800.0);
        let p = crate::ui::layout::popup_layout(win, 1.0);
        let tags = crate::exif::tags::parse(r#"[{"SourceFile":"a","EXIF:Make":"X"}]"#);
        let rows = crate::ui::scene::popup_rows(&tags, "", 1.0, 0.0, p.body);
        let i = rows.iter().position(|r| r.tag == "Make").unwrap();
        let (edit, del) = crate::ui::scene::popup_row_actions(&rows[i], p.body, 1.0);
        let ec = Vec2::new(edit.x + edit.w * 0.5, edit.y + edit.h * 0.5);
        let dc = Vec2::new(del.x + del.w * 0.5, del.y + del.h * 0.5);
        assert_eq!(hit_popup_edit(win, 1.0, ec, &tags, "", 0.0, false), PopupRegion::RowEdit(i));
        assert_eq!(hit_popup_edit(win, 1.0, dc, &tags, "", 0.0, false), PopupRegion::RowDelete(i));
    }

    #[test]
    fn fullscreen_overlay_regions() {
        let win = Vec2::new(1280.0, 800.0);
        let l = compute(win, 1.0, 1.0, true);
        let ce = Vec2::new(l.btn_fs_exit.x + 22.0, l.btn_fs_exit.y + 22.0);
        let cp = Vec2::new(l.btn_fs_play.x + 22.0, l.btn_fs_play.y + 22.0);
        assert_eq!(hit(&l, win, ce, 1.0), Region::FullscreenExit);
        assert_eq!(hit(&l, win, cp, 1.0), Region::SlideshowPlay);
    }
}
