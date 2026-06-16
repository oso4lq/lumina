//! Тонкая обёртка над glyphon для рендера текстовых ранов и глифов-иконок.

use crate::ui::scene::{Align, DrawCmd, IconFont};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

pub struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffers: Vec<(Buffer, f32, f32, [f32; 4], TextBounds)>, // буфер, left, top (физ. px), цвет, клип
}

/// Клип-границы текстового рана = его `rect`, пересечённый с экраном.
/// Так длинные неразрывные значения (хеши/пути) не вылезают за свой регион (напр. карточку popup).
fn rect_bounds(rect: &crate::ui::layout::Rect, screen: (u32, u32)) -> TextBounds {
    let sw = screen.0 as f32;
    let sh = screen.1 as f32;
    let left = rect.x.clamp(0.0, sw);
    let top = rect.y.clamp(0.0, sh);
    let right = (rect.x + rect.w).clamp(left, sw);
    let bottom = (rect.y + rect.h).clamp(top, sh);
    TextBounds {
        left: left.floor() as i32,
        top: top.floor() as i32,
        right: right.ceil() as i32,
        bottom: bottom.ceil() as i32,
    }
}

/// Линейный RGBA (0..1) → glyphon::Color (sRGB 0..255).
/// glyphon кладёт цвет как есть в sRGB-атлас; конвертируем линейный обратно в sRGB-байты.
fn to_glyphon_color(c: [f32; 4]) -> Color {
    let enc = |v: f32| -> u8 {
        let s = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        (s.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color::rgba(enc(c[0]), enc(c[1]), enc(c[2]), (c[3].clamp(0.0, 1.0) * 255.0) as u8)
}

impl TextLayer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let mut font_system = FontSystem::new();
        // Встроенный шрифт иконок действий (Tabler Icons, MIT).
        font_system
            .db_mut()
            .load_font_data(include_bytes!("../../assets/fonts/tabler-icons.ttf").to_vec());
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Self { font_system, swash_cache, viewport, atlas, renderer, buffers: Vec::new() }
    }

    /// Построить буферы из текстовых/иконочных команд и подготовить к отрисовке.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: (u32, u32),
        cmds: &[DrawCmd],
    ) -> Result<(), String> {
        self.viewport.update(queue, Resolution { width: screen.0, height: screen.1 });
        self.buffers.clear();

        for cmd in cmds {
            match cmd {
                DrawCmd::Text { rect, text, size, color, align, clip } => {
                    let mut buf = Buffer::new(&mut self.font_system, Metrics::new(*size, *size * 1.2));
                    buf.set_size(&mut self.font_system, Some(rect.w), Some(rect.h));
                    buf.set_text(
                        &mut self.font_system,
                        text,
                        Attrs::new().family(Family::SansSerif),
                        Shaping::Advanced,
                    );
                    buf.shape_until_scroll(&mut self.font_system, false);
                    // Горизонтальное выравнивание: измеряем ширину строки.
                    let line_w = buf
                        .layout_runs()
                        .next()
                        .map(|r| r.line_w)
                        .unwrap_or(0.0);
                    let left = match align {
                        Align::Left => rect.x,
                        Align::Center => rect.x + (rect.w - line_w) * 0.5,
                    };
                    let top = rect.y + (rect.h - *size * 1.2) * 0.5;
                    // Клип: по явной области `clip` (popup-тело) либо по собственному `rect`.
                    let bounds = rect_bounds(clip.as_ref().unwrap_or(rect), screen);
                    self.buffers.push((buf, left, top, *color, bounds));
                }
                DrawCmd::Icon { rect, glyph, size, color, font } => {
                    let mut buf = Buffer::new(&mut self.font_system, Metrics::new(*size, *size * 1.2));
                    buf.set_size(&mut self.font_system, Some(rect.w), Some(rect.h));
                    let s = glyph.to_string();
                    let family = match font {
                        IconFont::WindowMdl2 => crate::ui::scene::ICON_FONT_FAMILY,
                        IconFont::Tabler => crate::ui::scene::TABLER_FONT_FAMILY,
                    };
                    buf.set_text(
                        &mut self.font_system,
                        &s,
                        Attrs::new().family(Family::Name(family)),
                        Shaping::Advanced,
                    );
                    buf.shape_until_scroll(&mut self.font_system, false);
                    let line_w =
                        buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
                    let left = rect.x + (rect.w - line_w) * 0.5;
                    let top = rect.y + (rect.h - *size * 1.2) * 0.5;
                    self.buffers.push((buf, left, top, *color, rect_bounds(rect, screen)));
                }
                DrawCmd::Rect { .. } => {}
            }
        }

        let areas: Vec<TextArea> = self
            .buffers
            .iter()
            .map(|(buf, left, top, color, bounds)| TextArea {
                buffer: buf,
                left: *left,
                top: *top,
                scale: 1.0,
                bounds: *bounds,
                default_color: to_glyphon_color(*color),
                custom_glyphs: &[],
            })
            .collect();

        self.renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash_cache,
            )
            .map_err(|e| format!("glyphon prepare: {e:?}"))
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) -> Result<(), String> {
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .map_err(|e| format!("glyphon render: {e:?}"))
    }

    /// Освободить место в атласе между кадрами.
    pub fn trim(&mut self) {
        self.atlas.trim();
    }

    /// Ширина строки (физ. px) при том же шрифте/кегле, что и текстовые раны.
    /// Нужна для позиционирования каретки/выделения в поле поиска (метрики — только здесь).
    pub fn measure_width(&mut self, text: &str, size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let mut buf = Buffer::new(&mut self.font_system, Metrics::new(size, size * 1.2));
        buf.set_size(&mut self.font_system, None, None);
        buf.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        buf.shape_until_scroll(&mut self.font_system, false);
        buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0)
    }
}
