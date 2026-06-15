use crate::catalog::FolderCatalog;
use crate::decoder::{DecodedImage, Decoder, StandardDecoder};
use crate::renderer::Renderer;
use crate::view::ViewTransform;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

/// События, приходящие в event loop извне (из rayon).
pub enum UserEvent {
    Decoded {
        generation: u64,
        result: std::result::Result<DecodedImage, String>,
    },
}

pub struct AppState {
    pub catalog: Option<FolderCatalog>,
    pub view: ViewTransform,
    pub generation: u64,
    pub last_frame: Instant,
    pub cursor: glam::Vec2,
    pub dragging: bool,
    pub last_cursor: glam::Vec2,
    pub last_click: Option<(std::time::Instant, glam::Vec2)>,
    pub ctrl_down: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            catalog: None,
            view: ViewTransform::new(),
            generation: 0,
            last_frame: Instant::now(),
            cursor: glam::Vec2::ZERO,
            dragging: false,
            last_cursor: glam::Vec2::ZERO,
            last_click: None,
            ctrl_down: false,
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
            w.set_title(&format!("{name} — Lumina"));
        }
        rayon::spawn(move || {
            let result = StandardDecoder
                .decode(&path)
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Decoded { generation, result });
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
        let win = r.surface_size();
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
        let fit = crate::view::fit_zoom(r.surface_size(), img);
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
                self.load_current();
            }
            Err(e) => log::warn!("не удалось открыть папку для {path:?}: {e}"),
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Lumina")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create_window"));
        match Renderer::new(window.clone()) {
            Ok(r) => self.renderer = Some(r),
            Err(e) => {
                log::error!("инициализация рендера провалилась: {e}");
                event_loop.exit();
                return;
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
            UserEvent::Decoded { generation, result } => {
                if generation != self.state.generation {
                    return; // устаревший результат — игнор
                }
                match result {
                    Ok(img) => {
                        if let Some(r) = &mut self.renderer {
                            r.upload_texture(&img.rgba, img.width, img.height);
                            // вписать в окно
                            let win = r.surface_size();
                            let z = crate::view::fit_zoom(win, glam::Vec2::new(img.width as f32, img.height as f32));
                            self.state.view.set_zoom_immediate(z);
                            self.state.view.set_fit(true);
                            self.state.view.set_pan(glam::Vec2::ZERO);
                        }
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                    Err(e) => log::warn!("декодирование не удалось: {e}"),
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
                    // fit прилипает к ресайзу
                    if self.state.view.is_fit() {
                        if let Some(img) = r.image_size() {
                            let z = crate::view::fit_zoom(r.surface_size(), img);
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
                if let Some(r) = &mut self.renderer {
                    if let Err(e) = r.render(&self.state.view) {
                        log::warn!("render: {e}");
                    }
                }
                if self.state.view.is_animating() {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = glam::Vec2::new(position.x as f32, position.y as f32);
                if self.state.dragging {
                    let delta = pos - self.state.last_cursor;
                    let pan = self.state.view.pan() + delta;
                    self.state.view.set_pan(pan);
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
                self.state.last_cursor = pos;
                self.state.cursor = pos;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 50.0,
                };
                let out = crate::input::on_wheel(&mut self.state.view, self.state.cursor, lines);
                if out.redraw { if let Some(w) = &self.window { w.request_redraw(); } }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                use winit::event::{ElementState, MouseButton};
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
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
                                self.state.dragging = true;
                                if let Some(w) = &self.window {
                                    w.set_cursor(winit::window::Cursor::Icon(
                                        winit::window::CursorIcon::Grabbing,
                                    ));
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
            _ => {}
        }
    }
}
