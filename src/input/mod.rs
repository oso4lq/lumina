use crate::view::ViewTransform;
use glam::Vec2;
use winit::keyboard::KeyCode;

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

/// Действие, вызванное физической клавишей (раскладко-независимо).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    RotateCw,
    RotateCcw,
    FlipH,
    FlipV,
    ResetTransform,
    ToggleFullscreen,
    FitView,
    ActualSize,
}

/// Маппинг физической клавиши + модификаторов в действие.
/// Матчинг по позиции клавиши (KeyCode), а не по символу — работает на любой
/// раскладке (кириллица К/Р/М/… и др.).
pub fn map_key(code: KeyCode, ctrl: bool, shift: bool) -> Option<Action> {
    match (code, ctrl) {
        (KeyCode::KeyR, false) => Some(if shift { Action::RotateCcw } else { Action::RotateCw }),
        (KeyCode::KeyH, false) => Some(Action::FlipH),
        (KeyCode::KeyV, false) => Some(Action::FlipV),
        (KeyCode::KeyF, false) => Some(Action::ToggleFullscreen),
        (KeyCode::KeyZ, true) => Some(Action::ResetTransform),
        (KeyCode::Digit0, true) => Some(Action::FitView),
        (KeyCode::Digit1, true) => Some(Action::ActualSize),
        _ => None,
    }
}

/// Направление перелистывания свайпом.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Prev,
    Next,
}

/// Доля ширины viewer, после которой свайп считается перелистыванием.
pub const SWIPE_THRESHOLD_FRAC: f32 = 0.15;

/// Решение свайпа по горизонтальному смещению `dx` и ширине viewer.
/// Тянули влево (dx < -порог) → Next; вправо (dx > порог) → Prev; иначе None.
pub fn on_swipe_release(dx: f32, viewer_w: f32) -> Option<NavDir> {
    if viewer_w <= 0.0 {
        return None;
    }
    let threshold = SWIPE_THRESHOLD_FRAC * viewer_w;
    if dx <= -threshold {
        Some(NavDir::Next)
    } else if dx >= threshold {
        Some(NavDir::Prev)
    } else {
        None
    }
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

    #[test]
    fn map_key_rotate_cw_and_ccw() {
        use winit::keyboard::KeyCode;
        assert_eq!(map_key(KeyCode::KeyR, false, false), Some(Action::RotateCw));
        assert_eq!(map_key(KeyCode::KeyR, false, true), Some(Action::RotateCcw));
    }

    #[test]
    fn map_key_flips_and_fullscreen() {
        use winit::keyboard::KeyCode;
        assert_eq!(map_key(KeyCode::KeyH, false, false), Some(Action::FlipH));
        assert_eq!(map_key(KeyCode::KeyV, false, false), Some(Action::FlipV));
        assert_eq!(map_key(KeyCode::KeyF, false, false), Some(Action::ToggleFullscreen));
        // F под Ctrl — не fullscreen
        assert_eq!(map_key(KeyCode::KeyF, true, false), None);
    }

    #[test]
    fn map_key_ctrl_combos() {
        use winit::keyboard::KeyCode;
        assert_eq!(map_key(KeyCode::KeyZ, true, false), Some(Action::ResetTransform));
        assert_eq!(map_key(KeyCode::KeyZ, false, false), None); // Z без Ctrl — ничего
        assert_eq!(map_key(KeyCode::Digit0, true, false), Some(Action::FitView));
        assert_eq!(map_key(KeyCode::Digit1, true, false), Some(Action::ActualSize));
    }

    #[test]
    fn map_key_unknown_is_none() {
        use winit::keyboard::KeyCode;
        assert_eq!(map_key(KeyCode::KeyA, false, false), None);
        assert_eq!(map_key(KeyCode::KeyR, true, false), None); // R под Ctrl — не поворот
    }

    #[test]
    fn swipe_past_threshold_navigates() {
        // порог = 15% ширины. viewer_w=1000 → порог 150.
        assert_eq!(on_swipe_release(-200.0, 1000.0), Some(NavDir::Next)); // тянули влево → следующее
        assert_eq!(on_swipe_release(200.0, 1000.0), Some(NavDir::Prev));  // тянули вправо → предыдущее
    }

    #[test]
    fn swipe_below_threshold_is_none() {
        assert_eq!(on_swipe_release(100.0, 1000.0), None);
        assert_eq!(on_swipe_release(-149.0, 1000.0), None);
    }

    #[test]
    fn swipe_zero_width_is_none() {
        assert_eq!(on_swipe_release(500.0, 0.0), None);
    }
}
