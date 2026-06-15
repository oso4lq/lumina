mod app;
mod catalog;
mod decoder;
mod error;
mod input;
mod renderer;
mod view;

use app::{App, UserEvent};
use std::path::PathBuf;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    // CLI-арг: путь к фото (опционально)
    let initial: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.exists());

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("event loop");
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy, initial);
    event_loop.run_app(&mut app).expect("run_app");
}
