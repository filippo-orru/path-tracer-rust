use std::borrow::Cow;

use encase::{StorageBuffer, UniformBuffer};
use glam::{Mat4, Vec2, Vec3};
use iced::{
    Rectangle,
    widget::shader::{
        self, Primitive,
        wgpu::{self, ShaderModuleDescriptor},
    },
};

use crate::render::{Triangle, camera_data::CameraData};

#[derive(Debug)]
pub struct ViewportPrimitive {
    pub triangles: Vec<TriangleWithColor>,
    pub camera: CameraData,
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
        if !storage.has::<FragmentShaderPipeline>() {
            storage.store(FragmentShaderPipeline::new(device, format, viewport));
        }

        let pipeline = storage.get_mut::<FragmentShaderPipeline>().unwrap();
        if pipeline.viewport.physical_height() != viewport.physical_height()
            || pipeline.viewport.physical_width() != viewport.physical_width()
        {
            // println!(
            //     "Viewport size changed: {:?} -> {:?}",
            //     pipeline.viewport, viewport
            // );
            *pipeline = FragmentShaderPipeline::new(device, format, viewport);
        }

        // println!("Target size: {:?}", target_size);

        // Create view matrix from camera

        let sensor_origin: Vec3 = self.camera.position;
        let sensor_height: f32 = self.camera.sensor_height();
        let focal_length: f32 = self.camera.focal_length;
        // Create a "look-at" target point from the camera position and direction
        let lens_center = self.camera.lens_center();
        // println!("Lens center: {:?}", lens_center);

        let up = Vec3::new(0.0, 1.0, 0.0);

        // Create view matrix
        let view = Mat4::look_at_rh(sensor_origin, lens_center, up);

        // Calculate FOV based on sensor height and focal length
        let fov = 2.0 * (sensor_height / (2.0 * focal_length)).atan();

        // Debug info to compare with the fixed value
        // println!(
        //     "Calculated FOV: {:.3} radians ({:.1} degrees)",
        //     fov,
        //     fov.to_degrees()
        // );

        let aspect_ratio = target_size.width as f32 / target_size.height as f32;
        let projection = Mat4::perspective_rh(fov, aspect_ratio, 0.001, 1000.0);

        // Combine view and projection matrices
        let view_proj = projection * view;

        // Debug output to help diagnose any remaining issues
        // println!("Camera pos: {:?}, dir: {:?}", camera_pos, camera_dir);
        // println!("FOV: {} degrees", fov_y.to_degrees());
        // println!("View matrix: {:?}", view);
        // println!("Projection: {:?}", projection);
        // println!("View-proj: {:?}", view_proj);

        pipeline.update(
            queue,
            objects_shader::types::MyUniforms {
                view_proj: view_proj,
                resolution: Vec2::new(target_size.width as f32, target_size.height as f32),
            },
            &self.triangles,
        );
    }

    fn render(
        &self,
        encoder: &mut shader::wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &shader::wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        storage
            .get::<FragmentShaderPipeline>()
            .unwrap()
            .render(target, encoder, *clip_bounds);
    }
}

#[include_wgsl_oil::include_wgsl_oil("shaders/objects.wgsl")]
mod objects_shader {}

#[include_wgsl_oil::include_wgsl_oil("shaders/sky.wgsl")]
mod sky_shader {}

struct FragmentShaderPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    verts_count: u32,
    depth_texture_view: wgpu::TextureView,
    viewport: shader::Viewport,
}
impl FragmentShaderPipeline {
    fn new(
        device: &shader::wgpu::Device,
        format: shader::wgpu::TextureFormat,
        viewport: &shader::Viewport,
    ) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("objshader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(objects_shader::SOURCE)),
        });

        let size = wgpu::Extent3d {
            width: viewport.physical_width(),
            height: viewport.physical_height(),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
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

        let vertex_attributes: Vec<wgpu::VertexAttribute> = vec![
            // my_shader::types::Vertex::min_size()
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
            },
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FragmentShaderPipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: vertex_attributes
                        .iter()
                        .map(|attr| attr.format.size())
                        .sum::<u64>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vertex_attributes,
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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_quad uniform buffer"),
            size: std::mem::size_of::<objects_shader::types::MyUniforms>() as u64,
            // size: 0,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = pipeline.get_bind_group_layout(0);
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader_quad uniform bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_quad vertex buffer"),
            size: std::mem::size_of::<objects_shader::types::Vertex>() as u64 * 1024 * 40, // max 40k vertices
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer,
            verts_count: 0,
            depth_texture_view,
            viewport: viewport.clone(),
        }
    }

    fn update(
        &mut self,
        queue: &wgpu::Queue,
        uniforms: objects_shader::types::MyUniforms,
        triangles: &Vec<TriangleWithColor>,
    ) {
        let uniform_buffer_data = {
            let mut buffer = UniformBuffer::new(Vec::<u8>::new());
            buffer.write(&uniforms).unwrap();
            buffer.into_inner()
        };
        queue.write_buffer(&self.uniform_buffer, 0, &uniform_buffer_data);

        let vertex_buffer_data = {
            let mut buffer = StorageBuffer::new(Vec::<u8>::new());
            let vertices = triangles
                .iter()
                .flat_map(|tri| tri.to_vertices())
                .collect::<Vec<_>>();

            buffer.write(&vertices).unwrap();
            buffer.into_inner()
        };
        queue.write_buffer(&self.vertex_buffer, 0, &vertex_buffer_data);

        self.verts_count = triangles.len() as u32 * 3;
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("renderpass"),
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

        pass.draw(0..self.verts_count, 0..1);
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
