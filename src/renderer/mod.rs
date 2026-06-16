mod context;
mod pipeline;
mod text;
mod thumbnail;
mod ui_pipeline;

use crate::error::{LuminaError, Result};
use crate::ui::layout::Rect;
use crate::ui::scene::DrawCmd;
use crate::view::ViewTransform;
use context::GpuContext;
use glam::Vec2;
use pipeline::BlitPipeline;
use std::sync::Arc;
use text::TextLayer;
use thumbnail::ThumbnailLayer;
use ui_pipeline::UiPipeline;
use winit::window::Window;

pub struct Renderer {
    ctx: GpuContext,
    blit: BlitPipeline,
    ui: UiPipeline,         // фон-подложки (рисуются ДО миниатюр)
    ui_overlay: UiPipeline, // рамка/бейджи (рисуются ПОСЛЕ миниатюр)
    text: TextLayer,
    thumbs: ThumbnailLayer,
    image_size: Option<Vec2>,
    /// Высота titlebar (физ. px); 0 в fullscreen.
    titlebar_h: f32,
    /// Высота нижнего хрома divider+bottom_bar (физ. px); 0 в fullscreen.
    bottom_chrome_h: f32,
    /// Зона клипа миниатюр (карусель), физ. px.
    thumb_clip: Rect,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let ctx = GpuContext::new(window)?;
        let blit = BlitPipeline::new(&ctx.device, ctx.config.format);
        let ui = UiPipeline::new(&ctx.device, ctx.config.format);
        let ui_overlay = UiPipeline::new(&ctx.device, ctx.config.format);
        let text = TextLayer::new(&ctx.device, &ctx.queue, ctx.config.format);
        let thumbs = ThumbnailLayer::new(&ctx.device, ctx.config.format);
        Ok(Self {
            ctx, blit, ui, ui_overlay, text, thumbs,
            image_size: None,
            titlebar_h: 0.0,
            bottom_chrome_h: 0.0,
            thumb_clip: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
    }

    pub fn surface_size(&self) -> Vec2 {
        Vec2::new(self.ctx.config.width as f32, self.ctx.config.height as f32)
    }

    /// Размер viewer-региона (между titlebar и нижним хромом) — для математики вида.
    pub fn viewer_size(&self) -> Vec2 {
        let s = self.surface_size();
        Vec2::new(s.x, (s.y - self.titlebar_h - self.bottom_chrome_h).max(1.0))
    }

    pub fn set_titlebar_height(&mut self, h: f32) {
        self.titlebar_h = h;
    }

    pub fn set_bottom_chrome_height(&mut self, h: f32) {
        self.bottom_chrome_h = h;
    }

    /// Зона клипа миниатюр (карусель). Вне её миниатюры не рисуются.
    pub fn set_thumb_clip(&mut self, clip: Rect) {
        self.thumb_clip = clip;
    }

    pub fn image_size(&self) -> Option<Vec2> {
        self.image_size
    }

    pub fn upload_texture(&mut self, rgba: &[u8], width: u32, height: u32) {
        self.blit.upload(&self.ctx.device, &self.ctx.queue, rgba, width, height);
        self.image_size = Some(Vec2::new(width as f32, height as f32));
    }

    /// Загрузить текстуру миниатюры.
    pub fn set_thumbnail(&mut self, index: usize, rgba: &[u8], w: u32, h: u32) {
        self.thumbs.set(&self.ctx.device, &self.ctx.queue, index, rgba, w, h);
    }

    /// Освободить текстуру миниатюры (LRU-эвикция).
    pub fn drop_thumbnail(&mut self, index: usize) {
        self.thumbs.remove(index);
    }

    pub fn has_thumbnail(&self, index: usize) -> bool {
        self.thumbs.has(index)
    }

    /// Один кадр: фото в viewer-viewport, затем миниатюры, UI-прямоугольники, текст.
    pub fn render(&mut self, view: &ViewTransform, cmds: &[DrawCmd], thumb_rects: &[(usize, Rect)]) -> Result<()> {
        let frame = self
            .ctx
            .surface
            .get_current_texture()
            .map_err(|e| LuminaError::Gpu(format!("get_current_texture: {e}")))?;
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let screen = [self.ctx.config.width as f32, self.ctx.config.height as f32];

        // Прямоугольники из draw-команд, разделённые по слою:
        // Bg — подложки (до миниатюр), Overlay — рамка/бейджи (после миниатюр).
        use crate::ui::scene::RectLayer;
        let bg_rects: Vec<(Rect, [f32; 4], f32)> = cmds
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Rect { rect, color, radius, layer: RectLayer::Bg } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        let overlay_rects: Vec<(Rect, [f32; 4], f32)> = cmds
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Rect { rect, color, radius, layer: RectLayer::Overlay } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        self.ui.prepare(&self.ctx.device, &self.ctx.queue, screen, &bg_rects);
        self.ui_overlay.prepare(&self.ctx.device, &self.ctx.queue, screen, &overlay_rects);

        // Текст и глифы
        self.text
            .prepare(&self.ctx.device, &self.ctx.queue, (self.ctx.config.width, self.ctx.config.height), cmds)
            .map_err(LuminaError::Gpu)?;

        // Миниатюры — экранные размеры в uniform
        self.thumbs.set_screen(&self.ctx.queue, screen);
        let thumb_scale = self.scale_for_thumbs();

        // viewer-регион под titlebar и над нижним хромом
        let bar_h = self.titlebar_h.min(screen[1]);
        let viewer_h = (screen[1] - bar_h - self.bottom_chrome_h).max(0.0);
        let viewer = (0.0, bar_h, screen[0], viewer_h);
        let viewer_ok = viewer_h >= 1.0 && screen[0] >= 1.0;

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.blit.bg_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // 1) фото в viewer-регионе
            if viewer_ok {
                self.blit.draw(&self.ctx.queue, &mut pass, view, self.image_size, viewer);
                pass.set_viewport(0.0, 0.0, screen[0], screen[1], 0.0, 1.0);
            }
            // 2) фон-подложки (titlebar/divider/bottom bar/hover) — ДО миниатюр
            self.ui.draw(&mut pass);
            // 3) миниатюры — клипуются зоной карусели (scissor), иначе при сворачивании
            //    bottom bar они вылезали бы в viewer/углы.
            let cx = self.thumb_clip.x.max(0.0).min(screen[0]);
            let cy = self.thumb_clip.y.max(0.0).min(screen[1]);
            let cw = (self.thumb_clip.x + self.thumb_clip.w).min(screen[0]) - cx;
            let ch = (self.thumb_clip.y + self.thumb_clip.h).min(screen[1]) - cy;
            if cw >= 1.0 && ch >= 1.0 {
                pass.set_scissor_rect(cx as u32, cy as u32, cw as u32, ch as u32);
                self.thumbs.draw(&self.ctx.device, &self.ctx.queue, &mut pass, thumb_scale, thumb_rects);
                // вернуть scissor на весь экран
                pass.set_scissor_rect(0, 0, self.ctx.config.width, self.ctx.config.height);
            }
            // 4) overlay-прямоугольники (рамка активной, бейджи) — ПОВЕРХ миниатюр
            self.ui_overlay.draw(&mut pass);
            // 5) текст и глифы
            self.text.draw(&mut pass).map_err(LuminaError::Gpu)?;
        }
        self.ctx.queue.submit(Some(encoder.finish()));
        frame.present();
        self.text.trim();
        Ok(())
    }

    /// scale для скругления миниатюр выводим из высоты titlebar (titlebar_h / TITLEBAR_HEIGHT),
    /// fallback 1.0 в fullscreen (titlebar_h==0).
    fn scale_for_thumbs(&self) -> f32 {
        if self.titlebar_h > 0.0 {
            self.titlebar_h / crate::ui::theme::TITLEBAR_HEIGHT
        } else {
            1.0
        }
    }
}
