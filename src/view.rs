use glam::{Mat4, Vec2};

pub const ZOOM_MIN: f32 = 0.05;
pub const ZOOM_MAX: f32 = 32.0;
pub const ANIM_DURATION: f32 = 0.15;

pub struct ViewTransform {
    zoom: f32,
    pan: Vec2,
    is_fit: bool,
    zoom_target: f32,
    zoom_start: f32,
    anim_elapsed: f32,
    /// Нижняя граница зума: fit-zoom (фото не делаем меньше окна). ZOOM_MIN до загрузки.
    min_zoom: f32,
    // задел под v0.4
    pub rotation: u8,
    pub flip_h: bool,
    pub flip_v: bool,
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// fit-zoom: вписать изображение в окно по меньшей стороне.
pub fn fit_zoom(win: Vec2, img: Vec2) -> f32 {
    if img.x <= 0.0 || img.y <= 0.0 {
        return 1.0;
    }
    (win.x / img.x).min(win.y / img.y)
}

impl ViewTransform {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            is_fit: true,
            zoom_target: 1.0,
            zoom_start: 1.0,
            anim_elapsed: ANIM_DURATION, // не анимируется
            min_zoom: ZOOM_MIN,
            rotation: 0,
            flip_h: false,
            flip_v: false,
        }
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }
    pub fn pan(&self) -> Vec2 {
        self.pan
    }
    pub fn is_fit(&self) -> bool {
        self.is_fit
    }
    pub fn set_pan(&mut self, p: Vec2) {
        self.pan = p;
    }

    pub fn min_zoom(&self) -> f32 {
        self.min_zoom
    }

    /// Кламп зума в актуальный диапазон [min_zoom .. ZOOM_MAX].
    fn clamp_zoom(&self, z: f32) -> f32 {
        z.clamp(self.min_zoom, ZOOM_MAX)
    }

    /// Установить нижнюю границу зума (= fit-zoom). Подтягивает текущий зум, если он ниже.
    pub fn set_min_zoom(&mut self, m: f32) {
        self.min_zoom = m.clamp(ZOOM_MIN, ZOOM_MAX);
        if self.zoom < self.min_zoom {
            self.zoom = self.min_zoom;
            self.zoom_target = self.min_zoom;
        }
    }

    pub fn set_zoom_immediate(&mut self, z: f32) {
        self.zoom = self.clamp_zoom(z);
        self.zoom_target = self.zoom;
        self.anim_elapsed = ANIM_DURATION;
    }

    /// Зум с сохранением точки изображения под курсором (мгновенный).
    /// `win` нужен потому, что матрица центрирует картинку через (win - scaled)/2,
    /// поэтому опорная точка зума берётся относительно центра окна.
    pub fn zoom_at(&mut self, cursor: Vec2, win: Vec2, new_zoom: f32) {
        let old = self.zoom;
        let nz = self.clamp_zoom(new_zoom);
        let pivot = cursor - win * 0.5;
        // pan' = pivot - (pivot - pan) * (nz / old)
        self.pan = pivot - (pivot - self.pan) * (nz / old);
        self.zoom = nz;
        self.zoom_target = nz;
        self.anim_elapsed = ANIM_DURATION;
        self.is_fit = false;
    }

    /// Запустить плавную анимацию zoom к цели.
    pub fn animate_zoom_to(&mut self, target: f32) {
        self.zoom_start = self.zoom;
        self.zoom_target = self.clamp_zoom(target);
        self.anim_elapsed = 0.0;
    }

    pub fn is_animating(&self) -> bool {
        self.anim_elapsed < ANIM_DURATION
    }

    /// Продвинуть анимацию на dt секунд.
    pub fn tick(&mut self, dt: f32) {
        if !self.is_animating() {
            return;
        }
        self.anim_elapsed += dt;
        let t = (self.anim_elapsed / ANIM_DURATION).clamp(0.0, 1.0);
        self.zoom = self.zoom_start + (self.zoom_target - self.zoom_start) * ease_out_cubic(t);
        if t >= 1.0 {
            self.zoom = self.zoom_target;
            self.anim_elapsed = ANIM_DURATION;
        }
    }

    pub fn set_fit(&mut self, fit: bool) {
        self.is_fit = fit;
    }

    /// Ограничить pan так, чтобы изображение не отрывалось от краёв окна.
    /// По оси, где картинка больше окна, pan ∈ ±(scaled-win)/2 (без полей по краям).
    /// По оси, где картинка ≤ окна, pan = 0 (центрирование, перемещение запрещено).
    pub fn clamp_pan(&mut self, win: Vec2, img: Vec2) {
        let scaled = img * self.zoom;
        let limit_x = ((scaled.x - win.x) * 0.5).max(0.0);
        let limit_y = ((scaled.y - win.y) * 0.5).max(0.0);
        self.pan.x = self.pan.x.clamp(-limit_x, limit_x);
        self.pan.y = self.pan.y.clamp(-limit_y, limit_y);
    }

    /// Матрица: пиксели изображения → clip space, со вшитыми zoom и pan.
    /// Картинка центрируется в окне, затем сдвигается на pan и масштабируется zoom.
    pub fn matrix(&self, win: Vec2, img: Vec2) -> Mat4 {
        // Размер картинки на экране в пикселях
        let scaled = img * self.zoom;
        // Левый-верхний угол картинки: центрируем + pan
        let origin = (win - scaled) * 0.5 + self.pan;
        // Ортопроекция: пиксели экрана (0..win) → clip (-1..1), Y вниз
        let proj = Mat4::orthographic_rh(0.0, win.x, win.y, 0.0, -1.0, 1.0);
        // Модель: масштаб quad'а (0..1) до scaled и перенос в origin
        let model = Mat4::from_translation(glam::Vec3::new(origin.x, origin.y, 0.0))
            * Mat4::from_scale(glam::Vec3::new(scaled.x, scaled.y, 1.0));
        proj * model
    }
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_bounds() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert!(ease_out_cubic(0.5) > 0.5); // ease-out выпуклая
    }

    #[test]
    fn fit_zoom_picks_smaller_ratio() {
        // окно 1000×500, картинка 2000×2000 → ограничивает высота: 500/2000 = 0.25
        let z = fit_zoom(Vec2::new(1000.0, 500.0), Vec2::new(2000.0, 2000.0));
        assert!((z - 0.25).abs() < 1e-6);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1000.0);
        assert_eq!(v.zoom(), ZOOM_MAX);
        v.set_zoom_immediate(0.0001);
        assert_eq!(v.zoom(), ZOOM_MIN);
    }

    /// Точка изображения (в пикселях картинки) под экранным курсором —
    /// с учётом центрирования матрицей: origin = (win - img*zoom)/2 + pan.
    fn image_point_under(v: &ViewTransform, win: Vec2, img: Vec2, cursor: Vec2) -> Vec2 {
        let origin = (win - img * v.zoom()) * 0.5 + v.pan();
        (cursor - origin) / v.zoom()
    }

    #[test]
    fn zoom_at_cursor_keeps_point_fixed() {
        let win = Vec2::new(1280.0, 800.0);
        let img = Vec2::new(6000.0, 4000.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(0.3);
        v.set_pan(Vec2::new(40.0, -25.0));
        let cursor = Vec2::new(900.0, 220.0);
        // точка картинки под курсором до и после зума должна совпасть
        let before = image_point_under(&v, win, img, cursor);
        v.zoom_at(cursor, win, 0.6);
        let after = image_point_under(&v, win, img, cursor);
        assert!((before - after).length() < 0.5, "before={before:?} after={after:?}");
    }

    #[test]
    fn pan_clamped_to_image_bounds() {
        let win = Vec2::new(1000.0, 800.0);
        let img = Vec2::new(2000.0, 2000.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0); // scaled 2000×2000 > окна
        v.set_pan(Vec2::new(10_000.0, -10_000.0));
        v.clamp_pan(win, img);
        // limit_x = (2000-1000)/2 = 500, limit_y = (2000-800)/2 = 600
        assert_eq!(v.pan(), Vec2::new(500.0, -600.0));
    }

    #[test]
    fn pan_zeroed_when_image_fits() {
        let win = Vec2::new(2000.0, 2000.0);
        let img = Vec2::new(1000.0, 800.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0); // картинка меньше окна
        v.set_pan(Vec2::new(123.0, 45.0));
        v.clamp_pan(win, img);
        assert_eq!(v.pan(), Vec2::ZERO);
    }

    #[test]
    fn zoom_does_not_go_below_min() {
        let win = Vec2::new(1280.0, 800.0);
        let mut v = ViewTransform::new();
        v.set_min_zoom(0.2); // fit-zoom
        v.set_zoom_immediate(0.5);
        // попытка зумнуть далеко вниз упирается в min_zoom
        v.zoom_at(Vec2::new(640.0, 400.0), win, 0.01);
        assert_eq!(v.zoom(), 0.2);
    }

    #[test]
    fn animation_progresses_and_settles() {
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0);
        v.animate_zoom_to(2.0);
        assert!(v.is_animating());
        v.tick(ANIM_DURATION / 2.0);
        let mid = v.zoom();
        assert!(mid > 1.0 && mid < 2.0);
        v.tick(ANIM_DURATION); // с запасом
        assert!(!v.is_animating());
        assert!((v.zoom() - 2.0).abs() < 1e-4);
    }

    #[test]
    fn matrix_is_finite() {
        let v = ViewTransform::new();
        let m = v.matrix(Vec2::new(800.0, 600.0), Vec2::new(400.0, 300.0));
        assert!(m.to_cols_array().iter().all(|c| c.is_finite()));
    }
}
