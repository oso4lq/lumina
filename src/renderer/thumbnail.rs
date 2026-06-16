//! GPU-слой карусели: текстура на миниатюру, рисование квадом со SDF-скруглением.

use crate::ui::layout::Rect;
use crate::ui::theme;
use std::collections::HashMap;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Inst {
    pos: [f32; 2],
    size: [f32; 2],
    radius: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    screen: [f32; 2],
    _pad: [f32; 2],
}

struct Thumb {
    bind_group: wgpu::BindGroup,
}

pub struct ThumbnailLayer {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    globals: wgpu::Buffer,
    sampler: wgpu::Sampler,
    inst_buf: wgpu::Buffer,
    thumbs: HashMap<usize, Thumb>,
}

impl ThumbnailLayer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("thumb"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../assets/shaders/thumb.wgsl").into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("thumb-bind-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("thumb-globals"),
            contents: bytemuck::bytes_of(&Globals { screen: [1.0, 1.0], _pad: [0.0; 2] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("thumb-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("thumb-layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let inst_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Inst>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2, // pos
                1 => Float32x2, // size
                2 => Float32,   // radius
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("thumb-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[inst_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // один инстанс рисуется за раз (per-thumb bind group), буфер на 1 Inst
        let inst_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thumb-inst"),
            size: std::mem::size_of::<Inst>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { pipeline, bind_layout, globals, sampler, inst_buf, thumbs: HashMap::new() }
    }

    /// Загрузить/обновить текстуру миниатюры.
    pub fn set(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, index: usize, rgba: &[u8], w: u32, h: u32) {
        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("thumb-tex"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("thumb-bind"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.globals.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        self.thumbs.insert(index, Thumb { bind_group });
    }

    pub fn remove(&mut self, index: usize) {
        self.thumbs.remove(&index);
    }

    pub fn has(&self, index: usize) -> bool {
        self.thumbs.contains_key(&index)
    }

    /// Обновить экранные размеры (для NDC).
    pub fn set_screen(&self, queue: &wgpu::Queue, screen: [f32; 2]) {
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&Globals { screen, _pad: [0.0; 2] }));
    }

    /// Нарисовать видимые готовые миниатюры в открытый pass.
    /// Все инстансы пишутся в буфер ОДНОЙ записью (write_buffer применяется в начале
    /// сабмита, не упорядочен с draw — поэтому пер-draw перезапись давала стопку в одной точке),
    /// а отрисовка идёт через диапазон инстансов `i..i+1` со сменой bind group на текстуру.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass,
        scale: f32,
        rects: &[(usize, Rect)],
    ) {
        let radius = theme::THUMB_RADIUS * scale;
        let mut insts: Vec<Inst> = Vec::new();
        let mut order: Vec<usize> = Vec::new();
        for (idx, r) in rects {
            if r.w < 1.0 || r.h < 1.0 {
                continue;
            }
            if !self.thumbs.contains_key(idx) {
                continue;
            }
            insts.push(Inst { pos: [r.x, r.y], size: [r.w, r.h], radius, _pad: [0.0; 3] });
            order.push(*idx);
        }
        if insts.is_empty() {
            return;
        }
        let needed = (insts.len() * std::mem::size_of::<Inst>()) as u64;
        if self.inst_buf.size() < needed {
            self.inst_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("thumb-inst"),
                size: needed.next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&insts));
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.inst_buf.slice(..));
        for (i, idx) in order.iter().enumerate() {
            let Some(thumb) = self.thumbs.get(idx) else { continue };
            pass.set_bind_group(0, &thumb.bind_group, &[]);
            pass.draw(0..4, i as u32..i as u32 + 1);
        }
    }
}
