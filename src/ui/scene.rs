//! Построение списка draw-команд titlebar и bottom bar из состояния. Чистое, без GPU.

use crate::ui::hit::Region;
use crate::ui::layout::{Rect, UiLayout};
use crate::ui::theme::{self, ThemePalette};

/// Глифы Segoe MDL2 Assets для кнопок окна.
pub const GLYPH_MINIMIZE: char = '\u{E921}'; // ChromeMinimize
pub const GLYPH_MAXIMIZE: char = '\u{E922}'; // ChromeMaximize
pub const GLYPH_RESTORE: char = '\u{E923}';  // ChromeRestore
pub const GLYPH_CLOSE: char = '\u{E8BB}';    // ChromeClose
pub const GLYPH_EDIT: char = '\u{E70F}';   // Segoe MDL2 Edit (карандаш)
pub const GLYPH_DELETE: char = '\u{E74D}'; // Segoe MDL2 Delete (корзина)

/// Семейство шрифта кнопок окна.
pub const ICON_FONT_FAMILY: &str = "Segoe MDL2 Assets";

/// Семейство шрифта иконок действий (Tabler Icons 3.44.0).
pub const TABLER_FONT_FAMILY: &str = "tabler-icons";

/// Глифы Tabler (кодпоинты из tabler-icons.css 3.44.0).
pub const GLYPH_FULLSCREEN: char = '\u{EAEA}'; // ti-maximize
pub const GLYPH_INFO: char = '\u{EAC5}';       // ti-info-circle
pub const GLYPH_ROTATE_CW: char = '\u{EB15}';  // ti-rotate-clockwise
pub const GLYPH_PLAY: char = '\u{ED46}';       // ti-player-play
pub const GLYPH_FS_EXIT: char = '\u{EA29}';    // ti-arrows-minimize (выход из fullscreen)
pub const GLYPH_CHEVRON_LEFT: char = '\u{EA60}';  // ti-chevron-left
pub const GLYPH_CHEVRON_RIGHT: char = '\u{EA61}'; // ti-chevron-right

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconFont {
    WindowMdl2,
    Tabler,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
}

/// Слой прямоугольника относительно миниатюр карусели.
/// `Bg` рисуется ДО миниатюр (подложки), `Overlay` — ПОСЛЕ (рамка/бейджи поверх фото).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RectLayer {
    Bg,
    Overlay,
}

#[derive(Clone, Debug)]
pub enum DrawCmd {
    Rect { rect: Rect, color: [f32; 4], radius: f32, layer: RectLayer },
    /// `clip` — необязательная область отсечения текста; `None` → клип по собственному `rect`.
    /// Используется popup'ом: строки тела клипуются по области `body` (при скролле не лезут
    /// на поле поиска сверху / под карточку снизу).
    Text { rect: Rect, text: String, size: f32, color: [f32; 4], align: Align, clip: Option<Rect> },
    Icon { rect: Rect, glyph: char, size: f32, color: [f32; 4], font: IconFont },
}

/// Мета-информация о файле (из заголовка/ФС, не из EXIF).
#[derive(Clone, Debug, PartialEq)]
pub struct FileMeta {
    pub format_label: String, // "RAF · RAW"
    pub megapixels: f32,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

/// Строки мета-панели (без заголовков, в столбик): формат, разрешение, размер файла.
/// Напр.: ["JPG", "1920×1280px", "12.4MB"].
pub fn meta_lines(m: &FileMeta) -> Vec<String> {
    vec![
        m.format_label.clone(),
        format!("{}×{}px", m.width, m.height),
        humanize_bytes(m.bytes),
    ]
}

/// Размер в байтах → человекочитаемо ("12.4MB").
pub fn humanize_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1}MB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.1}KB", b / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

/// Состояние UI для рендера titlebar и bottom bar.
pub struct UiState {
    pub title: String,
    pub hovered: Region,
    pub maximized: bool,
    pub bottom_visible: bool, // цель toggle
    pub bottom_factor: f32,   // анимированная видимость 0..1
    pub fullscreen: bool,
    /// Прозрачность оверлейного тулбара fullscreen (0..1): 1 при движении курсора,
    /// плавно гаснет до 0 после простоя. Вне fullscreen — 0.
    pub fs_overlay: f32,
    pub meta: Option<FileMeta>,
    pub thumb_count: usize,
    pub active_index: usize,
    pub scroll: f32,
    /// EXIF popup открыт.
    pub exif_open: bool,
    /// Прозрачность экранных стрелок [prev, next] (0..1): к 1 при hover, иначе к 0.
    pub nav_alpha: [f32; 2],
    /// Можно ли листать prev/next (нет первого/последнего — стрелка скрыта и инертна).
    pub can_prev: bool,
    pub can_next: bool,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            hovered: Region::None,
            maximized: false,
            bottom_visible: true,
            bottom_factor: 1.0,
            fullscreen: false,
            fs_overlay: 0.0,
            meta: None,
            thumb_count: 0,
            active_index: 0,
            scroll: 0.0,
            exif_open: false,
            nav_alpha: [0.0, 0.0],
            can_prev: false,
            can_next: false,
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Построить draw-команды. `scale` — для перевода кеглей в физ. px.
/// Активная рамка миниатюры и бейджи — поверх миниатюр (рисуются позже текстуры в рендере).
pub fn build(
    state: &UiState,
    layout: &UiLayout,
    theme: &ThemePalette,
    scale: f32,
    thumb_rects: &[(usize, Rect)],
    raw_flags: &[bool],
) -> Vec<DrawCmd> {
    let mut cmds = Vec::new();

    // В fullscreen хрома нет, но есть оверлейный тулбар: [play] [выход] справа-сверху.
    // Прозрачность всего тулбара — fs_overlay (показ при движении, плавное гашение).
    if state.fullscreen {
        let a = state.fs_overlay.clamp(0.0, 1.0);
        if a <= 0.01 {
            return cmds; // оверлей погашен — ничего не рисуем (и хит-тест в app заблокирован)
        }
        let ai = theme::ACTION_ICON_SIZE * scale;
        for (rect, glyph, region) in [
            (layout.btn_fs_play, GLYPH_PLAY, Region::SlideshowPlay),
            (layout.btn_fs_exit, GLYPH_FS_EXIT, Region::FullscreenExit),
        ] {
            // полупрозрачная подложка (ярче при hover), умноженная на прозрачность тулбара
            let mut bg = theme.overlay_bg;
            if state.hovered == region {
                bg[3] = (bg[3] + 0.25).min(1.0);
            }
            bg[3] *= a;
            let mut ic = theme.text_primary;
            ic[3] *= a;
            cmds.push(DrawCmd::Rect { rect, color: bg, radius: 6.0 * scale, layer: RectLayer::Bg });
            cmds.push(DrawCmd::Icon { rect, glyph, size: ai, color: ic, font: IconFont::Tabler });
        }
        return cmds;
    }

    // --- Titlebar (как в v0.3a) ---
    cmds.push(DrawCmd::Rect { rect: layout.titlebar, color: theme.bg_surface, radius: 0.0, layer: RectLayer::Bg });
    if state.hovered == Region::Minimize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_min, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
    }
    if state.hovered == Region::Maximize {
        cmds.push(DrawCmd::Rect { rect: layout.btn_max, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
    }
    if state.hovered == Region::Close {
        cmds.push(DrawCmd::Rect { rect: layout.btn_close, color: theme.button_close_hover, radius: 0.0, layer: RectLayer::Bg });
    }
    cmds.push(DrawCmd::Text {
        rect: layout.title,
        text: state.title.clone(),
        size: theme::TITLE_FONT_SIZE * scale,
        color: theme.text_primary,
        align: Align::Center,
        clip: None,
    });
    let icon = theme::ICON_FONT_SIZE * scale;
    cmds.push(DrawCmd::Icon { rect: layout.btn_min, glyph: GLYPH_MINIMIZE, size: icon, color: theme.text_primary, font: IconFont::WindowMdl2 });
    cmds.push(DrawCmd::Icon {
        rect: layout.btn_max,
        glyph: if state.maximized { GLYPH_RESTORE } else { GLYPH_MAXIMIZE },
        size: icon,
        color: theme.text_primary,
        font: IconFont::WindowMdl2,
    });
    cmds.push(DrawCmd::Icon { rect: layout.btn_close, glyph: GLYPH_CLOSE, size: icon, color: theme.text_primary, font: IconFont::WindowMdl2 });

    // --- Divider ---
    cmds.push(DrawCmd::Rect { rect: layout.divider, color: theme.bg_surface, radius: 0.0, layer: RectLayer::Bg });
    // грип по центру (маленький прямоугольник)
    let grip_w = 60.0 * scale;
    let grip_h = 3.0 * scale;
    let grip = Rect {
        x: layout.divider.x + (layout.divider.w - grip_w) * 0.5,
        y: layout.divider.y + (layout.divider.h - grip_h) * 0.5,
        w: grip_w,
        h: grip_h,
    };
    cmds.push(DrawCmd::Rect { rect: grip, color: theme.divider_grip, radius: grip_h * 0.5, layer: RectLayer::Bg });

    // --- Bottom bar (если хоть немного видим) ---
    if state.bottom_factor > 0.0 {
        cmds.push(DrawCmd::Rect { rect: layout.bottom_bar, color: theme.bg_surface, radius: 0.0, layer: RectLayer::Bg });

        // Мета-панель: формат / разрешение / размер — в столбик, без заголовков.
        if let Some(meta) = &state.meta {
            let line_h = theme::META_VALUE_SIZE * scale * 1.55;
            let mut y = layout.meta.y + 12.0 * scale;
            for value in meta_lines(meta) {
                cmds.push(DrawCmd::Text {
                    rect: Rect { x: layout.meta.x + 12.0 * scale, y, w: layout.meta.w - 14.0 * scale, h: line_h },
                    text: value,
                    size: theme::META_VALUE_SIZE * scale,
                    color: theme.text_primary,
                    align: Align::Left,
                    clip: None,
                });
                y += line_h;
            }
        }

        // Активная рамка (контур, не заливка — иначе перекрыла бы фото) и бейджи поверх миниатюр
        for (idx, r) in thumb_rects {
            if *idx == state.active_index {
                let t = theme::THUMB_BORDER * scale;
                let c = theme.active_border;
                // четыре тонких прямоугольника по краям миниатюры
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x, y: r.y, w: r.w, h: t }, color: c, radius: 0.0, layer: RectLayer::Overlay });
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x, y: r.y + r.h - t, w: r.w, h: t }, color: c, radius: 0.0, layer: RectLayer::Overlay });
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x, y: r.y, w: t, h: r.h }, color: c, radius: 0.0, layer: RectLayer::Overlay });
                cmds.push(DrawCmd::Rect { rect: Rect { x: r.x + r.w - t, y: r.y, w: t, h: r.h }, color: c, radius: 0.0, layer: RectLayer::Overlay });
            }
            if raw_flags.get(*idx).copied().unwrap_or(false) {
                // бейдж формата в правом нижнем углу — фон + текст
                let bw = 22.0 * scale;
                let bh = 11.0 * scale;
                let badge = Rect { x: r.x + r.w - bw - 3.0 * scale, y: r.y + r.h - bh - 3.0 * scale, w: bw, h: bh };
                cmds.push(DrawCmd::Rect { rect: badge, color: theme.badge_bg, radius: 2.0 * scale, layer: RectLayer::Overlay });
            }
        }
        // (текст бейджа — в Task 9, когда есть расширения файлов; здесь только фон-плашка)

        // Кнопки действий: [поворот] [fullscreen] [инфо].
        if state.hovered == Region::ActionRotate {
            cmds.push(DrawCmd::Rect { rect: layout.btn_rotate, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
        }
        if state.hovered == Region::ActionFullscreen {
            cmds.push(DrawCmd::Rect { rect: layout.btn_fullscreen, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
        }
        if state.hovered == Region::ActionExif {
            cmds.push(DrawCmd::Rect { rect: layout.btn_exif, color: theme.button_hover, radius: 0.0, layer: RectLayer::Bg });
        }
        let ai = theme::ACTION_ICON_SIZE * scale;
        // Все три кнопки активны (поворот — v0.4a, EXIF — v0.4b, fullscreen) → яркий цвет.
        cmds.push(DrawCmd::Icon { rect: layout.btn_rotate, glyph: GLYPH_ROTATE_CW, size: ai, color: theme.text_primary, font: IconFont::Tabler });
        cmds.push(DrawCmd::Icon { rect: layout.btn_fullscreen, glyph: GLYPH_FULLSCREEN, size: ai, color: theme.text_primary, font: IconFont::Tabler });
        cmds.push(DrawCmd::Icon { rect: layout.btn_exif, glyph: GLYPH_INFO, size: ai, color: theme.text_primary, font: IconFont::Tabler });
    }

    // --- Экранные стрелки навигации (поверх фото; проявляются на hover) ---
    let chev = theme::NAV_CHEVRON_SIZE * scale;
    for (i, (rect, can, glyph)) in [
        (layout.nav_prev, state.can_prev, GLYPH_CHEVRON_LEFT),
        (layout.nav_next, state.can_next, GLYPH_CHEVRON_RIGHT),
    ]
    .into_iter()
    .enumerate()
    {
        let a = state.nav_alpha[i].clamp(0.0, 1.0);
        if !can || a <= 0.01 {
            continue;
        }
        let mut bg = theme.overlay_bg;
        bg[3] *= a;
        let mut ic = theme.text_primary;
        ic[3] *= a;
        cmds.push(DrawCmd::Rect { rect, color: bg, radius: 8.0 * scale, layer: RectLayer::Bg });
        cmds.push(DrawCmd::Icon { rect, glyph, size: chev, color: ic, font: IconFont::Tabler });
    }

    cmds
}

use crate::ui::textedit::TextEdit;

/// Верхний зазор тела popup (физ. px): равен пустому полю под заголовком в его зоне,
/// чтобы зазор «поиск↔первая группа» совпадал с зазором «заголовок↔поиск».
fn popup_body_top_pad(scale: f32) -> f32 {
    (theme::POPUP_HEADER_H - theme::POPUP_TITLE_SIZE * 1.2) * 0.5 * scale
}

/// Пересечение двух прямоугольников (пустое → нулевые w/h).
fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.w).min(b.x + b.w);
    let bottom = (a.y + a.h).min(b.y + b.h);
    Rect { x, y, w: (right - x).max(0.0), h: (bottom - y).max(0.0) }
}

/// Подходит ли строка под фильтр (без регистра): по "group:tag" и значению.
fn row_matches(filter: &str, group: &str, tag: &str, value: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    format!("{group}:{tag}").to_lowercase().contains(&f) || value.to_lowercase().contains(&f)
}

use std::collections::BTreeMap;

/// Операция над тегом в буфере правок (зеркало app-стейта; (group,tag) → op).
#[derive(Clone, Debug, PartialEq)]
pub enum PendingOp {
    Set(String),
    Delete,
}

/// Какой бар подтверждения показан в футере (вместо булева флага части 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmKind {
    None,
    CloseWithPending,
    OverwriteSave,
    StripAll,
}

/// Состояние редактирования для отрисовки popup (часть 2).
pub struct PopupEditState<'a> {
    pub pending: &'a BTreeMap<(String, String), PendingOp>,
    pub delete_gps: bool,
    /// Активная инлайн-правка: (group, tag) + буфер редактора и его метрики.
    pub editing: Option<(&'a str, &'a str)>,
    pub editor: &'a TextEdit,
    pub editor_caret_px: f32,
    pub editor_sel_px: Option<(f32, f32)>,
    /// Индекс строки под курсором (из popup_rows) — для показа ✎/✕ и hover GPS.
    pub hovered_row: Option<usize>,
    /// Активный бар подтверждения (закрытие/перезапись/стирание).
    pub confirm: ConfirmKind,
    /// Необратимый режим (тоггл): показывать кнопку «Стереть всё», danger-стиль.
    pub overwrite_mode: bool,
    /// Есть ли несохранённые правки (для активности Save).
    pub has_pending: bool,
}

/// Вид строки списка тегов popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupRowKind {
    Group,
    Tag,
}

/// Геометрия одной видимой строки списка (виртуальная позиция со скроллом, физ. px).
#[derive(Clone, Debug)]
pub struct PopupRow {
    pub kind: PopupRowKind,
    pub group: String,
    pub tag: String,   // пусто для Group
    pub value: String, // пусто для Group
    pub editable: bool,
    pub y: f32,
    pub h: f32,
}

/// Все видимые строки (заголовки групп + теги) с учётом фильтра и скролла.
/// Порядок и фильтрация — те же, что в отрисовке. Используется scene/hit/app.
pub fn popup_rows(
    tags: &crate::exif::tags::ExifTags,
    filter: &str,
    scale: f32,
    scroll: f32,
    body: Rect,
) -> Vec<PopupRow> {
    use crate::exif::tags::is_editable;
    use crate::ui::layout::{popup_group_h, popup_row_h};
    let row_h = popup_row_h(scale);
    let grp_h = popup_group_h(scale);
    let mut out = Vec::new();
    let mut y = body.y - scroll + popup_body_top_pad(scale);
    for g in &tags.groups {
        let visible: Vec<&(String, String)> =
            g.tags.iter().filter(|(t, v)| row_matches(filter, &g.name, t, v)).collect();
        if visible.is_empty() {
            continue;
        }
        out.push(PopupRow {
            kind: PopupRowKind::Group,
            group: g.name.clone(),
            tag: String::new(),
            value: String::new(),
            editable: false,
            y,
            h: grp_h,
        });
        y += grp_h;
        let editable = is_editable(&g.name);
        for (tag, value) in visible {
            out.push(PopupRow {
                kind: PopupRowKind::Tag,
                group: g.name.clone(),
                tag: tag.clone(),
                value: value.clone(),
                editable,
                y,
                h: row_h,
            });
            y += row_h;
        }
    }
    out
}

/// Прямоугольники иконок-действий (edit, delete) у правого края строки тега.
pub fn popup_row_actions(row: &PopupRow, body: Rect, scale: f32) -> (Rect, Rect) {
    let pad = theme::POPUP_PAD * scale;
    let s = theme::POPUP_ACTION_ICON * scale + 8.0 * scale; // зона иконки
    let del = Rect { x: body.x + body.w - pad - s, y: row.y, w: s, h: row.h };
    let edit = Rect { x: del.x - s, y: row.y, w: s, h: row.h };
    (edit, del)
}

/// Построить DrawCmd'ы EXIF popup (рисуются поверх всего в конце кадра).
/// `tags=None` + `error=Some` → баннер ошибки; `tags=None` без ошибки → «загрузка…».
#[allow(clippy::too_many_arguments)]
pub fn build_popup(
    win: glam::Vec2,
    scale: f32,
    theme: &ThemePalette,
    filename: &str,
    tags: Option<&crate::exif::tags::ExifTags>,
    search: &TextEdit,
    scroll: f32,
    error: Option<&str>,
    // Смещения (физ. px от начала текста поиска), измеренные шрифтом текстового слоя:
    caret_px: f32,                // позиция каретки
    sel_px: Option<(f32, f32)>,   // границы выделения (если есть)
    caret_visible: bool,          // фаза мигания + фокус: рисовать ли каретку сейчас
    focused: bool,                // поле поиска в фокусе (рамка фокуса)
    edit: &PopupEditState,
) -> Vec<DrawCmd> {
    use crate::ui::layout::{popup_group_h, popup_layout, popup_row_h};
    let mut cmds = Vec::new();
    let p = popup_layout(win, scale);
    let pad = theme::POPUP_PAD * scale;

    // Затемнение всего окна + карточка.
    cmds.push(DrawCmd::Rect { rect: Rect { x: 0.0, y: 0.0, w: win.x, h: win.y }, color: theme::POPUP_DIM, radius: 0.0, layer: RectLayer::Bg });
    cmds.push(DrawCmd::Rect { rect: p.card, color: theme.bg_surface, radius: theme::POPUP_RADIUS * scale, layer: RectLayer::Bg });

    // Заголовок: "EXIF — имя" + [✕]
    cmds.push(DrawCmd::Text {
        rect: Rect { x: p.header.x + pad, y: p.header.y, w: p.header.w - pad * 2.0, h: p.header.h },
        text: format!("EXIF — {filename}"),
        size: theme::POPUP_TITLE_SIZE * scale,
        color: theme.text_primary,
        align: Align::Left,
        clip: None,
    });
    cmds.push(DrawCmd::Icon { rect: p.close, glyph: GLYPH_CLOSE, size: theme::ICON_FONT_SIZE * scale, color: theme.text_primary, font: IconFont::WindowMdl2 });

    // Поиск: (рамка фокуса) + подложка + текст/плейсхолдер.
    let field = Rect { x: p.search.x + pad, y: p.search.y + 4.0 * scale, w: p.search.w - pad * 2.0, h: p.search.h - 8.0 * scale };
    if focused {
        // акцентная рамка вокруг поля (1.5px), рисуется под подложкой
        let b = 1.5 * scale;
        cmds.push(DrawCmd::Rect {
            rect: Rect { x: field.x - b, y: field.y - b, w: field.w + 2.0 * b, h: field.h + 2.0 * b },
            color: theme.accent,
            radius: 6.0 * scale + b,
            layer: RectLayer::Bg,
        });
    }
    cmds.push(DrawCmd::Rect {
        rect: field,
        color: theme.popup_field_bg,
        radius: 6.0 * scale,
        layer: RectLayer::Bg,
    });
    let q = search.text();
    let text_x = p.search.x + pad * 2.0;
    let line_h = theme::POPUP_ROW_SIZE * 1.2 * scale;
    let sel_y = p.search.y + (p.search.h - line_h) * 0.5;
    let field_right = p.search.x + p.search.w - pad;
    // Подсветка выделения (под текстом).
    if let Some((a, b)) = sel_px {
        let x0 = (text_x + a.min(b)).min(field_right);
        let x1 = (text_x + a.max(b)).min(field_right);
        if x1 > x0 {
            cmds.push(DrawCmd::Rect {
                rect: Rect { x: x0, y: sel_y, w: x1 - x0, h: line_h },
                color: theme.selection_bg,
                radius: 2.0 * scale,
                layer: RectLayer::Bg,
            });
        }
    }
    let search_text = if q.is_empty() { "Поиск по тегу…".to_string() } else { q.clone() };
    let search_color = if q.is_empty() { theme.text_secondary } else { theme.text_primary };
    cmds.push(DrawCmd::Text {
        rect: Rect { x: text_x, y: p.search.y, w: p.search.w - pad * 4.0, h: p.search.h },
        text: search_text,
        size: theme::POPUP_ROW_SIZE * scale,
        color: search_color,
        align: Align::Left,
        clip: None,
    });
    // Каретка (поле активно, пока popup открыт) — мигает по фазе caret_visible.
    if caret_visible {
        let caret_x = (text_x + caret_px).min(field_right);
        cmds.push(DrawCmd::Rect {
            rect: Rect { x: caret_x, y: sel_y, w: theme::POPUP_CARET_W * scale, h: line_h },
            color: theme.text_primary,
            radius: 0.0,
            layer: RectLayer::Bg,
        });
    }

    // Ошибка/загрузка вместо тела.
    let Some(tags) = tags else {
        let msg = error.unwrap_or("Загрузка EXIF…");
        cmds.push(DrawCmd::Text {
            rect: Rect { x: p.body.x + pad, y: p.body.y + pad, w: p.body.w - pad * 2.0, h: popup_row_h(scale) },
            text: msg.to_string(),
            size: theme::POPUP_ROW_SIZE * scale,
            color: theme.text_secondary,
            align: Align::Left,
            clip: None,
        });
        return cmds;
    };

    // Строки: единый итератор popup_rows (та же геометрия, что в hit/app).
    let body = p.body;
    let rows = popup_rows(tags, &search.text(), scale, scroll, body);
    let grp_h = popup_group_h(scale);
    let body_top = body.y;
    let body_bot = body.y + body.h;
    for (i, r) in rows.iter().enumerate() {
        if r.y + r.h <= body_top || r.y >= body_bot {
            continue; // вне тела
        }
        match r.kind {
            PopupRowKind::Group => {
                let band = intersect(Rect { x: body.x, y: r.y, w: body.w, h: grp_h }, body);
                if band.h > 0.0 {
                    cmds.push(DrawCmd::Rect { rect: band, color: theme.popup_group_bg, radius: 0.0, layer: RectLayer::Bg });
                }
                let gr = Rect { x: body.x + pad, y: r.y, w: body.w - pad * 2.0, h: grp_h };
                cmds.push(DrawCmd::Text { rect: gr, text: r.group.clone(), size: theme::POPUP_GROUP_SIZE * scale, color: theme.text_primary, align: Align::Left, clip: Some(intersect(gr, body)) });
                // GPS: действие «удалить всё» по hover группы (или если уже взведено)
                if r.group == "GPS" && (edit.hovered_row == Some(i) || edit.delete_gps) {
                    let (_e, del) = popup_row_actions(r, body, scale);
                    let col = if edit.delete_gps { theme.danger } else { theme.text_secondary };
                    cmds.push(DrawCmd::Icon { rect: del, glyph: GLYPH_DELETE, size: theme::POPUP_ACTION_ICON * scale, color: col, font: IconFont::WindowMdl2 });
                }
            }
            PopupRowKind::Tag => {
                let key = (r.group.clone(), r.tag.clone());
                let op = edit.pending.get(&key);
                let gps_deleted = edit.delete_gps && r.group == "GPS";
                let deleted = matches!(op, Some(PendingOp::Delete)) || gps_deleted;
                let editing_this = edit.editing == Some((r.group.as_str(), r.tag.as_str()));
                // ключ слева
                let kr = Rect { x: body.x + pad, y: r.y, w: (body.w - pad * 2.0) * 0.45, h: r.h };
                cmds.push(DrawCmd::Text { rect: kr, text: r.tag.clone(), size: theme::POPUP_ROW_SIZE * scale, color: theme.text_secondary, align: Align::Left, clip: Some(intersect(kr, body)) });
                let vx = body.x + (body.w - pad * 2.0) * 0.45;
                let vw = (body.w - pad * 2.0) * 0.55;
                let vr = Rect { x: vx, y: r.y, w: vw, h: r.h };
                if editing_this {
                    // инлайн-поле редактора: подложка + текст редактора + каретка/выделение
                    let field = Rect { x: vx, y: r.y + 3.0 * scale, w: vw - pad, h: r.h - 6.0 * scale };
                    cmds.push(DrawCmd::Rect { rect: field, color: theme.popup_field_bg, radius: 4.0 * scale, layer: RectLayer::Bg });
                    let tx = field.x + 6.0 * scale;
                    let line_h = theme::POPUP_ROW_SIZE * 1.2 * scale;
                    let ty = r.y + (r.h - line_h) * 0.5;
                    if let Some((a, b)) = edit.editor_sel_px {
                        let x0 = tx + a.min(b);
                        let x1 = tx + a.max(b);
                        if x1 > x0 {
                            cmds.push(DrawCmd::Rect { rect: Rect { x: x0, y: ty, w: x1 - x0, h: line_h }, color: theme.selection_bg, radius: 2.0 * scale, layer: RectLayer::Bg });
                        }
                    }
                    let etext = edit.editor.text();
                    cmds.push(DrawCmd::Text { rect: Rect { x: tx, y: r.y, w: field.w - 12.0 * scale, h: r.h }, text: etext, size: theme::POPUP_ROW_SIZE * scale, color: theme.text_primary, align: Align::Left, clip: Some(intersect(field, body)) });
                    if caret_visible {
                        let cx = (tx + edit.editor_caret_px).min(field.x + field.w - 2.0 * scale);
                        cmds.push(DrawCmd::Rect { rect: Rect { x: cx, y: ty, w: theme::POPUP_CARET_W * scale, h: line_h }, color: theme.text_primary, radius: 0.0, layer: RectLayer::Bg });
                    }
                } else {
                    // значение: pending Set → новое значение акцентом; Delete/gps → зачёркнуто/приглушено
                    let shown = match op {
                        Some(PendingOp::Set(v)) => v.clone(),
                        _ => r.value.clone(),
                    };
                    let col = match op {
                        Some(PendingOp::Set(_)) => theme.pending_mark,
                        _ if deleted => theme.text_secondary,
                        _ => theme.text_primary,
                    };
                    cmds.push(DrawCmd::Text { rect: vr, text: shown, size: theme::POPUP_ROW_SIZE * scale, color: col, align: Align::Left, clip: Some(intersect(vr, body)) });
                    if deleted {
                        // линия зачёркивания
                        let ly = r.y + r.h * 0.5;
                        cmds.push(DrawCmd::Rect { rect: Rect { x: vx, y: ly, w: vw * 0.9, h: 1.0 * scale }, color: theme.text_secondary, radius: 0.0, layer: RectLayer::Bg });
                    }
                    // ✎/✕ по hover редактируемой строки
                    if r.editable && edit.hovered_row == Some(i) && !deleted {
                        let (er, dr) = popup_row_actions(r, body, scale);
                        cmds.push(DrawCmd::Icon { rect: er, glyph: GLYPH_EDIT, size: theme::POPUP_ACTION_ICON * scale, color: theme.text_secondary, font: IconFont::WindowMdl2 });
                        cmds.push(DrawCmd::Icon { rect: dr, glyph: GLYPH_DELETE, size: theme::POPUP_ACTION_ICON * scale, color: theme.danger, font: IconFont::WindowMdl2 });
                    }
                }
            }
        }
    }

    // Футер: при confirm != None — бар подтверждения; иначе тоггл + действия.
    let (save, cancel) = crate::ui::layout::popup_footer_buttons(&p, scale);
    cmds.push(DrawCmd::Rect { rect: p.footer, color: theme.popup_group_bg, radius: 0.0, layer: RectLayer::Bg });
    let fpad = theme::POPUP_PAD * scale;
    let btn_text = |cmds: &mut Vec<DrawCmd>, rect: Rect, text: &str, bg: [f32; 4], fg: [f32; 4]| {
        cmds.push(DrawCmd::Rect { rect, color: bg, radius: 6.0 * scale, layer: RectLayer::Bg });
        cmds.push(DrawCmd::Text { rect, text: text.to_string(), size: theme::POPUP_BTN_SIZE * scale, color: fg, align: Align::Center, clip: Some(rect) });
    };
    let label_rect = Rect { x: p.footer.x + fpad, y: p.footer.y, w: cancel.x - p.footer.x - fpad, h: p.footer.h };
    match edit.confirm {
        ConfirmKind::CloseWithPending => {
            cmds.push(DrawCmd::Text { rect: label_rect, text: "Несохранённые изменения".to_string(), size: theme::POPUP_BTN_SIZE * scale, color: theme.text_primary, align: Align::Left, clip: Some(label_rect) });
            let keep = Rect { x: cancel.x - (save.x - cancel.x), y: cancel.y, w: cancel.w, h: cancel.h };
            btn_text(&mut cmds, keep, "Продолжить", theme.button_hover, theme.text_primary);
            btn_text(&mut cmds, cancel, "Отменить", theme.button_hover, theme.text_primary);
            btn_text(&mut cmds, save, "Сохранить", theme.save_bg, theme.text_primary);
        }
        ConfirmKind::OverwriteSave => {
            cmds.push(DrawCmd::Text { rect: label_rect, text: "Необратимо, бэкапа не будет".to_string(), size: theme::POPUP_BTN_SIZE * scale, color: theme.danger, align: Align::Left, clip: Some(label_rect) });
            btn_text(&mut cmds, cancel, "Отмена", theme.button_hover, theme.text_primary);
            btn_text(&mut cmds, save, "Перезаписать", theme.danger, theme.text_primary);
        }
        ConfirmKind::StripAll => {
            cmds.push(DrawCmd::Text { rect: label_rect, text: "Удалить ВСЕ метаданные, необратимо".to_string(), size: theme::POPUP_BTN_SIZE * scale, color: theme.danger, align: Align::Left, clip: Some(label_rect) });
            btn_text(&mut cmds, cancel, "Отмена", theme.button_hover, theme.text_primary);
            btn_text(&mut cmds, save, "Стереть", theme.danger, theme.text_primary);
        }
        ConfirmKind::None => {
            // тоггл «Необратимо» (слева): бокс + подпись
            let tog = crate::ui::layout::popup_footer_toggle(&p, scale);
            let box_sz = 14.0 * scale;
            let box_r = Rect { x: tog.x, y: tog.y + (tog.h - box_sz) * 0.5, w: box_sz, h: box_sz };
            let box_bg = if edit.overwrite_mode { theme.danger } else { theme.popup_field_bg };
            cmds.push(DrawCmd::Rect { rect: box_r, color: box_bg, radius: 3.0 * scale, layer: RectLayer::Bg });
            let lbl = Rect { x: box_r.x + box_sz + 6.0 * scale, y: tog.y, w: tog.w - box_sz - 6.0 * scale, h: tog.h };
            let lbl_col = if edit.overwrite_mode { theme.danger } else { theme.text_secondary };
            cmds.push(DrawCmd::Text { rect: lbl, text: "Необратимо".to_string(), size: theme::POPUP_BTN_SIZE * scale, color: lbl_col, align: Align::Left, clip: Some(lbl) });
            // «Стереть всё» — только в необратимом режиме
            if edit.overwrite_mode {
                let strip = crate::ui::layout::popup_footer_strip(&p, scale);
                btn_text(&mut cmds, strip, "Стереть всё", theme.danger_bg, theme.danger);
            }
            btn_text(&mut cmds, cancel, "Отменить всё", theme.button_hover, theme.text_primary);
            let (sbg, sfg) = if edit.has_pending { (theme.save_bg, theme.text_primary) } else { (theme.button_hover, theme.text_secondary) };
            btn_text(&mut cmds, save, "Сохранить", sbg, sfg);
        }
    }
    cmds
}

/// Полная высота содержимого popup (для клампа скролла), физ. px.
pub fn popup_content_height(tags: &crate::exif::tags::ExifTags, search: &TextEdit, scale: f32) -> f32 {
    use crate::ui::layout::{popup_group_h, popup_row_h};
    let filter = search.text();
    let mut h = popup_body_top_pad(scale); // верхний зазор тела входит в прокручиваемую высоту
    for g in &tags.groups {
        let n = g.tags.iter().filter(|(t, v)| row_matches(&filter, &g.name, t, v)).count();
        if n > 0 {
            h += popup_group_h(scale) + n as f32 * popup_row_h(scale);
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::compute;
    use glam::Vec2;

    fn fixture(f: impl FnOnce(&mut UiState)) -> Vec<DrawCmd> {
        let layout = compute(Vec2::new(1280.0, 800.0), 1.0, 1.0, false);
        let theme = ThemePalette::dark();
        let mut state = UiState::new();
        state.title = "a.jpg · Lumina".into();
        f(&mut state);
        let aspects = vec![1.5_f32; state.thumb_count];
        let thumbs = crate::ui::layout::carousel_thumb_rects(layout.carousel, &aspects, state.scroll, 1.0);
        let raw: Vec<bool> = (0..state.thumb_count).map(|i| i % 2 == 0).collect();
        build(&state, &layout, &theme, 1.0, &thumbs, &raw)
    }

    #[test]
    fn nav_arrow_shown_on_hover_when_can_navigate() {
        let cmds = fixture(|s| { s.can_next = true; s.nav_alpha = [0.0, 1.0]; });
        let has_right = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_CHEVRON_RIGHT));
        assert!(has_right);
    }

    #[test]
    fn nav_arrow_hidden_when_cannot_navigate() {
        let cmds = fixture(|s| { s.can_next = false; s.nav_alpha = [0.0, 1.0]; });
        let has_right = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_CHEVRON_RIGHT));
        assert!(!has_right);
    }

    #[test]
    fn nav_arrow_hidden_when_alpha_zero() {
        let cmds = fixture(|s| { s.can_prev = true; s.nav_alpha = [0.0, 0.0]; });
        let has_left = cmds.iter().any(|c| matches!(c, DrawCmd::Icon { glyph, .. } if *glyph == GLYPH_CHEVRON_LEFT));
        assert!(!has_left);
    }

    #[test]
    fn fullscreen_emits_only_overlay() {
        let cmds = fixture(|s| { s.fullscreen = true; s.fs_overlay = 1.0; });
        // нет хрома (titlebar bg высотой 32 отсутствует)
        let has_titlebar = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { rect, .. } if rect.h == 32.0));
        assert!(!has_titlebar);
        // ровно 2 оверлейных глифа: play + выход
        let icons: Vec<char> = cmds.iter().filter_map(|c| match c {
            DrawCmd::Icon { glyph, font: IconFont::Tabler, .. } => Some(*glyph),
            _ => None,
        }).collect();
        assert_eq!(icons.len(), 2);
        assert!(icons.contains(&GLYPH_PLAY));
        assert!(icons.contains(&GLYPH_FS_EXIT));
    }

    #[test]
    fn titlebar_present_with_three_window_icons() {
        let cmds = fixture(|_| {});
        let win_icons = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { font: IconFont::WindowMdl2, .. })).count();
        assert_eq!(win_icons, 3);
    }

    #[test]
    fn three_action_icons_tabler() {
        let cmds = fixture(|_| {});
        let tab = cmds.iter().filter(|c| matches!(c, DrawCmd::Icon { font: IconFont::Tabler, .. })).count();
        assert_eq!(tab, 3); // поворот + fullscreen + инфо
    }

    #[test]
    fn action_icons_share_active_color() {
        let cmds = fixture(|_| {});
        let theme = ThemePalette::dark();
        let mut colors = std::collections::HashMap::new();
        for c in &cmds {
            if let DrawCmd::Icon { glyph, color, font: IconFont::Tabler, .. } = c {
                colors.insert(*glyph, *color);
            }
        }
        // Поворот/fullscreen/EXIF — все активны → одинаковый яркий цвет (text_primary).
        assert_eq!(colors[&GLYPH_ROTATE_CW], theme.text_primary);
        assert_eq!(colors[&GLYPH_FULLSCREEN], theme.text_primary);
        assert_eq!(colors[&GLYPH_INFO], theme.text_primary);
    }

    #[test]
    fn hidden_bottom_keeps_divider_no_bar_bg() {
        let cmds = fixture(|s| { s.bottom_factor = 0.0; s.bottom_visible = false; });
        // bottom bar фон (высота 84) отсутствует
        let bar_bg = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { rect, .. } if rect.h == 84.0));
        assert!(!bar_bg);
        // divider всегда виден — присутствует грип (его цвет)
        let theme = ThemePalette::dark();
        let grip = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { color, .. } if *color == theme.divider_grip));
        assert!(grip);
    }

    #[test]
    fn active_thumb_border_present_when_visible() {
        let cmds = fixture(|s| { s.thumb_count = 10; s.active_index = 0; });
        let theme = ThemePalette::dark();
        let border = cmds.iter().any(|c| matches!(c, DrawCmd::Rect { color, .. } if *color == theme.active_border));
        assert!(border);
    }

    #[test]
    fn meta_lines_format() {
        let m = FileMeta { format_label: "RAF · RAW".into(), megapixels: 40.23, width: 7728, height: 5200, bytes: 40_265_318 };
        let lines = meta_lines(&m);
        assert_eq!(lines[0], "RAF · RAW");
        assert_eq!(lines[1], "7728×5200px");
        assert!(lines[2].contains("MB"));
    }

    #[test]
    fn humanize_bytes_units() {
        assert_eq!(humanize_bytes(512), "512B");
        assert!(humanize_bytes(2048).contains("KB"));
        assert!(humanize_bytes(5 * 1024 * 1024).contains("MB"));
    }

    fn sample_tags() -> crate::exif::tags::ExifTags {
        crate::exif::tags::parse(
            r#"[{"SourceFile":"a","EXIF:Make":"Fujifilm","EXIF:Model":"X-T5","GPS:GPSLatitude":"41 N"}]"#,
        )
    }

    fn empty_edit<'a>(editor: &'a crate::ui::textedit::TextEdit) -> PopupEditState<'a> {
        use std::collections::BTreeMap;
        // утечка пустой map ради 'a в тесте — допустимо (тестовый процесс короткоживущий)
        let pending: &'static BTreeMap<(String, String), PendingOp> = Box::leak(Box::new(BTreeMap::new()));
        PopupEditState {
            pending,
            delete_gps: false,
            editing: None,
            editor,
            editor_caret_px: 0.0,
            editor_sel_px: None,
            hovered_row: None,
            confirm: ConfirmKind::None,
            overwrite_mode: false,
            has_pending: false,
        }
    }

    #[test]
    fn popup_footer_buttons_emitted() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let edit = empty_edit(&editor);
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &edit);
        let has_save = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Сохранить"));
        let has_cancel = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Отменить всё"));
        assert!(has_save && has_cancel);
    }

    #[test]
    fn popup_pending_set_marks_new_value() {
        use std::collections::BTreeMap;
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let mut map: BTreeMap<(String, String), PendingOp> = BTreeMap::new();
        map.insert(("EXIF".into(), "Make".into()), PendingOp::Set("NEWVAL".into()));
        let edit = PopupEditState { pending: &map, delete_gps: false, editing: None, editor: &editor, editor_caret_px: 0.0, editor_sel_px: None, hovered_row: None, confirm: ConfirmKind::None, overwrite_mode: false, has_pending: true };
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &edit);
        // новое значение отрисовано вместо старого
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "NEWVAL")));
    }

    #[test]
    fn popup_confirm_close_bar() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let mut edit = empty_edit(&editor);
        edit.confirm = ConfirmKind::CloseWithPending;
        edit.has_pending = true;
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &edit);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text.contains("есохранён"))));
    }

    #[test]
    fn popup_rows_groups_then_tags_editable_flag() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let tags = sample_tags(); // EXIF:Make, EXIF:Model, GPS:GPSLatitude
        let search = crate::ui::textedit::TextEdit::new();
        let p = crate::ui::layout::popup_layout(win, 1.0);
        let rows = popup_rows(&tags, &search.text(), 1.0, 0.0, p.body);
        // первая строка — заголовок группы EXIF
        assert!(matches!(rows[0].kind, PopupRowKind::Group));
        assert_eq!(rows[0].group, "EXIF");
        // теговые строки EXIF editable, их значение присутствует
        let make = rows.iter().find(|r| r.tag == "Make").unwrap();
        assert!(matches!(make.kind, PopupRowKind::Tag));
        assert!(make.editable); // EXIF — записываемая группа
        // координаты возрастают сверху вниз
        assert!(rows[1].y >= rows[0].y);
    }

    #[test]
    fn popup_emits_card_and_rows() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &empty_edit(&editor));
        // есть текст значения Model
        let has_model = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "X-T5"));
        assert!(has_model);
        // есть заголовок группы EXIF
        let has_group = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "EXIF"));
        assert!(has_group);
    }

    #[test]
    fn popup_filter_limits_rows() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let mut search = crate::ui::textedit::TextEdit::new();
        search.insert_str("model"); // фильтр (без регистра) — только строка Model
        let editor = crate::ui::textedit::TextEdit::new();
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &empty_edit(&editor));
        let has_model = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "X-T5"));
        let has_make = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Fujifilm"));
        assert!(has_model);
        assert!(!has_make);
    }

    #[test]
    fn popup_error_banner_shown() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", None, &search, 0.0, Some("exiftool недоступен"), 0.0, None, true, true, &empty_edit(&editor));
        let has_err = cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text.contains("недоступен")));
        assert!(has_err);
    }

    #[test]
    fn popup_body_rows_clipped_within_body() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let p = crate::ui::layout::popup_layout(win, 1.0);
        // прокрутка на половину строки — первая строка частично уезжает за верх тела
        let editor = crate::ui::textedit::TextEdit::new();
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 13.0, None, 0.0, None, true, true, &empty_edit(&editor));
        // строки тела несут clip и не выходят за пределы body (не лезут на поиск/под карточку)
        let clipped = cmds.iter().filter(|c| matches!(c, DrawCmd::Text { clip: Some(_), .. })).count();
        assert!(clipped > 0, "у строк тела должен быть clip");
        for c in &cmds {
            if let DrawCmd::Text { clip: Some(cl), .. } = c {
                // футерные кнопки клипуются к себе (в футере, ниже тела) — проверяем только клипы тела
                if cl.y >= p.body.y + p.body.h {
                    continue;
                }
                assert!(cl.y >= p.body.y - 0.01, "clip не должен заходить выше body_top");
                assert!(cl.y + cl.h <= p.body.y + p.body.h + 0.01, "и ниже body_bot");
            }
        }
    }

    #[test]
    fn popup_overwrite_toggle_shows_strip() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let mut edit = empty_edit(&editor);
        edit.overwrite_mode = true;
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &edit);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Стереть всё")));
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Необратимо")));
    }

    #[test]
    fn popup_overwrite_save_confirm_bar() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let mut edit = empty_edit(&editor);
        edit.confirm = ConfirmKind::OverwriteSave;
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &edit);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Перезаписать")));
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text.contains("бэкапа"))));
    }

    #[test]
    fn popup_strip_confirm_bar() {
        let win = glam::Vec2::new(1280.0, 800.0);
        let theme = ThemePalette::dark();
        let tags = sample_tags();
        let search = crate::ui::textedit::TextEdit::new();
        let editor = crate::ui::textedit::TextEdit::new();
        let mut edit = empty_edit(&editor);
        edit.confirm = ConfirmKind::StripAll;
        let cmds = build_popup(win, 1.0, &theme, "a.jpg", Some(&tags), &search, 0.0, None, 0.0, None, true, true, &edit);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text == "Стереть")));
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::Text { text, .. } if text.contains("ВСЕ метаданные"))));
    }
}
