use glam::{Mat4, Vec2};

pub const ZOOM_MIN: f32 = 0.05;
pub const ZOOM_MAX: f32 = 32.0;
pub const ANIM_DURATION: f32 = 0.15;

/// Ручная трансформация отображения (поворот по часовой + отражения).
/// rotation — градусы ∈ {0, 90, 180, 270}.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transform {
    pub rotation: u16,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl Default for Transform {
    fn default() -> Self {
        Self { rotation: 0, flip_h: false, flip_v: false }
    }
}

impl Transform {
    /// Тождественна ли трансформация (нет поворота/отражений). Часть публичного API
    /// `Transform`; пока используется в тестах — потребитель появится в v0.4b (EXIF popup).
    #[allow(dead_code)]
    pub fn is_identity(&self) -> bool {
        self.rotation == 0 && !self.flip_h && !self.flip_v
    }

    /// Эффективные размеры изображения на экране с учётом поворота:
    /// для 90°/270° ширина и высота меняются местами.
    pub fn effective_dims(&self, img: Vec2) -> Vec2 {
        if self.rotation == 90 || self.rotation == 270 {
            Vec2::new(img.y, img.x)
        } else {
            img
        }
    }
}

pub struct ViewTransform {
    zoom: f32,
    pan: Vec2,
    is_fit: bool,
    zoom_target: f32,
    zoom_start: f32,
    anim_elapsed: f32,
    /// Нижняя граница зума: fit-zoom (фото не делаем меньше окна). ZOOM_MIN до загрузки.
    min_zoom: f32,
    transform: Transform,
    /// Транзиентное горизонтальное смещение фото (свайп-перелистывание).
    /// Не клампится и не сбрасывается fit-логикой; управляется извне.
    swipe_offset: f32,
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
            transform: Transform::default(),
            swipe_offset: 0.0,
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

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn set_transform(&mut self, t: Transform) {
        self.transform = t;
    }

    pub fn swipe_offset(&self) -> f32 {
        self.swipe_offset
    }

    pub fn set_swipe_offset(&mut self, dx: f32) {
        self.swipe_offset = dx;
    }

    pub fn rotate_cw(&mut self) {
        self.transform.rotation = (self.transform.rotation + 90) % 360;
    }

    pub fn rotate_ccw(&mut self) {
        self.transform.rotation = (self.transform.rotation + 270) % 360;
    }

    pub fn flip_horizontal(&mut self) {
        self.transform.flip_h = !self.transform.flip_h;
    }

    pub fn flip_vertical(&mut self) {
        self.transform.flip_v = !self.transform.flip_v;
    }

    pub fn reset_transform(&mut self) {
        self.transform = Transform::default();
    }

    /// Ограничить pan так, чтобы изображение не отрывалось от краёв окна.
    /// По оси, где картинка больше окна, pan ∈ ±(scaled-win)/2 (без полей по краям).
    /// По оси, где картинка ≤ окна, pan = 0 (центрирование, перемещение запрещено).
    /// Учитывает эффективные размеры (поворот 90°/270° меняет ширину/высоту местами).
    pub fn clamp_pan(&mut self, win: Vec2, img: Vec2) {
        let eff = self.transform.effective_dims(img);
        let scaled = eff * self.zoom;
        let limit_x = ((scaled.x - win.x) * 0.5).max(0.0);
        let limit_y = ((scaled.y - win.y) * 0.5).max(0.0);
        self.pan.x = self.pan.x.clamp(-limit_x, limit_x);
        self.pan.y = self.pan.y.clamp(-limit_y, limit_y);
    }

    /// fit-zoom с учётом текущего поворота (эффективные размеры).
    pub fn fit_zoom(&self, win: Vec2, img: Vec2) -> f32 {
        fit_zoom(win, self.transform.effective_dims(img))
    }

    /// Подстроить вид под смену размеров изображения (preview→full),
    /// сохранив экранный размер. old_img/new_img имеют одинаковое соотношение сторон,
    /// поэтому коэффициент масштаба не зависит от выбранной оси.
    pub fn rescale_for_new_image(&mut self, win: Vec2, old_img: Vec2, new_img: Vec2) {
        if old_img.y > 0.0 && new_img.y > 0.0 {
            let ratio = old_img.y / new_img.y;
            self.zoom = (self.zoom * ratio).clamp(ZOOM_MIN, ZOOM_MAX);
            self.zoom_target = self.zoom;
        }
        self.set_min_zoom(fit_zoom(win, new_img)); // подтянет zoom до min при необходимости
        self.clamp_pan(win, new_img);
    }

    /// Модель-матрица: unit-quad [0,1]² → экранные пиксели, с центрированием,
    /// pan, zoom, поворотом и отражением. Картинка вращается вокруг своего центра.
    /// Для rotation=0, flip=none эквивалентно старой формуле T(origin) * S(scaled).
    fn model_matrix(&self, win: Vec2, img: Vec2) -> Mat4 {
        // rotation поддерживается только кратным 90° (см. rotate_cw/ccw); set_transform
        // не валидирует, поэтому страхуемся в debug-сборке.
        debug_assert!(
            matches!(self.transform.rotation, 0 | 90 | 180 | 270),
            "rotation должен быть кратен 90°, получено {}",
            self.transform.rotation
        );
        let scaled = img * self.zoom; // размер изображения на экране (пиксели)
        let center = win * 0.5 + self.pan + Vec2::new(self.swipe_offset, 0.0);
        let fx = if self.transform.flip_h { -1.0 } else { 1.0 };
        let fy = if self.transform.flip_v { -1.0 } else { 1.0 };
        let angle = (self.transform.rotation as f32).to_radians();
        Mat4::from_translation(glam::Vec3::new(center.x, center.y, 0.0)) // T(center): позиционирование
            * Mat4::from_rotation_z(angle) // R: поворот вокруг центра
            * Mat4::from_scale(glam::Vec3::new(scaled.x * fx, scaled.y * fy, 1.0)) // S: масштаб + отражение
            * Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, 0.0)) // T(-0.5): центр quad'а в origin
    }

    /// Матрица: пиксели изображения → clip space, со вшитыми zoom, pan, поворотом и отражением.
    /// Картинка центрируется в окне, затем сдвигается на pan и масштабируется zoom.
    pub fn matrix(&self, win: Vec2, img: Vec2) -> Mat4 {
        // Ортопроекция: пиксели экрана (0..win) → clip (-1..1), Y вниз
        let proj = Mat4::orthographic_rh(0.0, win.x, win.y, 0.0, -1.0, 1.0);
        proj * self.model_matrix(win, img)
    }

    /// Четыре угла изображения в экранных пикселях — для геометрии, хит-тестов и отладки.
    /// Порядок соответствует UV-углам unit-quad ДО поворота: (0,0), (1,0), (0,1), (1,1).
    /// Пока используется в тестах геометрии; реальный потребитель — хит-тест повёрнутого фото (v0.4b).
    #[allow(dead_code)]
    pub fn screen_quad(&self, win: Vec2, img: Vec2) -> [Vec2; 4] {
        let m = self.model_matrix(win, img);
        let corners = [
            glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
            glam::Vec4::new(0.0, 1.0, 0.0, 1.0),
            glam::Vec4::new(1.0, 1.0, 0.0, 1.0),
        ];
        corners.map(|c| {
            let p = m * c;
            Vec2::new(p.x, p.y)
        })
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
    fn transform_default_is_identity() {
        let t = Transform::default();
        assert!(t.is_identity());
        assert_eq!(t.rotation, 0);
    }

    #[test]
    fn rotate_cw_cycles_through_four() {
        let mut v = ViewTransform::new();
        v.rotate_cw();
        assert_eq!(v.transform().rotation, 90);
        v.rotate_cw();
        assert_eq!(v.transform().rotation, 180);
        v.rotate_cw();
        assert_eq!(v.transform().rotation, 270);
        v.rotate_cw();
        assert_eq!(v.transform().rotation, 0); // 360 % 360
    }

    #[test]
    fn rotate_ccw_wraps_to_270() {
        let mut v = ViewTransform::new();
        v.rotate_ccw();
        assert_eq!(v.transform().rotation, 270);
    }

    #[test]
    fn flips_toggle() {
        let mut v = ViewTransform::new();
        v.flip_horizontal();
        assert!(v.transform().flip_h);
        v.flip_horizontal();
        assert!(!v.transform().flip_h);
        v.flip_vertical();
        assert!(v.transform().flip_v);
        v.flip_vertical();
        assert!(!v.transform().flip_v);
    }

    #[test]
    fn reset_transform_restores_identity() {
        let mut v = ViewTransform::new();
        v.rotate_cw();
        v.flip_horizontal();
        v.reset_transform();
        assert!(v.transform().is_identity());
    }

    #[test]
    fn set_transform_roundtrips() {
        let mut v = ViewTransform::new();
        let t = Transform { rotation: 180, flip_h: true, flip_v: false };
        v.set_transform(t);
        assert_eq!(v.transform(), t);
    }

    #[test]
    fn swipe_offset_shifts_quad_horizontally() {
        let win = Vec2::new(1000.0, 800.0);
        let img = Vec2::new(400.0, 200.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0);
        let q0 = v.screen_quad(win, img);
        v.set_swipe_offset(123.0);
        assert_eq!(v.swipe_offset(), 123.0);
        let q1 = v.screen_quad(win, img);
        for i in 0..4 {
            assert!((q1[i].x - q0[i].x - 123.0).abs() < 1e-3, "угол {i} по X");
            assert!((q1[i].y - q0[i].y).abs() < 1e-3, "угол {i} по Y");
        }
    }

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
    /// Только для rotation=0/flip=none (поворот не учитывается).
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

    #[test]
    fn rescale_preserves_screen_size_in_fit() {
        let win = Vec2::new(1000.0, 800.0);
        let old_img = Vec2::new(1000.0, 667.0);   // preview
        let new_img = Vec2::new(6000.0, 4002.0);  // full, то же соотношение
        let mut v = ViewTransform::new();
        v.set_min_zoom(fit_zoom(win, old_img));
        v.set_zoom_immediate(fit_zoom(win, old_img));
        v.set_fit(true);
        let screen_before = old_img * v.zoom();
        v.rescale_for_new_image(win, old_img, new_img);
        let screen_after = new_img * v.zoom();
        assert!((screen_before - screen_after).length() < 1.0);
        assert!((v.zoom() - fit_zoom(win, new_img)).abs() < 1e-4);
    }

    #[test]
    fn rescale_preserves_screen_size_when_zoomed() {
        let win = Vec2::new(1000.0, 800.0);
        let old_img = Vec2::new(1000.0, 667.0);
        let new_img = Vec2::new(2000.0, 1334.0);
        let mut v = ViewTransform::new();
        v.set_min_zoom(fit_zoom(win, old_img));
        v.set_zoom_immediate(2.0); // ручной зум сверх fit
        let screen_before = old_img * v.zoom();
        v.rescale_for_new_image(win, old_img, new_img);
        let screen_after = new_img * v.zoom();
        assert!((screen_before - screen_after).length() < 1.0);
    }

    #[test]
    fn rotated_90_swaps_effective_dims() {
        let t = Transform { rotation: 90, flip_h: false, flip_v: false };
        let eff = t.effective_dims(Vec2::new(6000.0, 4000.0));
        assert_eq!(eff, Vec2::new(4000.0, 6000.0));
        let t0 = Transform::default();
        assert_eq!(t0.effective_dims(Vec2::new(6000.0, 4000.0)), Vec2::new(6000.0, 4000.0));
    }

    #[test]
    fn fit_zoom_uses_effective_dims_after_rotation() {
        let win = Vec2::new(800.0, 800.0);
        let img = Vec2::new(4000.0, 2000.0);
        let mut v = ViewTransform::new();
        let fit0 = v.fit_zoom(win, img);
        assert!((fit0 - 0.2).abs() < 1e-6);
        v.rotate_cw();
        let img2 = Vec2::new(3000.0, 1000.0);
        let fit_land = ViewTransform::new().fit_zoom(win, img2);
        let fit_rot = v.fit_zoom(win, img2);
        assert!((fit_land - 800.0 / 3000.0).abs() < 1e-6);
        assert!((fit_rot - 800.0 / 3000.0).abs() < 1e-6);
        let mut vr = ViewTransform::new();
        vr.rotate_cw();
        let asym = Vec2::new(1000.0, 4000.0);
        assert!((vr.fit_zoom(win, asym) - 800.0 / 4000.0).abs() < 1e-6);
        assert!((ViewTransform::new().fit_zoom(win, asym) - 800.0 / 4000.0).abs() < 1e-6);
    }

    #[test]
    fn clamp_pan_uses_effective_dims_when_rotated() {
        let win = Vec2::new(1000.0, 1000.0);
        let img = Vec2::new(2000.0, 800.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(1.0);
        v.rotate_cw();
        v.set_pan(Vec2::new(10_000.0, 10_000.0));
        v.clamp_pan(win, img);
        assert_eq!(v.pan(), Vec2::new(0.0, 500.0));
    }

    #[test]
    fn screen_quad_bbox_matches_effective_size() {
        let win = Vec2::new(1000.0, 800.0);
        let img = Vec2::new(400.0, 200.0);
        let mut v = ViewTransform::new();
        v.set_zoom_immediate(2.0);
        v.rotate_cw();
        let q = v.screen_quad(win, img);
        let min_x = q.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = q.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = q.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = q.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        assert!(((max_x - min_x) - 200.0 * 2.0).abs() < 0.5);
        assert!(((max_y - min_y) - 400.0 * 2.0).abs() < 0.5);
    }

    #[test]
    fn matrix_finite_for_all_transforms() {
        let win = Vec2::new(1000.0, 800.0);
        let img = Vec2::new(400.0, 300.0);
        for rot in [0u16, 90, 180, 270] {
            for fh in [false, true] {
                for fv in [false, true] {
                    let mut v = ViewTransform::new();
                    v.set_transform(Transform { rotation: rot, flip_h: fh, flip_v: fv });
                    let m = v.matrix(win, img);
                    assert!(m.to_cols_array().iter().all(|c| c.is_finite()),
                        "rot={rot} fh={fh} fv={fv}");
                }
            }
        }
    }
}
