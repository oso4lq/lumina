mod context;
mod pipeline;

use crate::error::Result;
use crate::view::ViewTransform;
use context::GpuContext;
use pipeline::BlitPipeline;
use glam::Vec2;
use std::sync::Arc;
use winit::window::Window;

pub struct Renderer {
    ctx: GpuContext,
    pipeline: BlitPipeline,
    image_size: Option<Vec2>,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let ctx = GpuContext::new(window)?;
        let pipeline = BlitPipeline::new(&ctx.device, ctx.config.format);
        Ok(Self { ctx, pipeline, image_size: None })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
    }

    pub fn surface_size(&self) -> Vec2 {
        Vec2::new(self.ctx.config.width as f32, self.ctx.config.height as f32)
    }

    pub fn image_size(&self) -> Option<Vec2> {
        self.image_size
    }

    pub fn upload_texture(&mut self, rgba: &[u8], width: u32, height: u32) {
        self.pipeline.upload(&self.ctx.device, &self.ctx.queue, rgba, width, height);
        self.image_size = Some(Vec2::new(width as f32, height as f32));
    }

    pub fn render(&mut self, view: &ViewTransform) -> Result<()> {
        self.pipeline.render(&self.ctx, view, self.image_size)
    }
}
