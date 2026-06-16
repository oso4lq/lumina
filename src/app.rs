use crate::catalog::FolderCatalog;
use crate::decoder::{Decoder, DecodedImage, RawDecoder};
use crate::renderer::Renderer;
use crate::thumbnail::ThumbnailStore;
use crate::ui::scene::{self, FileMeta, UiState};
use crate::ui::theme::{self, ThemePalette};
use crate::ui::{hit, layout};
use crate::view::ViewTransform;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

/// Стадия декодирования.
#[derive(Clone, Copy, PartialEq)]
pub enum Stage {
    Preview,
    Full,
}

/// События, приходящие в event loop извне (из rayon).
pub enum UserEvent {
    Decoded {
        generation: u64,
        stage: Stage,
        result: std::result::Result<DecodedImage, String>,
    },
    Thumbnail {
        generation: u64,
        index: usize,
        rgba: Vec<u8>,
        w: u32,
        h: u32,
    },
}

pub struct AppState {
    pub catalog: Option<FolderCatalog>,
    pub view: ViewTransform,
    pub generation: u64,
    pub inited_generation: Option<u64>,
    pub last_frame: Instant,
    pub cursor: glam::Vec2,
    pub dragging: bool,
    pub last_cursor: glam::Vec2,
    pub last_click: Option<(std::time::Instant, glam::Vec2)>,
    pub ctrl_down: bool,
    pub ui: UiState,
    pub theme: ThemePalette,
    pub scale: f32,
    pub thumbs: ThumbnailStore,
    pub raw_flags: Vec<bool>,      // для бейджей: RAW-файл по индексу каталога
    pub badge_labels: Vec<String>, // текст бейджа (расширение в верхнем регистре)
    pub thumb_aspects: Vec<f32>,   // аспект (w/h) миниатюры по индексу; дефолт до загрузки
}

impl AppState {
    fn new() -> Self {
        Self {
            catalog: None,
            view: ViewTransform::new(),
            generation: 0,
            inited_generation: None,
            last_frame: Instant::now(),
            cursor: glam::Vec2::ZERO,
            dragging: false,
            last_cursor: glam::Vec2::ZERO,
            last_click: None,
            ctrl_down: false,
            ui: UiState::new(),
            theme: ThemePalette::dark(),
            scale: 1.0,
            thumbs: ThumbnailStore::new(256),
            raw_flags: Vec::new(),
            badge_labels: Vec::new(),
            thumb_aspects: Vec::new(),
        }
    }
}

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    state: AppState,
    proxy: EventLoopProxy<UserEvent>,
    initial_path: Option<PathBuf>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>, initial_path: Option<PathBuf>) -> Self {
        Self {
            window: None,
            renderer: None,
            state: AppState::new(),
            proxy,
            initial_path,
        }
    }

    /// Запустить декод файла по индексу каталога на rayon.
    fn load_current(&mut self) {
        let Some(catalog) = &self.state.catalog else { return };
        if catalog.is_empty() {
            return;
        }
        let path = catalog.current_path().to_path_buf();
        self.state.generation += 1;
        let generation = self.state.generation;
        let proxy = self.proxy.clone();
        if let Some(w) = &self.window {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            self.state.ui.title = format!("{name} · Lumina");
            w.request_redraw();
        }
        // Списки каталога для карусели и активный индекс
        if let Some(cat) = &self.state.catalog {
            let files = cat.files();
            let n = files.len();
            self.state.ui.thumb_count = n;
            self.state.ui.active_index = cat.current_index();
            self.state.thumb_aspects = vec![theme::THUMB_DEFAULT_AR; n];
            self.state.raw_flags = files
                .iter()
                .map(|p| RawDecoder::supports(&crate::decoder::ext_lower(p)))
                .collect();
            self.state.badge_labels = files
                .iter()
                .map(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.to_uppercase()).unwrap_or_default())
                .collect();
        }
        let ext = crate::decoder::ext_lower(&path);
        rayon::spawn(move || {
            let Some(decoder) = crate::decoder::decoder_for(&ext) else {
                // нет декодера → шлём ошибку как стадию Full
                let _ = proxy.send_event(UserEvent::Decoded {
                    generation,
                    stage: Stage::Full,
                    result: Err(format!("нет декодера для .{ext}")),
                });
                return;
            };
            // Стадия Preview (если есть)
            match decoder.decode_preview(&path) {
                Ok(Some(img)) => {
                    let _ = proxy.send_event(UserEvent::Decoded {
                        generation,
                        stage: Stage::Preview,
                        result: Ok(img),
                    });
                }
                Ok(None) => {}
                Err(e) => log::warn!("превью {path:?}: {e}"),
            }
            // Стадия Full
            let result = decoder.decode_full(&path).map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Decoded {
                generation,
                stage: Stage::Full,
                result,
            });
        });
    }

    /// Навигация по каталогу: -1 prev, +1 next, i32::MIN first, i32::MAX last.
    fn navigate(&mut self, n: i32) {
        let moved = if let Some(cat) = &mut self.state.catalog {
            match n {
                i32::MIN => { cat.go_first(); true }
                i32::MAX => { cat.go_last(); true }
                x if x > 0 => cat.next(),
                _ => cat.prev(),
            }
        } else {
            false
        };
        if moved {
            self.load_current();
        }
    }

    /// Double-click / Ctrl-клавиши: переключить fit ↔ 100% с анимацией.
    fn toggle_fit(&mut self) {
        let Some(r) = &self.renderer else { return };
        let Some(img) = r.image_size() else { return };
        let win = r.viewer_size();
        let fit = crate::view::fit_zoom(win, img);
        if self.state.view.is_fit() {
            // fit → 100%
            self.state.view.set_fit(false);
            self.state.view.animate_zoom_to(1.0);
        } else {
            // → fit
            self.state.view.set_fit(true);
            self.state.view.set_pan(glam::Vec2::ZERO);
            self.state.view.animate_zoom_to(fit);
        }
        if let Some(w) = &self.window { w.request_redraw(); }
    }

    /// Ctrl+0: вписать в окно (fit) с анимацией.
    fn set_fit_view(&mut self) {
        let Some(r) = &self.renderer else { return };
        let Some(img) = r.image_size() else { return };
        let fit = crate::view::fit_zoom(r.viewer_size(), img);
        self.state.view.set_fit(true);
        self.state.view.set_pan(glam::Vec2::ZERO);
        self.state.view.animate_zoom_to(fit);
        if let Some(w) = &self.window { w.request_redraw(); }
    }

    /// Ctrl+1: 100% (1:1) с анимацией.
    fn set_actual_size(&mut self) {
        self.state.view.set_fit(false);
        self.state.view.animate_zoom_to(1.0);
        if let Some(w) = &self.window { w.request_redraw(); }
    }

    /// Открыть файл: построить каталог его папки и начать загрузку.
    pub fn open_file(&mut self, path: PathBuf) {
        match FolderCatalog::open(&path) {
            Ok(cat) => {
                self.state.catalog = Some(cat);
                self.state.view = ViewTransform::new();
                self.state.inited_generation = None;
                self.state.thumbs.reset();
                self.state.ui.scroll = 0.0;
                self.load_current();
            }
            Err(e) => log::warn!("не удалось открыть папку для {path:?}: {e}"),
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.state.ui.fullscreen = !self.state.ui.fullscreen;
        if let Some(w) = &self.window {
            let fs = if self.state.ui.fullscreen {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                None
            };
            w.set_fullscreen(fs);
            w.request_redraw();
        }
        #[cfg(windows)]
        crate::platform::windows::set_fullscreen(self.state.ui.fullscreen);
    }

    /// Перейти к файлу по индексу каталога.
    fn navigate_to(&mut self, index: usize) {
        let moved = if let Some(cat) = &mut self.state.catalog {
            cat.go_to(index)
        } else {
            false
        };
        if moved {
            self.load_current();
        }
    }

    /// Запросить декод миниатюр для индексов окна, которых ещё нет.
    fn request_thumbnails(&mut self, window: Vec<usize>) {
        let Some(catalog) = &self.state.catalog else { return };
        let pending = self.state.thumbs.take_pending(&window);
        if pending.is_empty() {
            return;
        }
        let generation = self.state.thumbs.generation;
        let scale = self.state.scale;
        let th = (crate::ui::theme::THUMB_H * scale).round() as u32;
        for index in pending {
            let Some(path) = catalog.files().get(index) else { continue };
            let path = path.to_path_buf();
            let proxy = self.proxy.clone();
            let ext = crate::decoder::ext_lower(&path);
            rayon::spawn(move || {
                let Some(decoder) = crate::decoder::decoder_for(&ext) else { return };
                // источник: встроенное превью если есть, иначе полный декод
                let decoded = match decoder.decode_preview(&path) {
                    Ok(Some(img)) => Some(img),
                    _ => decoder.decode_full(&path).ok(),
                };
                let Some(img) = decoded else {
                    let _ = proxy.send_event(UserEvent::Thumbnail { generation, index, rgba: Vec::new(), w: 0, h: 0 });
                    return;
                };
                let (rgba, w, h) = crate::app::make_thumb(&img.rgba, img.width, img.height, th);
                let _ = proxy.send_event(UserEvent::Thumbnail { generation, index, rgba, w, h });
            });
        }
    }
}

/// Ресайз исходного RGBA до высоты `th` с сохранением аспекта (без кропа).
/// Возвращает (rgba, tw, th), где tw = round(th × аспект).
pub fn make_thumb(src: &[u8], sw: u32, sh: u32, th: u32) -> (Vec<u8>, u32, u32) {
    use image::{imageops, RgbaImage};
    let th = th.max(1);
    let ar = if sh > 0 { sw as f32 / sh as f32 } else { crate::ui::theme::THUMB_DEFAULT_AR };
    let ar = ar.clamp(crate::ui::theme::THUMB_MIN_AR, crate::ui::theme::THUMB_MAX_AR);
    let tw = ((th as f32) * ar).round().max(1.0) as u32;
    let Some(buf) = RgbaImage::from_raw(sw, sh, src.to_vec()) else {
        return (vec![0u8; (tw * th * 4) as usize], tw, th);
    };
    let resized = imageops::resize(&buf, tw, th, imageops::FilterType::Triangle);
    (resized.into_raw(), tw, th)
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Lumina")
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create_window"));
        match Renderer::new(window.clone()) {
            Ok(mut r) => {
                self.state.scale = window.scale_factor() as f32;
                r.set_titlebar_height(theme::TITLEBAR_HEIGHT * self.state.scale);
                self.renderer = Some(r);
            }
            Err(e) => {
                log::error!("инициализация рендера провалилась: {e}");
                event_loop.exit();
                return;
            }
        }

        #[cfg(windows)]
        {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            crate::platform::windows::set_scale(self.state.scale);
            if let Ok(h) = window.window_handle() {
                if let RawWindowHandle::Win32(w) = h.as_raw() {
                    if let Err(e) = crate::platform::windows::enable(w.hwnd.get()) {
                        log::warn!("frameless не включён ({e}); остаёмся с декорациями");
                    }
                }
            }
        }

        self.window = Some(window);

        // Открыть стартовый файл, если был передан.
        if let Some(path) = self.initial_path.take() {
            self.open_file(path);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Decoded { generation, stage, result } => {
                if generation != self.state.generation {
                    return; // устаревший результат — игнор
                }
                match result {
                    Ok(img) => {
                        let new_img = glam::Vec2::new(img.width as f32, img.height as f32);
                        if let Some(r) = &mut self.renderer {
                            let old_img = r.image_size();
                            r.upload_texture(&img.rgba, img.width, img.height);
                            let win = r.viewer_size();
                            if self.state.inited_generation != Some(generation) {
                                // первый кадр этой генерации → инициализация вида (как v0.1)
                                let z = crate::view::fit_zoom(win, new_img);
                                self.state.view.set_min_zoom(z);
                                self.state.view.set_zoom_immediate(z);
                                self.state.view.set_fit(true);
                                self.state.view.set_pan(glam::Vec2::ZERO);
                                self.state.inited_generation = Some(generation);
                            } else if let Some(old_img) = old_img {
                                // подмена preview→full: сохраняем экранный размер
                                self.state.view.rescale_for_new_image(win, old_img, new_img);
                            }
                        }
                        // мета-панель текущего фото
                        if let Some(cat) = &self.state.catalog {
                            let path = cat.current_path();
                            let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_uppercase();
                            let is_raw = RawDecoder::supports(&crate::decoder::ext_lower(path));
                            let format_label = if is_raw { format!("{ext} · RAW") } else { ext };
                            let mp = (img.width as f32 * img.height as f32) / 1_000_000.0;
                            self.state.ui.meta = Some(FileMeta {
                                format_label,
                                megapixels: mp,
                                width: img.width,
                                height: img.height,
                                bytes,
                            });
                        }
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                    Err(e) => {
                        // full упал; если preview уже показан (inited) — просто остаёмся на нём
                        log::warn!("декодирование ({}) не удалось: {e}",
                            if stage == Stage::Preview { "preview" } else { "full" });
                    }
                }
            }
            UserEvent::Thumbnail { generation, index, rgba, w, h } => {
                if generation != self.state.thumbs.generation {
                    return; // устаревшее поколение (сменилась папка)
                }
                let ok = !rgba.is_empty() && w > 0 && h > 0;
                if ok {
                    if let Some(r) = &mut self.renderer {
                        r.set_thumbnail(index, &rgba, w, h);
                    }
                    // реальный аспект миниатюры → лента переразложится на след. кадре
                    if let Some(a) = self.state.thumb_aspects.get_mut(index) {
                        *a = w as f32 / h as f32;
                    }
                }
                let freed = self.state.thumbs.mark_ready(index, ok);
                if let Some(r) = &mut self.renderer {
                    for i in freed {
                        r.drop_thumbnail(i);
                    }
                }
                if let Some(wnd) = &self.window {
                    wnd.request_redraw();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                    if let Some(img) = r.image_size() {
                        // fit меняется с размером окна → обновляем нижнюю границу зума
                        let z = crate::view::fit_zoom(r.viewer_size(), img);
                        self.state.view.set_min_zoom(z);
                        // fit прилипает к ресайзу
                        if self.state.view.is_fit() {
                            self.state.view.set_zoom_immediate(z);
                            self.state.view.set_pan(glam::Vec2::ZERO);
                        }
                    }
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::DroppedFile(path) => {
                self.open_file(path);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.state.last_frame).as_secs_f32();
                self.state.last_frame = now;
                self.state.view.tick(dt);
                // анимация toggle bottom bar (~200 мс)
                let target = if self.state.ui.bottom_visible { 1.0 } else { 0.0 };
                let f = self.state.ui.bottom_factor;
                let animating = (f - target).abs() > 0.001;
                self.state.ui.bottom_factor = if animating { f + (target - f) * (dt / 0.2).min(1.0) } else { target };

                let prep = self.renderer.as_ref().map(|r| {
                    let win = r.surface_size();
                    let l = layout::compute(win, self.state.scale, self.state.ui.bottom_factor, self.state.ui.fullscreen);
                    let thumb_rects = layout::carousel_thumb_rects(l.carousel, &self.state.thumb_aspects, self.state.ui.scroll, self.state.scale);
                    (l, thumb_rects)
                });
                if let Some((l, thumb_rects)) = prep {
                    let window: Vec<usize> = thumb_rects.iter().map(|(i, _)| *i).collect();
                    self.request_thumbnails(window);
                    let bottom_chrome = if self.state.ui.fullscreen { 0.0 } else { l.divider.h + l.bottom_bar.h };
                    let tb = if self.state.ui.fullscreen { 0.0 } else { theme::TITLEBAR_HEIGHT * self.state.scale };
                    let cmds = scene::build(&self.state.ui, &l, &self.state.theme, self.state.scale, &thumb_rects, &self.state.raw_flags);
                    if let Some(r) = &mut self.renderer {
                        r.set_bottom_chrome_height(bottom_chrome);
                        r.set_titlebar_height(tb);
                        // миниатюры клипуются зоной карусели (без остатков при сворачивании)
                        r.set_thumb_clip(l.carousel);
                        // viewer меняется при сворачивании bottom bar → фото заполняет освободившееся место
                        if let Some(img) = r.image_size() {
                            let z = crate::view::fit_zoom(r.viewer_size(), img);
                            self.state.view.set_min_zoom(z);
                            if self.state.view.is_fit() {
                                self.state.view.set_zoom_immediate(z);
                                self.state.view.set_pan(glam::Vec2::ZERO);
                            }
                        }
                        let ready: Vec<(usize, crate::ui::layout::Rect)> = thumb_rects.iter().filter(|(i, _)| r.has_thumbnail(*i)).copied().collect();
                        if let Err(e) = r.render(&self.state.view, &cmds, &ready) { log::warn!("render: {e}"); }
                    }
                }
                if self.state.view.is_animating() || animating {
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = glam::Vec2::new(position.x as f32, position.y as f32);
                if self.state.dragging {
                    let delta = pos - self.state.last_cursor;
                    self.state.view.set_pan(self.state.view.pan() + delta);
                    if let Some(r) = &self.renderer {
                        if let Some(img) = r.image_size() {
                            self.state.view.clamp_pan(r.viewer_size(), img);
                        }
                    }
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
                self.state.last_cursor = pos;
                self.state.cursor = pos;
                // hover по регионам (titlebar + bottom bar)
                if let Some(r) = &self.renderer {
                    let win = r.surface_size();
                    let l = layout::compute(win, self.state.scale, self.state.ui.bottom_factor, self.state.ui.fullscreen);
                    let region = hit::hit(&l, win, pos, self.state.scale);
                    if region != self.state.ui.hovered {
                        self.state.ui.hovered = region;
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 50.0,
                };
                let (win, region) = match &self.renderer {
                    Some(r) => {
                        let win = r.surface_size();
                        let l = layout::compute(win, self.state.scale, self.state.ui.bottom_factor, self.state.ui.fullscreen);
                        (Some(win), hit::hit(&l, win, self.state.cursor, self.state.scale))
                    }
                    None => (None, hit::Region::None),
                };
                let over_carousel = matches!(region, hit::Region::Carousel | hit::Region::Thumbnail(_));
                if over_carousel {
                    // горизонтальный скролл карусели
                    let step = 60.0 * self.state.scale;
                    let content = crate::ui::layout::carousel_content_width(&self.state.thumb_aspects, self.state.scale);
                    let view_w = self.renderer.as_ref().map(|r| {
                        let win = r.surface_size();
                        layout::compute(win, self.state.scale, self.state.ui.bottom_factor, self.state.ui.fullscreen).carousel.w
                    }).unwrap_or(0.0);
                    let max_scroll = (content - view_w).max(0.0);
                    self.state.ui.scroll = (self.state.ui.scroll - lines * step).clamp(0.0, max_scroll);
                    if let Some(w) = &self.window { w.request_redraw(); }
                } else if let Some(_win) = win {
                    // zoom как в v0.3a (курсор скорректирован на titlebar)
                    let vw = self.renderer.as_ref().map(|r| r.viewer_size()).unwrap_or_default();
                    let cursor_v = self.state.cursor - glam::Vec2::new(0.0, self.state.scale * theme::TITLEBAR_HEIGHT);
                    let out = crate::input::on_wheel(&mut self.state.view, cursor_v, vw, lines);
                    if let Some(r) = &self.renderer {
                        if let Some(img) = r.image_size() {
                            self.state.view.clamp_pan(vw, img);
                        }
                    }
                    if out.redraw {
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                use winit::event::{ElementState, MouseButton};
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            // клики по регионам UI имеют приоритет над логикой фото
                            let hit_info = self.renderer.as_ref().map(|r| {
                                let win = r.surface_size();
                                let l = layout::compute(win, self.state.scale, self.state.ui.bottom_factor, self.state.ui.fullscreen);
                                let region = hit::hit(&l, win, self.state.cursor, self.state.scale);
                                (l, region)
                            });
                            if let Some((l, region)) = hit_info {
                                match region {
                                    hit::Region::Close => { event_loop.exit(); return; }
                                    hit::Region::Minimize => {
                                        if let Some(w) = &self.window { w.set_minimized(true); }
                                        return;
                                    }
                                    hit::Region::Maximize => {
                                        if let Some(w) = &self.window {
                                            let m = !w.is_maximized();
                                            w.set_maximized(m);
                                            self.state.ui.maximized = m;
                                        }
                                        return;
                                    }
                                    hit::Region::ActionFullscreen => { self.toggle_fullscreen(); return; }
                                    hit::Region::ActionExif => { return; } // инертна (v0.4)
                                    hit::Region::Divider => {
                                        self.state.ui.bottom_visible = !self.state.ui.bottom_visible;
                                        if let Some(w) = &self.window { w.request_redraw(); }
                                        return;
                                    }
                                    hit::Region::Carousel | hit::Region::Thumbnail(_) => {
                                        if let Some(idx) = hit::hit_thumbnail(l.carousel, &self.state.thumb_aspects, self.state.ui.scroll, self.state.scale, self.state.cursor) {
                                            self.navigate_to(idx);
                                        }
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                            // двойной клик: < 400 мс и малое смещение
                            let now = std::time::Instant::now();
                            let dbl = self.state.last_click.map_or(false, |(t, p)| {
                                now.duration_since(t).as_millis() < 400
                                    && (p - self.state.cursor).length() < 6.0
                            });
                            if dbl {
                                self.toggle_fit();
                                self.state.last_click = None;
                            } else {
                                self.state.last_click = Some((now, self.state.cursor));
                                // pan только когда фото увеличено зумом сверх fit
                                let can_pan = self.state.view.zoom() > self.state.view.min_zoom();
                                self.state.dragging = can_pan;
                                if can_pan {
                                    if let Some(w) = &self.window {
                                        w.set_cursor(winit::window::Cursor::Icon(
                                            winit::window::CursorIcon::Grabbing,
                                        ));
                                    }
                                }
                            }
                        }
                        ElementState::Released => {
                            self.state.dragging = false;
                            if let Some(w) = &self.window {
                                w.set_cursor(winit::window::Cursor::Icon(
                                    winit::window::CursorIcon::Default,
                                ));
                            }
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{Key, NamedKey};
                if event.state.is_pressed() {
                    // F / F11 / Esc — fullscreen, перехватываются первыми
                    match event.logical_key.as_ref() {
                        Key::Named(NamedKey::F11) => { self.toggle_fullscreen(); return; }
                        Key::Named(NamedKey::Escape) => {
                            if self.state.ui.fullscreen { self.toggle_fullscreen(); }
                            return;
                        }
                        Key::Character(c) if c.eq_ignore_ascii_case("f") && !self.state.ctrl_down => {
                            self.toggle_fullscreen();
                            return;
                        }
                        _ => {}
                    }
                    let nav = match event.logical_key.as_ref() {
                        Key::Named(NamedKey::ArrowRight) => Some(crate::input::NavKey::Next),
                        Key::Named(NamedKey::ArrowLeft) => Some(crate::input::NavKey::Prev),
                        Key::Named(NamedKey::Home) => Some(crate::input::NavKey::First),
                        Key::Named(NamedKey::End) => Some(crate::input::NavKey::Last),
                        _ => None,
                    };
                    if let Some(k) = nav {
                        if let Some(n) = crate::input::on_nav_key(k).navigate {
                            self.navigate(n);
                        }
                    } else if self.state.ctrl_down {
                        if let Key::Character(c) = event.logical_key.as_ref() {
                            match c {
                                "0" => self.set_fit_view(),
                                "1" => self.set_actual_size(),
                                _ => {}
                            }
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.state.ctrl_down = mods.state().control_key();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.state.scale = scale_factor as f32;
                if let Some(r) = &mut self.renderer {
                    r.set_titlebar_height(theme::TITLEBAR_HEIGHT * self.state.scale);
                }
                #[cfg(windows)]
                crate::platform::windows::set_scale(self.state.scale);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}
