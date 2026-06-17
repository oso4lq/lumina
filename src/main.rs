mod app;
mod catalog;
mod decoder;
mod error;
mod exif;
mod input;
mod platform;
mod renderer;
mod thumbcache;
mod thumbnail;
mod ui;
mod view;

use app::{App, UserEvent};
use std::path::PathBuf;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    // CLI-арг: путь к фото; иначе — системный диалог выбора файла.
    let initial: Option<PathBuf> = match std::env::args().nth(1).map(PathBuf::from) {
        Some(p) if p.exists() => Some(p),
        _ => rfd::FileDialog::new()
            .add_filter("Изображения", &["jpg", "jpeg", "png", "bmp", "gif", "tiff", "tif", "webp"])
            .pick_file(),
    };

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("event loop");
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy, initial);
    event_loop.run_app(&mut app).expect("run_app");
}
