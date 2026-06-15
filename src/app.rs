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
}

impl AppState {
    fn new() -> Self {
        Self {
            catalog: None,
            view: ViewTransform::new(),
            generation: 0,
            last_frame: Instant::now(),
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
            _ => {}
        }
    }
}
