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

    pub fn set_zoom_immediate(&mut self, z: f32) {
        self.zoom = z.clamp(ZOOM_MIN, ZOOM_MAX);
        self.zoom_target = self.zoom;
        self.anim_elapsed = ANIM_DURATION;
    }

    /// Зум с сохранением точки изображения под курсором (мгновенный).
    pub fn zoom_at(&mut self, cursor: Vec2, new_zoom: f32) {
        let old = self.zoom;
        let nz = new_zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        // pan' = cursor - (cursor - pan) * (nz / old)
        self.pan = cursor - (cursor - self.pan) * (nz / old);
        self.zoom = nz;
        self.zoom_target = nz;
        self.anim_elapsed = ANIM_DURATION;
        self.is_fit = false;
    }

    /// Запустить плавную анимацию zoom к цели.
    pub fn animate_zoom_to(&mut self, target: f32) {
        self.zoom_start = self.zoom;
        self.zoom_target = target.clamp(ZOOM_MIN, ZOOM_MAX);
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

    #[test]
    fn zoom_at_cursor_keeps_point_fixed() {
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0);
        v.set_pan(Vec2::ZERO);
        let cursor = Vec2::new(100.0, 80.0);
        // точка изображения под курсором до зума
        let before = (cursor - v.pan()) / v.zoom();
        v.zoom_at(cursor, 2.0); // зум к 2x с центром под курсором
        let after = (cursor - v.pan()) / v.zoom();
        assert!((before - after).length() < 1e-3);
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
