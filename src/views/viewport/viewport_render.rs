use std::borrow::Cow;

use encase::{ShaderSize, ShaderType, StorageBuffer, UniformBuffer, internal::WriteInto};
use glam::{Mat4, Vec2, Vec3, Vec4};
use iced::{
    Rectangle,
    widget::shader::{
        self, Primitive,
        wgpu::{self, ShaderModuleDescriptor},
    },
};

use crate::render::{RenderConfig, Triangle};

#[derive(Debug)]
pub struct ViewportPrimitive {
    pub config: RenderConfig,
}

struct RenderPipelineCache {
    sky: SkyFragmentShaderPipeline,
    objects: ObjectFragmentShaderPipeline,
    viewport: shader::Viewport,
}
impl RenderPipelineCache {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
    ) -> Self {
        Self {
            sky: SkyFragmentShaderPipeline::new(device, format, viewport),
            objects: ObjectFragmentShaderPipeline::new(device, format, viewport),
            viewport: viewport.clone(),
        }
    }
}

impl Primitive for ViewportPrimitive {
    fn prepare(
        &self,
        device: &shader::wgpu::Device,
        queue: &shader::wgpu::Queue,
        format: shader::wgpu::TextureFormat,
        storage: &mut shader::Storage,
        _target_size: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        // Check if viewport size changed
        if !storage.has::<RenderPipelineCache>() {
            storage.store(RenderPipelineCache::new(device, format, viewport));
        }

        let pipelines_cache = storage.get_mut::<RenderPipelineCache>().unwrap();
        if pipelines_cache.viewport.physical_height() != viewport.physical_height()
            || pipelines_cache.viewport.physical_width() != viewport.physical_width()
        {
            *pipelines_cache = RenderPipelineCache::new(device, format, viewport);
        }

        pipelines_cache.sky.update(queue, &self.config);
        pipelines_cache.objects.update(queue, &self.config);
    }

    fn render(
        &self,
        encoder: &mut shader::wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &shader::wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        let render_pipeline_cache = storage.get::<RenderPipelineCache>().unwrap();

        render_pipeline_cache
            .sky
            .pipeline
            .render(target, encoder, *clip_bounds);

        render_pipeline_cache
            .objects
            .pipeline
            .render(target, encoder, *clip_bounds);
    }
}

#[include_wgsl_oil::include_wgsl_oil("shaders/objects.wgsl")]
mod objects_shader {}

#[include_wgsl_oil::include_wgsl_oil("shaders/sky.wgsl")]
mod sky_shader {}

// struct Buffers {
//     uniforms: objects_shader::types::MyUniforms,
//     triangles: &Vec<TriangleWithColor>,
// }

use std::marker::PhantomData;

struct FragmentShaderPipeline<
    Uniforms: WriteInto + encase::ShaderType,
    Vert: WriteInto + encase::ShaderType,
> {
    label: String,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    verts_count: u32,
    depth_texture_view: wgpu::TextureView,
    viewport: shader::Viewport,
    _uniforms_marker: PhantomData<Uniforms>,
    _vert_marker: PhantomData<Vert>,
}

impl<Uniforms, Vert> FragmentShaderPipeline<Uniforms, Vert>
where
    Uniforms: ShaderType + WriteInto,
    Vert: ShaderType + WriteInto + ShaderSize,
{
    fn new(
        label: &str,
        shader_source: &str,
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
        vertex_attributes: Option<Vec<wgpu::VertexFormat>>,
    ) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&format!("{label}_shader")),
            source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source)),
        });

        let size = wgpu::Extent3d {
            width: viewport.physical_width(),
            height: viewport.physical_height(),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(&format!("{label}_depth_texture")),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let depth_texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut vertex_attributes_mapped: Vec<wgpu::VertexAttribute> = vec![];
        let mut offset = 0;
        for (index, attr) in vertex_attributes
            .unwrap_or(Self::default_vertext_attrs())
            .into_iter()
            .enumerate()
        {
            vertex_attributes_mapped.push(wgpu::VertexAttribute {
                offset,
                shader_location: index as u32,
                format: attr,
            });
            offset += attr.size();
        }

        let vertex_struct_size = offset;

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_vertex_buffer")),
            size: vertex_struct_size * 1024 * 40, // max 40k vertices
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{label}_fragment_pipeline")),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: vertex_struct_size as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vertex_attributes_mapped,
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let uniform_struct_size = std::mem::size_of::<Uniforms>().max(80); // Uniforms must be at least 80 bytes
        // println!(
        //     "Uniform struct size: {} (would be {})",
        //     uniform_struct_size,
        //     std::mem::size_of::<Uniforms>()
        // );
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_uniform_buffer")),
            size: uniform_struct_size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = pipeline.get_bind_group_layout(0);
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label}_uniform_bind_group")),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            label: label.to_string(),
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer,
            verts_count: 0,
            depth_texture_view,
            viewport: viewport.clone(),
            _uniforms_marker: PhantomData,
            _vert_marker: PhantomData,
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, uniforms: Uniforms, verts: Vec<Vert>) {
        self.verts_count = verts.len() as u32;

        let uniform_buf = Self::uniform_buf(uniforms);
        // println!("Uploading {:?} to uniform buffer", &uniform_buf);
        queue.write_buffer(&self.uniform_buffer, 0, &uniform_buf);

        let verts = Self::storage_buf(verts);
        // println!("Uploading {:?} to vertex buffer", verts);
        queue.write_buffer(&self.vertex_buffer, 0, &verts);
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("{}_renderpass", self.label)),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // println!("Viewport: {:?}", viewport);

        pass.set_pipeline(&self.pipeline);
        pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        // println!("Drawing {} vertices", self.verts_count);
        pass.draw(0..self.verts_count, 0..1);
    }

    fn uniform_buf(uniforms: Uniforms) -> Vec<u8> {
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&uniforms).unwrap();
        let buf = buffer.into_inner();
        if buf.len() < 80 {
            let mut padded = buf.clone();
            padded.resize(80, 0);
            padded
        } else {
            buf
        }
    }

    fn storage_buf(slice: Vec<Vert>) -> Vec<u8> {
        let mut buffer = StorageBuffer::new(Vec::<u8>::new());
        buffer.write(&slice).unwrap();
        buffer.into_inner()
    }

    fn default_vertext_attrs() -> Vec<wgpu::VertexFormat> {
        vec![
            wgpu::VertexFormat::Float32x4, // position
            wgpu::VertexFormat::Float32x4, // color
        ]
    }
}

struct ObjectFragmentShaderPipeline {
    pipeline:
        FragmentShaderPipeline<objects_shader::types::MyUniforms, objects_shader::types::Vertex>,
}
impl ObjectFragmentShaderPipeline {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
    ) -> Self {
        Self {
            pipeline: FragmentShaderPipeline::new(
                "object",
                objects_shader::SOURCE,
                device,
                format,
                viewport,
                None,
            ),
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, config: &RenderConfig) {
        // Create view matrix
        let view_proj = {
            let sensor_origin: Vec3 = config.scene.camera.position;
            let sensor_height: f32 = config.scene.camera.sensor_height();
            let focal_length: f32 = config.scene.camera.focal_length;
            let lens_center = config.scene.camera.lens_center();

            let up = Vec3::new(0.0, 1.0, 0.0);

            let view = Mat4::look_at_rh(sensor_origin, lens_center, up);
            let fov = 2.0 * (sensor_height / (2.0 * focal_length)).atan();

            let aspect_ratio = self.pipeline.viewport.physical_width() as f32
                / self.pipeline.viewport.physical_height() as f32;
            let projection = Mat4::perspective_rh(fov, aspect_ratio, 0.001, 1000.0);

            projection * view
        };

        let verts = {
            config
                .scene
                .objects
                .iter()
                .flat_map(|object| {
                    object.to_triangles().into_iter().flat_map(|tri| {
                        TriangleWithColor {
                            tri: tri.transformed(&object.position),
                            color: object.material.color,
                        }
                        .to_vertices()
                    })
                })
                .collect()
        };

        self.pipeline.update(
            queue,
            objects_shader::types::MyUniforms {
                view_proj: view_proj,
                // resolution: Vec2::new(
                //     self.pipeline.viewport.physical_width() as f32,
                //     self.pipeline.viewport.physical_height() as f32,
                // ),
            },
            verts,
        );
    }
}

struct SkyFragmentShaderPipeline {
    pipeline: FragmentShaderPipeline<sky_shader::types::Uniforms, sky_shader::types::Vertex>,
}
impl SkyFragmentShaderPipeline {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
    ) -> Self {
        Self {
            pipeline: FragmentShaderPipeline::new(
                "sky",
                sky_shader::SOURCE,
                device,
                format,
                viewport,
                None,
            ),
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, config: &RenderConfig) {
        let size = self.pipeline.viewport.physical_size();

        self.pipeline.update(
            queue,
            sky_shader::types::Uniforms {
                top_color: Vec3::new(0.2, 0.2, 0.2),
                bottom_color: Vec3::new(0.13, 0.1, 0.1),
                resolution: Vec2::new(size.width as f32, size.height as f32),
                camera_direction: config.scene.camera.get_current_direction(),
            },
            vec![
                TriangleWithColor {
                    tri: Triangle {
                        a: Vec3::new(-1.0, -1.0, 0.0),
                        b: Vec3::new(1.0, -1.0, 0.0),
                        c: Vec3::new(1.0, 1.0, 0.0),
                    },
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
                TriangleWithColor {
                    tri: Triangle {
                        a: Vec3::new(-1.0, -1.0, 0.0),
                        b: Vec3::new(1.0, 1.0, 0.0),
                        c: Vec3::new(-1.0, 1.0, 0.0),
                    },
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
            ]
            .into_iter()
            .flat_map(|t| {
                t.to_vertices()
                    .into_iter()
                    .map(|v| sky_shader::types::Vertex {
                        position: v.position,
                        color: v.color,
                    })
            })
            .collect(),
        );
    }
}

#[derive(Debug)]
pub struct TriangleWithColor {
    pub tri: Triangle,
    pub color: Vec3,
}

impl TriangleWithColor {
    fn to_vertices(&self) -> Vec<objects_shader::types::Vertex> {
        [self.tri.a, self.tri.b, self.tri.c]
            .into_iter()
            .map(|position| objects_shader::types::Vertex {
                position: position.extend(1.0),
                color: self.color.extend(1.0),
            })
            .collect()
    }
}
