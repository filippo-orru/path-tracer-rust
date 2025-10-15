use glam::{Mat4, Vec2, Vec3};
use iced::{
    advanced::Shell,
    event, mouse,
    widget::shader::{
        self,
        wgpu::{self},
        Primitive,
    },
    Rectangle,
};

use crate::render::{CameraData, RenderConfig, Triangle};

pub struct ViewportProgram {
    pub config: RenderConfig,
    // pub controls: Controls,
}

#[derive(Debug)]
pub struct ViewportPrimitive {
    pub triangles: Vec<TriangleWithColor>,
    pub camera: CameraData,
}

impl ViewportPrimitive {
    fn get_vertex_buffer(&self) -> Vec<f32> {
        let mut verts = Vec::with_capacity(self.triangles.len() * 3 * (2 + 4)); // pos + color

        for triangle in self.triangles.iter() {
            for position in &[triangle.tri.a, triangle.tri.b, triangle.tri.c] {
                {
                    let color = triangle.color.to_array();
                    verts.extend_from_slice(&position.to_array());
                    verts.push(1.0);
                    verts.extend_from_slice(&color);
                    verts.push(1.0);
                };
            }
        }

        verts
    }
}

#[derive(Debug)]
pub struct TriangleWithColor {
    pub tri: Triangle,
    pub color: Vec3,
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    view_proj: [f32; 4 * 4],
    padding: [f32; 80], // Uniforms need to be min. 80 bytes, idk why
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
        let sensor_view_direction: Vec3 = self.camera.direction.normalize();
        let sensor_width: f32 = self.camera.sensor_width;
        let focal_length: f32 = self.camera.focal_length;
        // lens center (pinhole)
        let lens_center = sensor_origin + sensor_view_direction * focal_length;

        // Create a "look-at" target point from the camera position and direction
        let target = lens_center + sensor_view_direction;

        // Assuming Y is up in your world space
        let up = Vec3::new(0.0, 1.0, 0.0);

        // Create view matrix
        let view = Mat4::look_at_rh(sensor_origin, target, up);
        let fov = 2.0 * (sensor_width / (2.0 * focal_length)).atan();

        let aspect_ratio = target_size.width / target_size.height;
        // Using right-handed coordinate system for consistency
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
            &Uniforms {
                view_proj: view_proj.to_cols_array(),
                padding: [0.0; 80],
            },
            &self.get_vertex_buffer(),
            self.triangles.len() as u32 * 3,
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
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
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

        let vertex_attributes = [
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
            size: std::mem::size_of::<Uniforms>() as u64,
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
            size: std::mem::size_of::<[f32; 4 * 2]>() as u64 * 1024 * 4 * 10, // 40k vertices with pos+color
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
        uniforms: &Uniforms,
        vertex_buffer: &[f32],
        verts_count: u32,
    ) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertex_buffer));
        self.verts_count = verts_count;
        // println!("Updated verts: {:?}", vertex_buffer);
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
        // Clear the viewport with a blue color
        pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
        let blue = wgpu::Color {
            r: 0.0,
            g: 0.1,
            b: 0.2,
            a: 1.0,
        };
        pass.set_blend_constant(blue);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        pass.draw(0..self.verts_count, 0..1);
    }
}

#[derive(Default)]
pub struct ViewportState {
    pub offset: Vec2,
}

impl<Message> shader::Program<Message> for ViewportProgram {
    type State = ViewportState;

    type Primitive = ViewportPrimitive;

    fn draw(
        &self,
        state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        ViewportPrimitive {
            triangles: self
                .config
                .scene
                .objects
                .iter()
                .flat_map(|object| {
                    object
                        .to_triangles()
                        .into_iter()
                        .map(|tri| TriangleWithColor {
                            tri: tri.transformed(&object.position),
                            color: object.material.color,
                        })
                })
                .collect(),
            camera: self.config.scene.camera,
        }
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: shader::Event,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _shell: &mut Shell<'_, Message>,
    ) -> (event::Status, Option<Message>) {
        match event {
            shader::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                // state.offset = Vec2::new(
                //     ((position.x - bounds.x) / bounds.width) * 2.0 - 1.0,
                //     -(((position.y - bounds.y) / bounds.height) * 2.0 - 1.0),
                // );

                (event::Status::Captured, None)
            }
            _ => (event::Status::Ignored, None),
        }
    }
}
