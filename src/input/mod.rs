use crate::view::ViewTransform;
use glam::Vec2;

/// Что приложение должно сделать после обработки ввода.
#[derive(Debug, Default, PartialEq)]
pub struct InputOutcome {
    pub redraw: bool,
    /// Навигация по каталогу: -1 prev, +1 next, i32::MIN = first, i32::MAX = last.
    pub navigate: Option<i32>,
}

/// Шаг зума колёсиком: положительный delta → приблизить.
pub const WHEEL_STEP: f32 = 1.15;

/// Обработать «прокрутку колёсика на delta_lines» в позиции курсора.
/// `win` — размер окна (нужен для корректного центра зума под курсором).
pub fn on_wheel(view: &mut ViewTransform, cursor: Vec2, win: Vec2, delta_lines: f32) -> InputOutcome {
    if delta_lines == 0.0 {
        return InputOutcome::default();
    }
    let factor = if delta_lines > 0.0 { WHEEL_STEP } else { 1.0 / WHEEL_STEP };
    view.zoom_at(cursor, win, view.zoom() * factor);
    InputOutcome { redraw: true, navigate: None }
}

/// Навигационная клавиша. Возвращает outcome.navigate.
pub fn on_nav_key(key: NavKey) -> InputOutcome {
    let n = match key {
        NavKey::Next => 1,
        NavKey::Prev => -1,
        NavKey::First => i32::MIN,
        NavKey::Last => i32::MAX,
    };
    InputOutcome { redraw: false, navigate: Some(n) }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavKey {
    Next,
    Prev,
    First,
    Last,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_up_zooms_in() {
        let win = Vec2::new(1280.0, 800.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0);
        let out = on_wheel(&mut v, Vec2::new(10.0, 10.0), win, 1.0);
        assert!(out.redraw);
        assert!(v.zoom() > 1.0);
    }

    #[test]
    fn wheel_down_zooms_out() {
        let win = Vec2::new(1280.0, 800.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0);
        on_wheel(&mut v, Vec2::new(10.0, 10.0), win, -1.0);
        assert!(v.zoom() < 1.0);
    }

    #[test]
    fn nav_keys_map_to_navigate() {
        assert_eq!(on_nav_key(NavKey::Next).navigate, Some(1));
        assert_eq!(on_nav_key(NavKey::Prev).navigate, Some(-1));
        assert_eq!(on_nav_key(NavKey::First).navigate, Some(i32::MIN));
        assert_eq!(on_nav_key(NavKey::Last).navigate, Some(i32::MAX));
    }
}
