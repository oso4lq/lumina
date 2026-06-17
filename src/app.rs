use crate::catalog::FolderCatalog;
use crate::decoder::{Decoder, DecodedImage, RawDecoder};
use crate::renderer::Renderer;
use crate::thumbnail::ThumbnailStore;
use crate::ui::scene::{self, FileMeta, UiState};
use crate::ui::theme::{self, ThemePalette};
use crate::ui::{hit, layout};
use crate::view::{Transform, ViewTransform};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
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
    ExifLoaded {
        generation: u64,
        result: std::result::Result<crate::exif::tags::ExifTags, String>,
    },
    ExifSaved {
        generation: u64,
        result: std::result::Result<(), String>,
    },
    /// Упреждённый сосед декодирован — положить в префетч-кэш (вид не трогаем).
    Prefetched {
        path: PathBuf,
        image: DecodedImage,
    },
    /// Содержимое открытой папки изменилось (от наблюдателя ФС) — пере-сканировать каталог.
    FolderChanged,
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
    pub shift_down: bool,
    pub transforms: HashMap<PathBuf, Transform>,
    pub ui: UiState,
    pub theme: ThemePalette,
    pub scale: f32,
    pub thumbs: ThumbnailStore,
    pub raw_flags: Vec<bool>,      // для бейджей: RAW-файл по индексу каталога
    pub badge_labels: Vec<String>, // текст бейджа (расширение в верхнем регистре)
    pub thumb_aspects: Vec<f32>,   // аспект (w/h) миниатюры по индексу; дефолт до загрузки
    pub thumbs_in_flight: usize,   // число одновременных декодов миниатюр (троттлинг)
    pub cursor_idle: f32,          // секунды с последнего движения курсора (для fullscreen-оверлея)
    pub swiping: bool,
    pub swipe_press_x: f32,
    pub swipe_anim: bool,
    pub exif_tags: Option<crate::exif::tags::ExifTags>,
    pub exif_search: crate::ui::textedit::TextEdit,
    pub exif_scroll: f32,
    pub exif_error: Option<String>,
    pub exif_generation: u64,
    /// Отсчёт фазы мигания каретки поиска (сбрасывается при открытии и вводе → каретка сразу видна).
    pub exif_blink_since: Instant,
    /// Видима ли каретка в последнем отрисованном кадре (гейт перерисовки по смене фазы).
    pub exif_caret_on: bool,
    /// Поле поиска в фокусе (каретка мигает, клавиатура идёт в поиск).
    pub exif_focus: bool,
    pub exif_pending: std::collections::BTreeMap<(String, String), crate::ui::scene::PendingOp>,
    pub exif_pending_delete_gps: bool,
    pub exif_editing: Option<(String, String)>,
    pub exif_editor: crate::ui::textedit::TextEdit,
    pub exif_confirm: crate::ui::scene::ConfirmKind,
    pub exif_overwrite_mode: bool,
    pub exif_close_after_save: bool,
    pub exif_hovered_row: Option<usize>,
    /// In-memory кэш декодированных кадров для мгновенной навигации (префетч ±2).
    pub prefetch: crate::prefetch::PrefetchCache,
    /// Пути соседей, для которых уже запущен фоновый декод (защита от повторных спавнов).
    pub prefetch_inflight: std::collections::HashSet<PathBuf>,
    /// Накопитель горизонтального смещения тачпада (для пошаговой навигации свайпом).
    pub trackpad_accum: f32,
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
            shift_down: false,
            transforms: HashMap::new(),
            ui: UiState::new(),
            theme: ThemePalette::dark(),
            scale: 1.0,
            thumbs: ThumbnailStore::new(256),
            raw_flags: Vec::new(),
            badge_labels: Vec::new(),
            thumb_aspects: Vec::new(),
            thumbs_in_flight: 0,
            cursor_idle: 0.0,
            swiping: false,
            swipe_press_x: 0.0,
            swipe_anim: false,
            exif_tags: None,
            exif_search: crate::ui::textedit::TextEdit::new(),
            exif_scroll: 0.0,
            exif_error: None,
            exif_generation: 0,
            exif_blink_since: Instant::now(),
            exif_caret_on: true,
            exif_focus: false,
            exif_pending: std::collections::BTreeMap::new(),
            exif_pending_delete_gps: false,
            exif_editing: None,
            exif_editor: crate::ui::textedit::TextEdit::new(),
            exif_confirm: crate::ui::scene::ConfirmKind::None,
            exif_overwrite_mode: false,
            exif_close_after_save: false,
            exif_hovered_row: None,
            prefetch: crate::prefetch::PrefetchCache::new(512 * 1024 * 1024),
            prefetch_inflight: std::collections::HashSet::new(),
            trackpad_accum: 0.0,
        }
    }
}

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    state: AppState,
    proxy: EventLoopProxy<UserEvent>,
    initial_path: Option<PathBuf>,
    /// Наблюдатель за открытой папкой (Drop останавливает поток); пересоздаётся при смене папки.
    _watcher: Option<crate::watcher::FolderWatcher>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>, initial_path: Option<PathBuf>) -> Self {
        Self {
            window: None,
            renderer: None,
            state: AppState::new(),
            proxy,
            initial_path,
            _watcher: None,
        }
    }

    /// Запустить декод файла по индексу каталога на rayon.
    fn load_current(&mut self) {
        let Some(catalog) = &self.state.catalog else { return };
        if catalog.is_empty() {
            return;
        }
        let path = catalog.current_path().to_path_buf();
        // Восстанавливаем сохранённый ручной трансформ ДО инкремента generation: при первом
        // кадре новой генерации вид re-init'ится (set_fit + rotation-aware fit_zoom уже с учётом
        // этого поворота), поэтому масштаб/центрирование пересчитаются корректно.
        let saved = self.state.transforms.get(&path).copied().unwrap_or_default();
        self.state.view.set_transform(saved);
        self.state.generation += 1;
        let generation = self.state.generation;
        let proxy = self.proxy.clone();
        if let Some(w) = &self.window {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let title = match crate::exif::read_model(&path) {
                Some(model) => format!("{name} · {model} · Lumina"),
                None => format!("{name} · Lumina"),
            };
            self.state.ui.title = title.clone();
            w.set_title(&title); // системный заголовок (таскбар/alt-tab)
            w.request_redraw();
        }
        // Списки каталога для карусели и активный индекс
        if let Some(cat) = &self.state.catalog {
            let files = cat.files();
            let n = files.len();
            self.state.ui.thumb_count = n;
            self.state.ui.active_index = cat.current_index();
            let idx = cat.current_index();
            self.state.ui.can_prev = idx > 0;
            self.state.ui.can_next = idx + 1 < n;
            // Сохраняем уже вычисленные аспекты при навигации внутри папки (n не меняется);
            // сброс к дефолту — только при смене набора файлов (open_file чистит список).
            reconcile_aspects(&mut self.state.thumb_aspects, n);
            self.state.raw_flags = files
                .iter()
                .map(|p| RawDecoder::supports(&crate::decoder::ext_lower(p)))
                .collect();
            self.state.badge_labels = files
                .iter()
                .map(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.to_uppercase()).unwrap_or_default())
                .collect();
        }
        // Быстрый путь: сосед уже в префетч-кэше → показать мгновенно, без декода.
        if let Some(img) = self.state.prefetch.get(&path) {
            self.show_image(generation, &img);
            self.prefetch_neighbors();
            return;
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
        let fit = self.state.view.fit_zoom(win, img);
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
        let fit = self.state.view.fit_zoom(r.viewer_size(), img);
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

    /// Исполнить действие, замапленное с клавиатуры (см. input::map_key).
    fn apply_action(&mut self, action: crate::input::Action) {
        use crate::input::Action;
        match action {
            Action::RotateCw => { self.state.view.rotate_cw(); self.after_transform_change(); }
            Action::RotateCcw => { self.state.view.rotate_ccw(); self.after_transform_change(); }
            Action::FlipH => { self.state.view.flip_horizontal(); self.after_transform_change(); }
            Action::FlipV => { self.state.view.flip_vertical(); self.after_transform_change(); }
            Action::ResetTransform => { self.state.view.reset_transform(); self.after_transform_change(); }
            Action::ToggleFullscreen => self.toggle_fullscreen(),
            Action::FitView => self.set_fit_view(),
            Action::ActualSize => self.set_actual_size(),
            Action::ToggleExif => self.toggle_exif(),
        }
    }

    /// Открыть/закрыть EXIF popup. При открытии — async-чтение тегов через exiftool.
    fn toggle_exif(&mut self) {
        let open = !self.state.ui.exif_open;
        self.state.ui.exif_open = open;
        if open {
            self.state.exif_tags = None;
            self.state.exif_error = None;
            self.state.exif_scroll = 0.0;
            self.state.exif_search.clear();
            self.state.exif_blink_since = Instant::now();
            self.state.exif_caret_on = true;
            self.state.exif_focus = true; // при открытии поле сразу в фокусе — можно печатать
            self.state.exif_pending.clear();
            self.state.exif_pending_delete_gps = false;
            self.state.exif_editing = None;
            self.state.exif_editor.clear();
            self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
            self.state.exif_overwrite_mode = false;
            self.state.exif_close_after_save = false;
            self.state.exif_hovered_row = None;
            let Some(cat) = &self.state.catalog else { return };
            let path = cat.current_path().to_path_buf();
            self.state.exif_generation += 1;
            let generation = self.state.exif_generation;
            let proxy = self.proxy.clone();
            rayon::spawn(move || {
                let result = crate::exif::read::read_tags(&path).map_err(|e| e.to_string());
                let _ = proxy.send_event(UserEvent::ExifLoaded { generation, result });
            });
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Собрать pending в Vec<TagEdit> и записать через exiftool на rayon.
    fn exif_save(&mut self, mode: crate::exif::write::WriteMode) {
        use crate::exif::tags::TagEdit;
        let mut edits: Vec<TagEdit> = Vec::new();
        if self.state.exif_pending_delete_gps {
            edits.push(TagEdit::DeleteAllGps);
        }
        for ((group, tag), op) in &self.state.exif_pending {
            match op {
                crate::ui::scene::PendingOp::Set(v) => edits.push(TagEdit::Set { group: group.clone(), tag: tag.clone(), value: v.clone() }),
                crate::ui::scene::PendingOp::Delete => edits.push(TagEdit::Delete { group: group.clone(), tag: tag.clone() }),
            }
        }
        if edits.is_empty() {
            return;
        }
        let Some(cat) = &self.state.catalog else { return };
        let path = cat.current_path().to_path_buf();
        self.state.exif_generation += 1;
        let generation = self.state.exif_generation;
        let proxy = self.proxy.clone();
        rayon::spawn(move || {
            let result = crate::exif::write::write_edits(&path, &edits, mode).map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::ExifSaved { generation, result });
        });
    }

    /// Стереть все метаданные текущего файла (необратимо) на rayon.
    fn exif_strip_all(&mut self) {
        let Some(cat) = &self.state.catalog else { return };
        let path = cat.current_path().to_path_buf();
        self.state.exif_generation += 1;
        let generation = self.state.exif_generation;
        let proxy = self.proxy.clone();
        rayon::spawn(move || {
            let result = crate::exif::write::strip_all(&path).map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::ExifSaved { generation, result });
        });
    }

    /// Завершить инлайн-редактирование: записать значение в pending ТОЛЬКО при реальном
    /// изменении. Если значение совпало с исходным (вход в поле и выход без правки) — снять
    /// возможную прежнюю правку, чтобы поле не подсвечивалось как изменённое.
    fn exif_commit_edit(&mut self) {
        if let Some((g, t)) = self.state.exif_editing.take() {
            let v = self.state.exif_editor.text();
            let original = self
                .state
                .exif_tags
                .as_ref()
                .and_then(|tags| crate::exif::tags::get(tags, &g, &t))
                .unwrap_or_default();
            if v == original {
                self.state.exif_pending.remove(&(g, t));
            } else {
                self.state.exif_pending.insert((g, t), crate::ui::scene::PendingOp::Set(v));
            }
        }
    }

    fn exif_has_pending(&self) -> bool {
        !self.state.exif_pending.is_empty() || self.state.exif_pending_delete_gps
    }

    /// Прочитать текст из буфера обмена (None при ошибке/пустом).
    fn clipboard_get() -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok().filter(|s| !s.is_empty())
    }
    /// Положить текст в буфер обмена (ошибки игнорируем).
    fn clipboard_set(text: &str) {
        if !text.is_empty() {
            if let Ok(mut c) = arboard::Clipboard::new() {
                let _ = c.set_text(text.to_string());
            }
        }
    }

    /// После ручной трансформации: запомнить по пути, пересчитать fit/clamp, перерисовать.
    fn after_transform_change(&mut self) {
        if let Some(cat) = &self.state.catalog {
            let path = cat.current_path().to_path_buf();
            self.state.transforms.insert(path, self.state.view.transform());
        }
        if let Some(r) = &self.renderer {
            if let Some(img) = r.image_size() {
                let win = r.viewer_size();
                let fit = self.state.view.fit_zoom(win, img);
                self.state.view.set_min_zoom(fit);
                if self.state.view.is_fit() {
                    self.state.view.set_zoom_immediate(fit);
                    self.state.view.set_pan(glam::Vec2::ZERO);
                }
                self.state.view.clamp_pan(win, img);
            }
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Открыть файл: построить каталог его папки и начать загрузку.
    pub fn open_file(&mut self, path: PathBuf) {
        match FolderCatalog::open(&path) {
            Ok(cat) => {
                self.state.catalog = Some(cat);
                // Наблюдать за папкой открытого файла (авто-обновление каталога).
                if let Some(dir) = path.parent() {
                    self._watcher = crate::watcher::FolderWatcher::watch(dir, self.proxy.clone());
                }
                self.state.view = ViewTransform::new();
                self.state.inited_generation = None;
                self.state.thumbs.reset();
                self.state.prefetch.clear();
                self.state.prefetch_inflight.clear();
                self.state.transforms.clear();
                // сброс аспектов миниатюр: смена папки → reconcile_aspects пере-инициализирует
                self.state.thumb_aspects.clear();
                self.state.ui.scroll = 0.0;
                self.load_current();
            }
            Err(e) => log::warn!("не удалось открыть папку для {path:?}: {e}"),
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.state.ui.fullscreen = !self.state.ui.fullscreen;
        let on = self.state.ui.fullscreen;
        // при входе показываем оверлей и заводим таймер простоя (погаснет через 3 с)
        self.state.cursor_idle = 0.0;
        self.state.ui.fs_overlay = if on { 1.0 } else { 0.0 };
        // Сообщаем wndproc состояние ДО set_fullscreen, чтобы WM_NCCALCSIZE во время
        // перехода сразу отдавал согласованный размер клиента (иначе рассинхрон → resize-петля).
        #[cfg(windows)]
        crate::platform::windows::set_fullscreen(on);
        if let Some(w) = &self.window {
            let fs = if on {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                None
            };
            w.set_fullscreen(fs);
            w.request_redraw();
        }
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

    /// Установить показываемый кадр в рендерер и (на первом кадре генерации) инициализировать вид.
    /// Единый путь и для результата декода (`Decoded`), и для хита префетч-кэша.
    fn show_image(&mut self, generation: u64, img: &DecodedImage) {
        let new_img = glam::Vec2::new(img.width as f32, img.height as f32);
        if let Some(r) = &mut self.renderer {
            let old_img = r.image_size();
            r.upload_texture(&img.rgba, img.width, img.height);
            let win = r.viewer_size();
            if self.state.inited_generation != Some(generation) {
                // первый кадр этой генерации → инициализация вида (как v0.1)
                let z = self.state.view.fit_zoom(win, new_img);
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

    /// Запустить фоновый декод соседей (±1, ±2), которых ещё нет в кэше и не в работе.
    fn prefetch_neighbors(&mut self) {
        let Some(cat) = &self.state.catalog else { return };
        let cur = cat.current_index();
        let files = cat.files();
        let n = files.len();
        let mut targets: Vec<PathBuf> = Vec::new();
        for d in [1usize, 2usize] {
            if cur + d < n { targets.push(files[cur + d].to_path_buf()); }
            if cur >= d { targets.push(files[cur - d].to_path_buf()); }
        }
        for path in targets {
            if self.state.prefetch.contains(&path) || self.state.prefetch_inflight.contains(&path) {
                continue;
            }
            self.state.prefetch_inflight.insert(path.clone());
            let proxy = self.proxy.clone();
            let ext = crate::decoder::ext_lower(&path);
            rayon::spawn(move || {
                let Some(decoder) = crate::decoder::decoder_for(&ext) else { return };
                if let Ok(img) = decoder.decode_full(&path) {
                    let _ = proxy.send_event(UserEvent::Prefetched { path, image: img });
                }
            });
        }
    }

    /// Запросить декод миниатюр для индексов окна, которых ещё нет.
    /// Троттлинг: не более `MAX_INFLIGHT` одновременных декодов — полноразмерный
    /// декод (напр. 40MP ≈ 160 МБ) тяжёл, шторм из десятка сразу подвешивал UI.
    /// Остальные подхватятся на следующих кадрах (каждое завершение → redraw).
    fn request_thumbnails(&mut self, window: Vec<usize>) {
        const MAX_INFLIGHT: usize = 4;
        if self.state.catalog.is_none() {
            return;
        }
        let generation = self.state.thumbs.generation;
        let scale = self.state.scale;
        let th = (crate::ui::theme::THUMB_H * scale).round() as u32;
        for index in window {
            if self.state.thumbs_in_flight >= MAX_INFLIGHT {
                break;
            }
            if self.state.thumbs.state(index).is_some() {
                continue; // уже запрошена/готова/неудачна
            }
            let path = match self.state.catalog.as_ref().and_then(|c| c.files().get(index)) {
                Some(p) => p.to_path_buf(),
                None => continue,
            };
            let _ = self.state.thumbs.take_pending(&[index]); // пометить Loading
            self.state.thumbs_in_flight += 1;
            let proxy = self.proxy.clone();
            let ext = crate::decoder::ext_lower(&path);
            rayon::spawn(move || {
                // Ключ кэша по метаданным файла (mtime+size инвалидируют при правке).
                let key = std::fs::metadata(&path).ok().map(|md| {
                    let mtime = md.modified().ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    crate::thumbcache::cache_key(&path, mtime, md.len(), th)
                });
                let dir = crate::thumbcache::cache_dir();
                // 1) попытка из дискового кэша
                if let Some(k) = &key {
                    if let Some(img) = crate::thumbcache::load(&dir, k) {
                        let _ = proxy.send_event(UserEvent::Thumbnail {
                            generation, index, rgba: img.rgba, w: img.width, h: img.height,
                        });
                        return;
                    }
                }
                // 2) промах → декод (встроенное превью если есть, иначе полный) + ресайз + запись в кэш
                let Some(decoder) = crate::decoder::decoder_for(&ext) else { return };
                let decoded = match decoder.decode_preview(&path) {
                    Ok(Some(img)) => Some(img),
                    _ => decoder.decode_full(&path).ok(),
                };
                let Some(img) = decoded else {
                    let _ = proxy.send_event(UserEvent::Thumbnail { generation, index, rgba: Vec::new(), w: 0, h: 0 });
                    return;
                };
                let (rgba, w, h) = crate::app::make_thumb(&img.rgba, img.width, img.height, th);
                if let Some(k) = &key {
                    crate::thumbcache::store(&dir, k, &DecodedImage { rgba: rgba.clone(), width: w, height: h });
                }
                let _ = proxy.send_event(UserEvent::Thumbnail { generation, index, rgba, w, h });
            });
        }
    }
}

/// Иконка окна из встроенного PNG (256×256). None при ошибке декода.
fn window_icon() -> Option<winit::window::Icon> {
    let bytes = include_bytes!("../assets/icon/lumina-256.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
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

/// Согласовать длину списка аспектов миниатюр с числом файлов каталога `n`.
/// При совпадении длины СОХРАНЯЕТ значения: навигация внутри папки не должна
/// сбрасывать уже вычисленные аспекты к дефолту — декодированные миниатюры
/// повторно не декодируются (троттлинг в `request_thumbnails`), значит обновить
/// аспект назад было бы некому, и портретные миниатюры «расплылись» бы в landscape.
/// При расхождении (другая папка, см. сброс в `open_file`) — пере-инициализация дефолтом.
fn reconcile_aspects(aspects: &mut Vec<f32>, n: usize) {
    if aspects.len() != n {
        aspects.clear();
        aspects.resize(n, crate::ui::theme::THUMB_DEFAULT_AR);
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = Window::default_attributes()
            .with_title("Lumina")
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        if let Some(icon) = window_icon() {
            attrs = attrs.with_window_icon(Some(icon));
        }
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

        // Разовая эвикция дискового кэша миниатюр (фоном, бюджет 256 МБ).
        rayon::spawn(|| {
            const THUMB_CACHE_BUDGET: u64 = 256 * 1024 * 1024;
            crate::thumbcache::prune(&crate::thumbcache::cache_dir(), THUMB_CACHE_BUDGET);
        });

        // Открыть стартовый файл, если был передан.
        if let Some(path) = self.initial_path.take() {
            self.open_file(path);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Пока EXIF popup открыт — гоним мигание каретки: будим цикл к следующей смене фазы
        // и перерисовываем только когда фаза перевернулась (а не каждый кадр).
        if self.state.ui.exif_open && (self.state.exif_focus || self.state.exif_editing.is_some()) {
            let blink = (Instant::now() - self.state.exif_blink_since).as_secs_f32();
            let visible = ((blink / crate::ui::theme::POPUP_CARET_BLINK) as u64) % 2 == 0;
            if visible != self.state.exif_caret_on {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            let into_phase = blink % crate::ui::theme::POPUP_CARET_BLINK;
            let wait = (crate::ui::theme::POPUP_CARET_BLINK - into_phase).max(0.02);
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + std::time::Duration::from_secs_f32(wait),
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
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
                        self.show_image(generation, &img);
                        // Полноразмерный кадр → в префетч-кэш + упредить соседей.
                        // Превью не кэшируем (это не full-res).
                        if stage == Stage::Full {
                            if let Some(cat) = &self.state.catalog {
                                let p = cat.current_path().to_path_buf();
                                self.state.prefetch.insert(p, std::sync::Arc::new(img));
                            }
                            self.prefetch_neighbors();
                        }
                    }
                    Err(e) => {
                        // full упал; если preview уже показан (inited) — просто остаёмся на нём
                        log::warn!("декодирование ({}) не удалось: {e}",
                            if stage == Stage::Preview { "preview" } else { "full" });
                    }
                }
            }
            UserEvent::Prefetched { path, image } => {
                self.state.prefetch_inflight.remove(&path);
                self.state.prefetch.insert(path, std::sync::Arc::new(image));
                // вид не трогаем: это лишь наполнение горячего кэша
            }
            UserEvent::FolderChanged => {
                let changed = self.state.catalog.as_mut().map(|c| c.refresh().unwrap_or(false)).unwrap_or(false);
                if changed {
                    // индексы съехали → сбросить миниатюры (видимые перезапросятся) и аспекты;
                    // префетч-кэш по путям остаётся валиден, но in-flight по старым индексам сбрасываем;
                    // load_current пере-вычислит все производные UI-данные и перезагрузит текущий кадр.
                    self.state.thumb_aspects.clear();
                    self.state.thumbs.reset();
                    self.state.prefetch.clear();
                    self.state.prefetch_inflight.clear();
                    self.load_current();
                }
            }
            UserEvent::Thumbnail { generation, index, rgba, w, h } => {
                // каждый спавн декода шлёт ровно одно событие → освобождаем слот троттлинга
                self.state.thumbs_in_flight = self.state.thumbs_in_flight.saturating_sub(1);
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
            UserEvent::ExifLoaded { generation, result } => {
                if generation != self.state.exif_generation || !self.state.ui.exif_open {
                    return; // устаревший результат или popup уже закрыт
                }
                match result {
                    Ok(tags) => { self.state.exif_tags = Some(tags); self.state.exif_error = None; }
                    Err(e) => { self.state.exif_tags = None; self.state.exif_error = Some(e); }
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            UserEvent::ExifSaved { generation, result } => {
                if generation != self.state.exif_generation || !self.state.ui.exif_open {
                    return;
                }
                match result {
                    Ok(()) => {
                        // успех: очистить правки, перечитать теги, обновить заголовок
                        self.state.exif_pending.clear();
                        self.state.exif_pending_delete_gps = false;
                        self.state.exif_editing = None;
                        self.state.exif_error = None;
                        self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
                        self.state.exif_overwrite_mode = false; // безопасность: следующее сохранение снова осознанное
                        if self.state.exif_close_after_save {
                            self.state.exif_close_after_save = false;
                            self.toggle_exif(); // закрыть после сохранения по подтверждению
                            return;
                        }
                        if let Some(cat) = &self.state.catalog {
                            let path = cat.current_path().to_path_buf();
                            // обновить заголовок при смене Model
                            if let Some(w) = &self.window {
                                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                                let title = match crate::exif::read_model(&path) {
                                    Some(m) => format!("{name} · {m} · Lumina"),
                                    None => format!("{name} · Lumina"),
                                };
                                w.set_title(&title);
                            }
                            self.state.exif_generation += 1;
                            let generation = self.state.exif_generation;
                            let proxy = self.proxy.clone();
                            rayon::spawn(move || {
                                let result = crate::exif::read::read_tags(&path).map_err(|e| e.to_string());
                                let _ = proxy.send_event(UserEvent::ExifLoaded { generation, result });
                            });
                        }
                    }
                    Err(e) => { self.state.exif_error = Some(e); }
                }
                if let Some(w) = &self.window { w.request_redraw(); }
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
                        let z = self.state.view.fit_zoom(r.viewer_size(), img);
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

                // экранные стрелки: альфа к 1 при hover полосы (если можно листать), иначе к 0
                let nav_targets = [
                    if self.state.ui.hovered == hit::Region::NavPrev && self.state.ui.can_prev { 1.0 } else { 0.0 },
                    if self.state.ui.hovered == hit::Region::NavNext && self.state.ui.can_next { 1.0 } else { 0.0 },
                ];
                let mut nav_animating = false;
                for i in 0..2 {
                    let a = self.state.ui.nav_alpha[i];
                    let t = nav_targets[i];
                    if (a - t).abs() > 0.001 {
                        self.state.ui.nav_alpha[i] = a + (t - a) * (dt / 0.12).min(1.0);
                        nav_animating = true;
                    } else {
                        self.state.ui.nav_alpha[i] = t;
                    }
                }

                // свайп: плавный откат смещения к 0, если перелистывания не случилось
                if self.state.swipe_anim {
                    let cur = self.state.view.swipe_offset();
                    let next = cur + (0.0 - cur) * (dt / 0.2).min(1.0);
                    if next.abs() < 0.5 {
                        self.state.view.set_swipe_offset(0.0);
                        self.state.swipe_anim = false;
                    } else {
                        self.state.view.set_swipe_offset(next);
                    }
                }

                // fullscreen-оверлей: показан при движении курсора, плавно гаснет после 3 с простоя
                let mut overlay_active = false;
                if self.state.ui.fullscreen {
                    self.state.cursor_idle += dt;
                    if self.state.cursor_idle >= 3.0 {
                        // плавное гашение (~0.4 с)
                        self.state.ui.fs_overlay = (self.state.ui.fs_overlay - dt / 0.4).max(0.0);
                    }
                    // активно (нужны перерисовки), пока идёт отсчёт до 3 с или продолжается фейд
                    overlay_active = self.state.cursor_idle < 3.0 || self.state.ui.fs_overlay > 0.0;
                } else {
                    self.state.ui.fs_overlay = 0.0;
                    self.state.cursor_idle = 0.0;
                }

                let prep = self.renderer.as_ref().map(|r| {
                    let win = r.surface_size();
                    let l = layout::compute(win, self.state.scale, self.state.ui.bottom_factor, self.state.ui.fullscreen);
                    let thumb_rects = layout::carousel_thumb_rects(l.carousel, &self.state.thumb_aspects, self.state.ui.scroll, self.state.scale);
                    (l, thumb_rects, win)
                });
                if let Some((l, thumb_rects, r_surface_size)) = prep {
                    let window: Vec<usize> = thumb_rects.iter().map(|(i, _)| *i).collect();
                    self.request_thumbnails(window);
                    let bottom_chrome = if self.state.ui.fullscreen { 0.0 } else { l.divider.h + l.bottom_bar.h };
                    let tb = if self.state.ui.fullscreen { 0.0 } else { theme::TITLEBAR_HEIGHT * self.state.scale };
                    let mut cmds = scene::build(&self.state.ui, &l, &self.state.theme, self.state.scale, &thumb_rects, &self.state.raw_flags);
                    if self.state.ui.exif_open {
                        let filename = self.state.catalog.as_ref()
                            .and_then(|c| c.current_path().file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
                            .unwrap_or_default();
                        // Позиция каретки/выделения поля поиска — измеряем шрифтом текстового слоя.
                        let font_px = theme::POPUP_ROW_SIZE * self.state.scale;
                        let chars: Vec<char> = self.state.exif_search.text().chars().collect();
                        let caret = self.state.exif_search.caret().min(chars.len());
                        let before: String = chars[..caret].iter().collect();
                        let sel_strs = self.state.exif_search.selection().map(|(a, b)| {
                            (chars[..a].iter().collect::<String>(), chars[..b].iter().collect::<String>())
                        });
                        let (caret_px, sel_px) = if let Some(r) = self.renderer.as_mut() {
                            let cp = r.measure_text_width(&before, font_px);
                            let sp = sel_strs
                                .as_ref()
                                .map(|(sa, sb)| (r.measure_text_width(sa, font_px), r.measure_text_width(sb, font_px)));
                            (cp, sp)
                        } else {
                            (0.0, None)
                        };
                        // если редактируем — каретка/метрики для редактора, поиск без фокуса
                        let editing_active = self.state.exif_editing.is_some();
                        let (ed_caret_px, ed_sel_px) = if editing_active {
                            let chars: Vec<char> = self.state.exif_editor.text().chars().collect();
                            let caret = self.state.exif_editor.caret().min(chars.len());
                            let before: String = chars[..caret].iter().collect();
                            let sel = self.state.exif_editor.selection().map(|(a, b)| (chars[..a].iter().collect::<String>(), chars[..b].iter().collect::<String>()));
                            if let Some(r) = self.renderer.as_mut() {
                                let cp = r.measure_text_width(&before, font_px);
                                let sp = sel.as_ref().map(|(sa, sb)| (r.measure_text_width(sa, font_px), r.measure_text_width(sb, font_px)));
                                (cp, sp)
                            } else { (0.0, None) }
                        } else { (0.0, None) };
                        // фаза мигания каретки от времени с последнего ввода/открытия (в фокусе или при редакторе)
                        let blink = (now - self.state.exif_blink_since).as_secs_f32();
                        let phase_on = ((blink / theme::POPUP_CARET_BLINK) as u64) % 2 == 0;
                        let focused = self.state.exif_focus && !editing_active;
                        let caret_visible = (self.state.exif_focus || editing_active) && phase_on;
                        self.state.exif_caret_on = caret_visible;
                        let editing_ref = self.state.exif_editing.as_ref().map(|(g, t)| (g.as_str(), t.as_str()));
                        let edit_state = crate::ui::scene::PopupEditState {
                            pending: &self.state.exif_pending,
                            delete_gps: self.state.exif_pending_delete_gps,
                            editing: editing_ref,
                            editor: &self.state.exif_editor,
                            editor_caret_px: ed_caret_px,
                            editor_sel_px: ed_sel_px,
                            hovered_row: self.state.exif_hovered_row,
                            confirm: self.state.exif_confirm,
                            overwrite_mode: self.state.exif_overwrite_mode,
                            has_pending: !self.state.exif_pending.is_empty() || self.state.exif_pending_delete_gps,
                        };
                        cmds.extend(scene::build_popup(
                            r_surface_size,
                            self.state.scale,
                            &self.state.theme,
                            &filename,
                            self.state.exif_tags.as_ref(),
                            &self.state.exif_search,
                            self.state.exif_scroll,
                            self.state.exif_error.as_deref(),
                            caret_px,
                            sel_px,
                            caret_visible,
                            focused,
                            &edit_state,
                        ));
                    }
                    if let Some(r) = &mut self.renderer {
                        r.set_bottom_chrome_height(bottom_chrome);
                        r.set_titlebar_height(tb);
                        // миниатюры клипуются зоной карусели (без остатков при сворачивании)
                        r.set_thumb_clip(l.carousel);
                        // viewer меняется при сворачивании bottom bar → фото заполняет освободившееся место
                        if let Some(img) = r.image_size() {
                            let z = self.state.view.fit_zoom(r.viewer_size(), img);
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
                if self.state.view.is_animating() || animating || overlay_active || nav_animating || self.state.swipe_anim {
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
                } else if self.state.swiping {
                    let dx = pos.x - self.state.swipe_press_x;
                    self.state.view.set_swipe_offset(dx);
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
                self.state.last_cursor = pos;
                self.state.cursor = pos;
                // EXIF popup открыт — считаем строку под курсором (для ✎/✕ и hover GPS).
                if self.state.ui.exif_open {
                    if let (Some(r), Some(tags)) = (&self.renderer, &self.state.exif_tags) {
                        let win = r.surface_size();
                        let body = layout::popup_layout(win, self.state.scale).body;
                        let rows = scene::popup_rows(tags, &self.state.exif_search.text(), self.state.scale, self.state.exif_scroll, body);
                        self.state.exif_hovered_row = rows.iter().position(|row| self.state.cursor.y >= row.y && self.state.cursor.y < row.y + row.h && body.contains(self.state.cursor));
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                    return;
                }
                // движение курсора → мгновенно показать fullscreen-оверлей и сбросить таймер простоя
                if self.state.ui.fullscreen {
                    self.state.cursor_idle = 0.0;
                    self.state.ui.fs_overlay = 1.0;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
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
                // EXIF popup: колесо скроллит список тегов.
                if self.state.ui.exif_open {
                    if let (Some(r), Some(tags)) = (&self.renderer, &self.state.exif_tags) {
                        let win = r.surface_size();
                        let p = layout::popup_layout(win, self.state.scale);
                        let content = scene::popup_content_height(tags, &self.state.exif_search, self.state.scale);
                        let max_scroll = (content - p.body.h).max(0.0);
                        let step = 40.0 * self.state.scale;
                        self.state.exif_scroll = (self.state.exif_scroll - lines * step).clamp(0.0, max_scroll);
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                    return;
                }
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
                    // Тачпад: горизонтально-доминирующий PixelDelta → пошаговая навигация.
                    if let winit::event::MouseScrollDelta::PixelDelta(p) = delta {
                        let dx = p.x as f32;
                        let dy = p.y as f32;
                        let steps = crate::input::on_trackpad_pan(
                            &mut self.state.trackpad_accum, dx, dy, crate::input::TRACKPAD_NAV_STEP_PX);
                        if steps != 0 {
                            // максимум один шаг за событие (инерционный бросок не пролистывает пачку)
                            self.navigate(if steps > 0 { 1 } else { -1 });
                            return;
                        }
                        // горизонтальный жест без полного шага — поглощаем (не зумим)
                        if dx.abs() >= dy.abs() && dx != 0.0 {
                            return;
                        }
                    }
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
                    // EXIF popup перехватывает клики целиком.
                    if self.state.ui.exif_open {
                        if state == ElementState::Pressed {
                            let win = self.renderer.as_ref().map(|r| r.surface_size()).unwrap_or(glam::Vec2::ZERO);
                            let filter = self.state.exif_search.text();
                            let empty = crate::exif::tags::ExifTags::default();
                            let tags = self.state.exif_tags.as_ref().unwrap_or(&empty);
                            let region = hit::hit_popup_edit(win, self.state.scale, self.state.cursor, tags, &filter, self.state.exif_scroll, self.state.exif_confirm, self.state.exif_overwrite_mode);
                            // строки (owned) — чтобы завершить borrow tags до мутаций self
                            let rows = crate::ui::scene::popup_rows(tags, &filter, self.state.scale, self.state.exif_scroll, layout::popup_layout(win, self.state.scale).body);
                            // активный редактор: клик вне него — коммит
                            if self.state.exif_editing.is_some() && !matches!(region, hit::PopupRegion::Search) {
                                self.exif_commit_edit();
                            }
                            match region {
                                hit::PopupRegion::Close | hit::PopupRegion::Outside => {
                                    if self.exif_has_pending() { self.state.exif_confirm = crate::ui::scene::ConfirmKind::CloseWithPending; }
                                    else { self.toggle_exif(); }
                                }
                                hit::PopupRegion::Search => {
                                    self.state.exif_focus = true;
                                    self.state.exif_blink_since = Instant::now();
                                    self.state.exif_caret_on = true;
                                }
                                hit::PopupRegion::Body => { self.state.exif_focus = false; }
                                hit::PopupRegion::RowEdit(i) | hit::PopupRegion::RowDelete(i) | hit::PopupRegion::GpsDeleteAll(i) => {
                                    if let Some(r) = rows.get(i) {
                                        match region {
                                            hit::PopupRegion::RowEdit(_) => {
                                                // начать редактирование: editor = текущее значение (или pending Set)
                                                let key = (r.group.clone(), r.tag.clone());
                                                let cur = match self.state.exif_pending.get(&key) {
                                                    Some(crate::ui::scene::PendingOp::Set(v)) => v.clone(),
                                                    _ => r.value.clone(),
                                                };
                                                self.state.exif_editor.set_text(&cur);
                                                self.state.exif_editing = Some(key);
                                                self.state.exif_focus = false; // фокус у редактора
                                                self.state.exif_blink_since = Instant::now();
                                                self.state.exif_caret_on = true;
                                            }
                                            hit::PopupRegion::RowDelete(_) => {
                                                let key = (r.group.clone(), r.tag.clone());
                                                // повторный ✕ снимает удаление
                                                if matches!(self.state.exif_pending.get(&key), Some(crate::ui::scene::PendingOp::Delete)) {
                                                    self.state.exif_pending.remove(&key);
                                                } else {
                                                    self.state.exif_pending.insert(key, crate::ui::scene::PendingOp::Delete);
                                                }
                                            }
                                            hit::PopupRegion::GpsDeleteAll(_) => {
                                                self.state.exif_pending_delete_gps = !self.state.exif_pending_delete_gps;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                hit::PopupRegion::FooterToggle => {
                                    self.state.exif_overwrite_mode = !self.state.exif_overwrite_mode;
                                }
                                hit::PopupRegion::FooterStrip => {
                                    self.state.exif_confirm = crate::ui::scene::ConfirmKind::StripAll;
                                }
                                hit::PopupRegion::FooterSave => {
                                    if self.state.exif_overwrite_mode && self.exif_has_pending() {
                                        self.state.exif_confirm = crate::ui::scene::ConfirmKind::OverwriteSave;
                                    } else {
                                        self.exif_save(crate::exif::write::WriteMode::Backup);
                                    }
                                }
                                hit::PopupRegion::FooterCancel => {
                                    // отменить все правки и закрыть popup
                                    self.state.exif_pending.clear();
                                    self.state.exif_pending_delete_gps = false;
                                    self.state.exif_editing = None;
                                    self.toggle_exif();
                                }
                                hit::PopupRegion::ConfirmPrimary => {
                                    match self.state.exif_confirm {
                                        crate::ui::scene::ConfirmKind::CloseWithPending => {
                                            self.state.exif_close_after_save = true;
                                            if self.state.exif_overwrite_mode {
                                                self.state.exif_confirm = crate::ui::scene::ConfirmKind::OverwriteSave; // цепочка: подтвердить необратимость
                                            } else {
                                                self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
                                                self.exif_save(crate::exif::write::WriteMode::Backup);
                                            }
                                        }
                                        crate::ui::scene::ConfirmKind::OverwriteSave => {
                                            self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
                                            self.exif_save(crate::exif::write::WriteMode::Overwrite);
                                        }
                                        crate::ui::scene::ConfirmKind::StripAll => {
                                            self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
                                            self.exif_strip_all();
                                        }
                                        crate::ui::scene::ConfirmKind::None => {}
                                    }
                                }
                                hit::PopupRegion::ConfirmSecondary => {
                                    let was = self.state.exif_confirm;
                                    self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
                                    self.state.exif_close_after_save = false;
                                    if was == crate::ui::scene::ConfirmKind::CloseWithPending { self.toggle_exif(); }
                                }
                                hit::PopupRegion::ConfirmTertiary => {
                                    self.state.exif_confirm = crate::ui::scene::ConfirmKind::None; // Продолжить
                                }
                            }
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                        return;
                    }
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
                                    // оверлейные кнопки fullscreen — только пока тулбар видим
                                    hit::Region::FullscreenExit if self.state.ui.fs_overlay > 0.1 => { self.toggle_fullscreen(); return; }
                                    hit::Region::SlideshowPlay if self.state.ui.fs_overlay > 0.1 => { return; } // инертна (slideshow — v0.6)
                                    hit::Region::ActionExif => { self.toggle_exif(); return; }
                                    hit::Region::ActionRotate => {
                                        self.state.view.rotate_cw();
                                        self.after_transform_change();
                                        return;
                                    }
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
                                    hit::Region::NavPrev => {
                                        if self.state.ui.can_prev { self.navigate(-1); }
                                        return;
                                    }
                                    hit::Region::NavNext => {
                                        if self.state.ui.can_next { self.navigate(1); }
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
                                // pan только когда фото увеличено зумом сверх fit; иначе — свайп-перелистывание
                                let can_pan = self.state.view.zoom() > self.state.view.min_zoom();
                                if can_pan {
                                    self.state.dragging = true;
                                    if let Some(w) = &self.window {
                                        w.set_cursor(winit::window::Cursor::Icon(
                                            winit::window::CursorIcon::Grabbing,
                                        ));
                                    }
                                } else {
                                    // размер по умолчанию (fit) → начинаем свайп
                                    self.state.swiping = true;
                                    self.state.swipe_press_x = self.state.cursor.x;
                                    self.state.swipe_anim = false;
                                    self.state.view.set_swipe_offset(0.0);
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
                            if self.state.swiping {
                                self.state.swiping = false;
                                let dx = self.state.view.swipe_offset();
                                let vw = self.renderer.as_ref().map(|r| r.viewer_size().x).unwrap_or(0.0);
                                match crate::input::on_swipe_release(dx, vw) {
                                    Some(crate::input::NavDir::Next) if self.state.ui.can_next => {
                                        self.state.view.set_swipe_offset(0.0);
                                        self.navigate(1);
                                    }
                                    Some(crate::input::NavDir::Prev) if self.state.ui.can_prev => {
                                        self.state.view.set_swipe_offset(0.0);
                                        self.navigate(-1);
                                    }
                                    _ => {
                                        // не дотянули / край каталога → плавный откат
                                        self.state.swipe_anim = true;
                                        if let Some(w) = &self.window { w.request_redraw(); }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{Key, NamedKey, PhysicalKey};
                if event.state.is_pressed() {
                    // Пока EXIF popup открыт — клавиатура поглощается модалом; ввод идёт в поиск
                    // только когда поле в фокусе. Шорткаты приложения не срабатывают.
                    if self.state.ui.exif_open {
                        // Esc: при активном редакторе — отмена правки; иначе закрытие popup.
                        if let Key::Named(NamedKey::Escape) = event.logical_key.as_ref() {
                            if self.state.exif_editing.is_some() {
                                self.state.exif_editing = None;
                                self.state.exif_editor.clear();
                                if let Some(w) = &self.window { w.request_redraw(); }
                                return;
                            }
                            if self.state.exif_confirm != crate::ui::scene::ConfirmKind::None {
                                self.state.exif_confirm = crate::ui::scene::ConfirmKind::None;
                                self.state.exif_close_after_save = false;
                                if let Some(w) = &self.window { w.request_redraw(); }
                                return;
                            }
                            self.toggle_exif();
                            return;
                        }
                        // Активный инлайн-редактор: Enter коммитит, иначе правим editor.
                        if self.state.exif_editing.is_some() {
                            let shift = self.state.shift_down;
                            match event.logical_key.as_ref() {
                                Key::Named(NamedKey::Enter) => { self.exif_commit_edit(); }
                                Key::Named(NamedKey::Backspace) => self.state.exif_editor.backspace(),
                                Key::Named(NamedKey::Delete) => self.state.exif_editor.delete(),
                                Key::Named(NamedKey::ArrowLeft) => self.state.exif_editor.move_left(shift),
                                Key::Named(NamedKey::ArrowRight) => self.state.exif_editor.move_right(shift),
                                Key::Named(NamedKey::Home) => self.state.exif_editor.move_home(shift),
                                Key::Named(NamedKey::End) => self.state.exif_editor.move_end(shift),
                                _ => {
                                    if self.state.ctrl_down {
                                        match event.physical_key {
                                            PhysicalKey::Code(winit::keyboard::KeyCode::KeyA) => self.state.exif_editor.select_all(),
                                            PhysicalKey::Code(winit::keyboard::KeyCode::KeyC) => { if let Some(s) = self.state.exif_editor.selected_text() { Self::clipboard_set(&s); } }
                                            PhysicalKey::Code(winit::keyboard::KeyCode::KeyX) => { if let Some(s) = self.state.exif_editor.cut() { Self::clipboard_set(&s); } }
                                            PhysicalKey::Code(winit::keyboard::KeyCode::KeyV) => { if let Some(s) = Self::clipboard_get() { self.state.exif_editor.insert_str(&s); } }
                                            _ => {}
                                        }
                                    } else if let Some(txt) = &event.text {
                                        let s: String = txt.chars().filter(|c| !c.is_control()).collect();
                                        if !s.is_empty() { self.state.exif_editor.insert_str(&s); }
                                    }
                                }
                            }
                            self.state.exif_blink_since = Instant::now();
                            self.state.exif_caret_on = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                            return;
                        }
                        // Поле без фокуса — клавиатура поглощается, но в поиск не вводится.
                        if !self.state.exif_focus {
                            return;
                        }
                        let shift = self.state.shift_down;
                        let mut handled = true;
                        match event.logical_key.as_ref() {
                            Key::Named(NamedKey::Backspace) => self.state.exif_search.backspace(),
                            Key::Named(NamedKey::Delete) => self.state.exif_search.delete(),
                            Key::Named(NamedKey::ArrowLeft) => self.state.exif_search.move_left(shift),
                            Key::Named(NamedKey::ArrowRight) => self.state.exif_search.move_right(shift),
                            Key::Named(NamedKey::Home) => self.state.exif_search.move_home(shift),
                            Key::Named(NamedKey::End) => self.state.exif_search.move_end(shift),
                            _ => {
                                // Ctrl+A — выделить всё; C/V/X — буфер обмена; печатаемый текст — вставить.
                                if self.state.ctrl_down {
                                    match event.physical_key {
                                        PhysicalKey::Code(winit::keyboard::KeyCode::KeyA) => self.state.exif_search.select_all(),
                                        PhysicalKey::Code(winit::keyboard::KeyCode::KeyC) => { if let Some(s) = self.state.exif_search.selected_text() { Self::clipboard_set(&s); } }
                                        PhysicalKey::Code(winit::keyboard::KeyCode::KeyX) => { if let Some(s) = self.state.exif_search.cut() { Self::clipboard_set(&s); } }
                                        PhysicalKey::Code(winit::keyboard::KeyCode::KeyV) => { if let Some(s) = Self::clipboard_get() { self.state.exif_search.insert_str(&s); } }
                                        _ => { handled = false; }
                                    }
                                } else if let Some(txt) = &event.text {
                                    // фильтруем управляющие символы
                                    let s: String = txt.chars().filter(|c| !c.is_control()).collect();
                                    if s.is_empty() { handled = false; } else { self.state.exif_search.insert_str(&s); }
                                } else {
                                    handled = false;
                                }
                            }
                        }
                        if handled {
                            self.state.exif_scroll = 0.0; // фильтр изменился → к началу
                            self.state.exif_blink_since = Instant::now(); // каретка видна сразу после ввода
                            self.state.exif_caret_on = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                        return; // popup поглощает клавиатуру целиком
                    }
                    // NamedKey-перехваты (сами по себе раскладко-независимы).
                    match event.logical_key.as_ref() {
                        Key::Named(NamedKey::F11) => { self.toggle_fullscreen(); return; }
                        Key::Named(NamedKey::Escape) => {
                            if self.state.ui.fullscreen { self.toggle_fullscreen(); }
                            return;
                        }
                        _ => {}
                    }
                    // Навигация стрелками/Home/End.
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
                        return;
                    }
                    // Действия по ФИЗИЧЕСКОЙ клавише (раскладко-независимо): R/H/V/F/Ctrl+Z/Ctrl+0/Ctrl+1.
                    if let PhysicalKey::Code(code) = event.physical_key {
                        if let Some(action) = crate::input::map_key(code, self.state.ctrl_down, self.state.shift_down) {
                            self.apply_action(action);
                            return;
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.state.ctrl_down = mods.state().control_key();
                self.state.shift_down = mods.state().shift_key();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::THUMB_DEFAULT_AR;

    #[test]
    fn reconcile_keeps_values_when_len_matches() {
        // навигация внутри папки (n не меняется) НЕ должна сбрасывать уже
        // вычисленные аспекты к дефолту — иначе портретные миниатюры станут landscape
        let mut a = vec![0.75, 1.333, 0.75];
        reconcile_aspects(&mut a, 3);
        assert_eq!(a, vec![0.75, 1.333, 0.75]);
    }

    #[test]
    fn reconcile_resets_on_len_change() {
        // другая папка (другое число файлов) → пере-инициализация дефолтом
        let mut a = vec![0.75, 1.333];
        reconcile_aspects(&mut a, 4);
        assert_eq!(a, vec![THUMB_DEFAULT_AR; 4]);
    }

    #[test]
    fn reconcile_from_empty_builds_defaults() {
        let mut a: Vec<f32> = Vec::new();
        reconcile_aspects(&mut a, 2);
        assert_eq!(a, vec![THUMB_DEFAULT_AR; 2]);
    }
}
