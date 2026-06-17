//! Тонкая обёртка: наблюдение за папкой через notify (+ дебаунс) с прокидыванием
//! факта изменения в winit-event-loop как UserEvent::FolderChanged. Логики согласования нет.
use crate::app::UserEvent;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::path::Path;
use std::time::Duration;
use winit::event_loop::EventLoopProxy;

/// Хранит дебаунсер; его Drop останавливает поток наблюдения.
pub struct FolderWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl FolderWatcher {
    /// Начать нерекурсивное наблюдение за `dir`. Любое пакетированное изменение →
    /// один UserEvent::FolderChanged. None при ошибке (наблюдение просто не активируется).
    pub fn watch(dir: &Path, proxy: EventLoopProxy<UserEvent>) -> Option<FolderWatcher> {
        let mut debouncer = new_debouncer(
            Duration::from_millis(300),
            move |res: DebounceEventResult| {
                if res.is_ok() {
                    let _ = proxy.send_event(UserEvent::FolderChanged);
                }
            },
        )
        .ok()?;
        debouncer.watcher().watch(dir, RecursiveMode::NonRecursive).ok()?;
        Some(FolderWatcher { _debouncer: debouncer })
    }
}
