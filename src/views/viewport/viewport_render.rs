use std::borrow::Cow;

use encase::{ShaderSize, ShaderType, internal::WriteInto};
use glam::{Vec2, Vec3};
use iced::{
    Rectangle,
    widget::shader::{
        self, Primitive,
        wgpu::{self, ShaderModuleDescriptor},
    },
};

use crate::{
    render::{SceneData, Triangle, camera_data::CameraData},
    views::{
        viewport::viewport_render::objects_types_shader::types::Vertex, viewport_tab::ViewportState,
    },
};

use std::marker::PhantomData;

#[include_wgsl_oil::include_wgsl_oil("shaders/sky.wgsl")]
mod sky_shader {}

#[derive(Debug)]
pub struct ViewportPrimitive {
    pub scene: SceneData,
    pub viewport_state: ViewportState,
}

impl Primitive for ViewportPrimitive {
    fn prepare(
        &self,
        device: &shader::wgpu::Device,
        queue: &shader::wgpu::Queue,
        format: shader::wgpu::TextureFormat,
        storage: &mut shader::Storage,
        target_size: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        // Check if viewport size changed
        if !storage.has::<RenderPipelines>() {
            storage.store(RenderPipelines::new(device, format, viewport, target_size));
        }

        let pipelines_cache = storage.get_mut::<RenderPipelines>().unwrap();
        if pipelines_cache.viewport.physical_height() != viewport.physical_height()
            || pipelines_cache.viewport.physical_width() != viewport.physical_width()
        {
            *pipelines_cache = RenderPipelines::new(device, format, viewport, target_size);
        }

        pipelines_cache.update(queue, &self.scene, &self.viewport_state);
    }

    fn render(
        &self,
        encoder: &mut shader::wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &shader::wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        let render_pipelines = storage.get::<RenderPipelines>().unwrap();

        render_pipelines.render(target, encoder, *clip_bounds);
    }
}

struct RenderPipelines {
    sky_layer: SkyLayer,
    objects_layer: ObjectsLayer,
    viewport: shader::Viewport,
}
impl RenderPipelines {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
        target_size: &Rectangle,
    ) -> Self {
        Self {
            sky_layer: SkyLayer::new(device, format, viewport),
            objects_layer: ObjectsLayer::new(device, format, viewport, target_size),
            viewport: viewport.clone(),
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, scene: &SceneData, viewport_state: &ViewportState) {
        self.sky_layer.update(queue, scene);
        self.objects_layer.update(queue, scene, viewport_state);
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clip_bounds: Rectangle<u32>,
    ) {
        let mut sky_objects_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sky_objects_renderpass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.objects_layer.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let viewport = clip_bounds;
        // println!("Viewport: {:?}", viewport);
        sky_objects_pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );

        // TODO
        //  consider moving the rendering of sky and objects into their own functions

        // Draw sky
        sky_objects_pass.set_pipeline(&self.sky_layer.pipeline.render_pipeline);
        sky_objects_pass.set_bind_group(0, &self.sky_layer.pipeline.bind_group, &[]);
        sky_objects_pass.set_vertex_buffer(0, self.sky_layer.vertex_buffer.buffer.slice(..));
        sky_objects_pass.draw(0..self.sky_layer.vertex_buffer.vertex_count, 0..1);

        // write depth texture to buffer and use it for outlines.
        sky_objects_pass.set_pipeline(&self.objects_layer.objects.render_pipeline);
        sky_objects_pass.set_bind_group(0, &self.objects_layer.objects.bind_group, &[]);
        sky_objects_pass.set_vertex_buffer(0, self.objects_layer.vertex_buffer.buffer.slice(..));
        sky_objects_pass.draw(0..self.objects_layer.vertex_buffer.vertex_count, 0..1);
        drop(sky_objects_pass);

        // Draw outlines
        let mut outline_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("outline_renderpass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        outline_pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );
        outline_pass.set_pipeline(&self.objects_layer.outline.render_pipeline);
        outline_pass.set_bind_group(0, &self.objects_layer.outline.bind_group, &[]);
        outline_pass.set_vertex_buffer(0, self.objects_layer.vertex_buffer.buffer.slice(..));
        outline_pass.draw(0..self.objects_layer.vertex_buffer.vertex_count, 0..1);
    }
}

struct FragmentShaderPipeline<Uniforms: WriteInto + ShaderType> {
    render_pipeline: wgpu::RenderPipeline,

    bind_group: wgpu::BindGroup,
    uniform_buffer: UniformBuffer<Uniforms>,

    viewport: shader::Viewport,
}

struct VertexBuffer<Vert: WriteInto + ShaderType + ShaderSize> {
    buffer: wgpu::Buffer,
    vertex_count: u32,

    _vert_marker: PhantomData<Vert>,
}
impl<Vert: WriteInto + ShaderType + ShaderSize> VertexBuffer<Vert> {
    fn new(device: &wgpu::Device, label: &str, max_vertices: usize) -> Self {
        let vertex_struct_size = std::mem::size_of::<Vert>();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_vertex_buffer")),
            size: (vertex_struct_size * max_vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            vertex_count: 0,
            _vert_marker: PhantomData,
        }
    }

    fn set_data(&mut self, queue: &wgpu::Queue, verts: &Vec<Vert>) {
        self.vertex_count = verts.len() as u32;

        let mut buffer = encase::StorageBuffer::new(Vec::<u8>::new());
        buffer.write(verts).unwrap();
        let buf = buffer.into_inner();

        let buf = if buf.len() < 80 {
            let mut padded = buf.clone();
            padded.resize(80, 0);
            padded
        } else {
            buf
        };

        queue.write_buffer(&self.buffer, 0, &buf);
    }
}

struct UniformBuffer<Uniforms: WriteInto + ShaderType> {
    buffer: wgpu::Buffer,
    _uniforms_marker: PhantomData<Uniforms>,
}
impl<Uniforms: WriteInto + ShaderType> UniformBuffer<Uniforms> {
    const MIN_SIZE: usize = 64; // Minimum size for wgpu uniform buffers

    fn new(device: &wgpu::Device, label: &str) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_uniform_buffer")),
            size: Self::_size(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer: uniform_buffer,
            _uniforms_marker: PhantomData,
        }
    }
    fn set_data(&self, queue: &wgpu::Queue, uniforms: Uniforms) {
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&uniforms).unwrap();
        let buf = buffer.into_inner();
        let buf = if buf.len() < Self::MIN_SIZE {
            let mut padded = buf.clone();
            padded.resize(Self::MIN_SIZE, 0);
            padded
        } else {
            buf
        };

        queue.write_buffer(&self.buffer, 0, &buf);
    }

    fn _size() -> u64 {
        (std::mem::size_of::<Uniforms>()).max(Self::MIN_SIZE) as u64
    }
}

#[include_wgsl_oil::include_wgsl_oil("shaders/objects_types.wgsl")]
mod objects_types_shader {}

#[include_wgsl_oil::include_wgsl_oil("shaders/objects.wgsl")]
mod objects_shader {}

#[include_wgsl_oil::include_wgsl_oil("shaders/outline.wgsl")]
mod outline_shader {}

struct ObjectsLayer {
    depth_texture_view: wgpu::TextureView,
    vertex_buffer: VertexBuffer<Vertex>,

    objects: FragmentShaderPipeline<objects_shader::types::Uniforms>,
    outline: FragmentShaderPipeline<outline_shader::types::Uniforms>,

    target_size: Rectangle,
}
impl ObjectsLayer {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
        target_size: &Rectangle,
    ) -> Self {
        let layer_label = "objects_layer";
        let size = wgpu::Extent3d {
            width: viewport.physical_width(),
            height: viewport.physical_height(),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(&format!("{layer_label}_depth_texture")),
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

        let mut vertex_struct_size = 0;
        let vertex_attributes_mapped: Vec<wgpu::VertexAttribute> = vec![
            wgpu::VertexFormat::Float32x4, // position
            wgpu::VertexFormat::Float32x4, // color
        ]
        .into_iter()
        .enumerate()
        .map(|(index, format)| {
            let attr = wgpu::VertexAttribute {
                offset: vertex_struct_size,
                shader_location: index as u32,
                format,
            };
            vertex_struct_size += format.size();
            attr
        })
        .collect();

        // TODO
        //  consider making the buffer smaller and drawing meshes in chunks
        let vertex_buffer = VertexBuffer::new(device, layer_label, 1024 * 40);

        Self {
            objects: {
                let label: &str = "object";
                let shader_source: &str = objects_shader::SOURCE;
                let shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: Some(&format!("{label}_shader")),
                    source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source)),
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
                            format: format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                });

                let uniform_buffer = UniformBuffer::new(device, label);

                let bind_group_layout = pipeline.get_bind_group_layout(0);
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("{label}_bind_group_0")),
                    layout: &bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.buffer.as_entire_binding(),
                    }]
                    .into_iter()
                    .chain(None.unwrap_or(vec![]))
                    .collect::<Vec<_>>(),
                });

                FragmentShaderPipeline {
                    render_pipeline: pipeline,
                    uniform_buffer,
                    bind_group,
                    viewport: viewport.clone(),
                }
            },
            outline: {
                let label: &str = "outline";
                let shader_source: &str = outline_shader::SOURCE;
                let shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: Some(&format!("{label}_shader")),
                    source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source)),
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
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fragment_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                });

                let uniform_buffer = UniformBuffer::new(device, label);

                let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("outline_depth_sampler"),
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                });

                let bind_group_layout = pipeline.get_bind_group_layout(0);
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("{label}_bind_group_0")),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&depth_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&texture_sampler),
                        },
                    ]
                    .into_iter()
                    .chain(None.unwrap_or(vec![]))
                    .collect::<Vec<_>>(),
                });

                FragmentShaderPipeline {
                    render_pipeline: pipeline,
                    uniform_buffer,
                    bind_group,
                    viewport: viewport.clone(),
                }
            },
            depth_texture_view,
            vertex_buffer,
            target_size: target_size.clone(),
        }
    }

    fn get_view_proj(&self, scene: &SceneData) -> glam::Mat4 {
        // Create view matrix
        let aspect_ratio = self.target_size.width as f32 / self.target_size.height as f32;
        return scene.camera.get_view_projection(aspect_ratio);
    }

    fn get_verts(scene: &SceneData, viewport_state: &ViewportState) -> Vec<Vertex> {
        let grid_tris = Self::get_grid(&scene.camera);
        let object_tris = scene
            .objects
            .iter()
            .enumerate()
            .flat_map(|(object_id, object)| {
                object
                    .to_triangles()
                    .into_iter()
                    .map(move |tri| TriangleWithColor {
                        tri: tri.transformed(object.position), // TODO apply transformation in vertex shader
                        color: if viewport_state.selected_object == Some(object_id) {
                            Vec3::new(1.0, 0.0, 0.0)
                        } else {
                            object.material.color
                        },
                    })
            });

        let tris = grid_tris
            .into_iter()
            .chain(object_tris)
            // .chain(ray_tris)
            .flat_map(|tri| tri.to_vertices())
            .collect();
        tris
    }

    fn update(&mut self, queue: &wgpu::Queue, scene: &SceneData, viewport_state: &ViewportState) {
        let view_proj = self.get_view_proj(scene);
        let verts = Self::get_verts(scene, viewport_state);

        self.objects
            .uniform_buffer
            .set_data(queue, objects_shader::types::Uniforms { view_proj });
        self.outline
            .uniform_buffer
            .set_data(queue, outline_shader::types::Uniforms { view_proj });
        self.vertex_buffer.set_data(queue, &verts);
    }

    fn get_grid(camera: &CameraData) -> Vec<TriangleWithColor> {
        let grid_lines = 5;
        let zoom_level = camera.position.length() / 5.0;
        let spacing = 10_i32.pow((zoom_level * 1.2 + 1.0).log10().floor() as u32) as f32;
        // println!("zoom: {zoom_level}, spacing: {spacing}");
        let line_width = 0.02 * zoom_level;

        let mut tris = vec![];

        for axis in [Vec3::X, Vec3::Z] {
            let axis_vec = (axis, Vec3::new(0.0, 1.0, 0.0).cross(axis));

            for i in -grid_lines..=grid_lines {
                let offset = i as f32 * spacing;

                // Create positions based on the current axis
                let position1 = axis_vec.0 * (offset - line_width / 2.0)
                    - axis_vec.1 * (grid_lines as f32 * spacing);
                let position2 = axis_vec.0 * (offset + line_width / 2.0)
                    - axis_vec.1 * (grid_lines as f32 * spacing);
                let position3 = position1 + axis_vec.1 * (grid_lines as f32 * spacing * 2.0);
                let position4 = position2 + axis_vec.1 * (grid_lines as f32 * spacing * 2.0);

                let color = Vec3::new(0.5, 0.5, 0.5);

                tris.extend(TriangleWithColor::from_quad(
                    position1, position2, position4, position3, color,
                ));
            }
        }

        tris
    }
}

struct SkyLayer {
    pipeline: FragmentShaderPipeline<sky_shader::types::Uniforms>,
    vertex_buffer: VertexBuffer<Vertex>,
    // target_size: Rectangle,
    verts: Vec<Vertex>,
}
impl SkyLayer {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
        // target_size: &Rectangle,
    ) -> Self {
        let label: &str = "sky";
        let vertex_buffer = VertexBuffer::new(
            device, label, 6, // max 6 vertices for a full-screen quad
        );

        let verts = vec![
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
        .flat_map(|t| t.to_vertices())
        .collect();

        Self {
            pipeline: {
                let shader_source: &str = sky_shader::SOURCE;
                let shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: Some(&format!("{label}_shader")),
                    source: wgpu::ShaderSource::Wgsl(Cow::from(shader_source)),
                });

                let mut vertex_struct_size = 0;
                let vertex_attributes_mapped: Vec<wgpu::VertexAttribute> = vec![
                    wgpu::VertexFormat::Float32x4, // position
                    wgpu::VertexFormat::Float32x4, // color
                ]
                .into_iter()
                .enumerate()
                .map(|(index, format)| {
                    let attr = wgpu::VertexAttribute {
                        offset: vertex_struct_size,
                        shader_location: index as u32,
                        format,
                    };
                    vertex_struct_size += format.size();
                    attr
                })
                .collect();

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
                            format: format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                });

                let uniform_buffer = UniformBuffer::new(device, label);

                let bind_group_layout = pipeline.get_bind_group_layout(0);
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("{label}_bind_group_0")),
                    layout: &bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.buffer.as_entire_binding(),
                    }]
                    .into_iter()
                    .chain(None.unwrap_or(vec![]))
                    .collect::<Vec<_>>(),
                });

                FragmentShaderPipeline {
                    render_pipeline: pipeline,
                    uniform_buffer,
                    bind_group,
                    viewport: viewport.clone(),
                }
            },
            vertex_buffer,
            // target_size: target_size.clone(),
            verts,
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, scene: &SceneData) {
        // TODO check if the size is correct
        let size = self.pipeline.viewport.physical_size();

        self.pipeline.uniform_buffer.set_data(
            queue,
            sky_shader::types::Uniforms {
                top_color: Vec3::new(0.2, 0.2, 0.2),
                bottom_color: Vec3::new(0.13, 0.1, 0.1),
                resolution: Vec2::new(size.width as f32, size.height as f32),
                camera_direction: scene.camera.direction(),
            },
        );
        self.vertex_buffer.set_data(queue, &self.verts);
    }
}

#[derive(Debug)]
pub struct TriangleWithColor {
    pub tri: Triangle,
    pub color: Vec3,
}

impl TriangleWithColor {
    fn from_quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Vec3) -> Vec<TriangleWithColor> {
        vec![
            TriangleWithColor {
                tri: Triangle { a, b, c },
                color,
            },
            TriangleWithColor {
                tri: Triangle { a, b: c, c: d },
                color,
            },
        ]
    }

    fn to_vertices(&self) -> Vec<Vertex> {
        [self.tri.a, self.tri.b, self.tri.c]
            .into_iter()
            .map(|position| Vertex {
                position: position.extend(1.0),
                color: self.color.extend(1.0),
            })
            .collect()
    }
}
