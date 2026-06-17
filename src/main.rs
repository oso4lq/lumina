mod app;
mod catalog;
mod decoder;
mod error;
mod exif;
mod input;
mod platform;
mod prefetch;
mod renderer;
mod thumbcache;
mod thumbnail;
mod ui;
mod view;
mod watcher;

use app::{App, UserEvent};
use std::path::PathBuf;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    // CLI-арг: путь к фото; иначе — системный диалог выбора файла.
    let initial: Option<PathBuf> = match std::env::args().nth(1).map(PathBuf::from) {
        Some(p) if p.exists() => Some(p),
        _ => rfd::FileDialog::new()
            // все поддерживаемые форматы, включая RAW (raf/nef/arw/cr2/dng/…) и HEIC/HEIF
            .add_filter("Изображения", &decoder::supported_extensions())
            .pick_file(),
    };

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("event loop");
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy, initial);
    event_loop.run_app(&mut app).expect("run_app");
}
