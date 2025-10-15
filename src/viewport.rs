use glam::{Mat4, Vec2, Vec3};
use iced::{
    advanced::Shell,
    event, mouse,
    widget::shader::{self, wgpu, Primitive},
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

        fn add_vert(verts: &mut Vec<f32>, position: Vec3, color: [f32; 3]) {
            verts.extend_from_slice(&[position.x, position.y, position.z, 1.0]);
            verts.extend_from_slice(&color);
            verts.push(1.0);
        }

        for triangle in self.triangles.iter() {
            for vertex in &[triangle.tri.a, triangle.tri.b, triangle.tri.c] {
                add_vert(&mut verts, *vertex, triangle.color.to_array());
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
    projection: [f32; 4 * 4],
    padding: [f32; 80 - (4 * 4 * 4)], // Need min. 80 bytes for uniforms, idk why
}

impl Primitive for ViewportPrimitive {
    fn prepare(
        &self,
        device: &shader::wgpu::Device,
        queue: &shader::wgpu::Queue,
        format: shader::wgpu::TextureFormat,
        storage: &mut shader::Storage,
        target_size: &iced::Rectangle,
        _viewport: &shader::Viewport,
    ) {
        if !storage.has::<FragmentShaderPipeline>() {
            storage.store(FragmentShaderPipeline::new(device, format));
        }

        let pipeline = storage.get_mut::<FragmentShaderPipeline>().unwrap();

        // println!("Target size: {:?}", target_size);

        let fov = self.camera.focal_length * 40.0; // rough approximation for 35mm sensor

        pipeline.update(
            queue,
            &Uniforms {
                projection: Mat4::perspective_lh(
                    fov.to_radians(),
                    target_size.width / target_size.height,
                    0.001,
                    1000.0,
                )
                .to_cols_array(),
                // center: self.controls.center,
                // scale: self.controls.scale(),
                // max_iter: self.controls.max_iter,
                padding: [0.0; 80 - (4 * 4 * 4)],
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
}
impl FragmentShaderPipeline {
    fn new(device: &shader::wgpu::Device, format: shader::wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

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
            depth_stencil: None,
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

        let size = std::mem::size_of::<Uniforms>() as u64;
        println!("Uniforms size: {}", size);
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_quad uniform buffer"),
            size: size,
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
            size: std::mem::size_of::<[f32; 4 * 2]>() as u64 * 1024 * 4, // 4k vertices with pos+color
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer,
            verts_count: 0,
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
            depth_stencil_attachment: None,
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
                            tri: tri.clone(), // TODO avoid clone
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
