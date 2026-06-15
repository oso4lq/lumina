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
use ui_pipeline::UiPipeline;
use winit::window::Window;

pub struct Renderer {
    ctx: GpuContext,
    blit: BlitPipeline,
    ui: UiPipeline,
    text: TextLayer,
    image_size: Option<Vec2>,
    /// Высота titlebar в физ. px (viewer = всё ниже неё).
    titlebar_h: f32,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let ctx = GpuContext::new(window)?;
        let blit = BlitPipeline::new(&ctx.device, ctx.config.format);
        let ui = UiPipeline::new(&ctx.device, ctx.config.format);
        let text = TextLayer::new(&ctx.device, &ctx.queue, ctx.config.format);
        Ok(Self { ctx, blit, ui, text, image_size: None, titlebar_h: 0.0 })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
    }

    pub fn surface_size(&self) -> Vec2 {
        Vec2::new(self.ctx.config.width as f32, self.ctx.config.height as f32)
    }

    /// Размер viewer-региона (под titlebar) — для математики вида.
    pub fn viewer_size(&self) -> Vec2 {
        let s = self.surface_size();
        Vec2::new(s.x, (s.y - self.titlebar_h).max(1.0))
    }

    pub fn set_titlebar_height(&mut self, h: f32) {
        self.titlebar_h = h;
    }

    pub fn image_size(&self) -> Option<Vec2> {
        self.image_size
    }

    pub fn upload_texture(&mut self, rgba: &[u8], width: u32, height: u32) {
        self.blit.upload(&self.ctx.device, &self.ctx.queue, rgba, width, height);
        self.image_size = Some(Vec2::new(width as f32, height as f32));
    }

    /// Один кадр: фото в viewer-viewport, затем UI-прямоугольники, затем текст.
    pub fn render(&mut self, view: &ViewTransform, cmds: &[DrawCmd]) -> Result<()> {
        let frame = self
            .ctx
            .surface
            .get_current_texture()
            .map_err(|e| LuminaError::Gpu(format!("get_current_texture: {e}")))?;
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let screen = [self.ctx.config.width as f32, self.ctx.config.height as f32];

        // Прямоугольники из draw-команд
        let rects: Vec<(Rect, [f32; 4], f32)> = cmds
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Rect { rect, color, radius } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        self.ui.prepare(&self.ctx.device, &self.ctx.queue, screen, &rects);

        // Текст и глифы
        self.text
            .prepare(
                &self.ctx.device,
                &self.ctx.queue,
                (self.ctx.config.width, self.ctx.config.height),
                cmds,
            )
            .map_err(LuminaError::Gpu)?;

        // viewer-регион под titlebar. В вырожденно маленьком окне (высота меньше
        // titlebar — бывает кратко при SWP_FRAMECHANGED) места под фото нет: высоту
        // клампим к окну, а блит пропускаем, иначе set_viewport уходит за таргет.
        let bar_h = self.titlebar_h.min(screen[1]);
        let viewer_h = screen[1] - bar_h;
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
            // 1) фото в viewer-регионе (если под titlebar есть место)
            if viewer_ok {
                self.blit.draw(&self.ctx.queue, &mut pass, view, self.image_size, viewer);
                // viewport обратно на весь экран для UI/текста
                pass.set_viewport(0.0, 0.0, screen[0], screen[1], 0.0, 1.0);
            }
            // 2) UI-прямоугольники
            self.ui.draw(&mut pass);
            // 3) текст
            self.text.draw(&mut pass).map_err(LuminaError::Gpu)?;
        }
        self.ctx.queue.submit(Some(encoder.finish()));
        frame.present();
        self.text.trim();
        Ok(())
    }
}
