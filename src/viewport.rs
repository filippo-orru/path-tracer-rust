use glam::{Mat4, Vec3};
use iced::{
    Element, Length, Point, Rectangle,
    advanced::Shell,
    event,
    keyboard::{
        Event::{KeyPressed, KeyReleased},
        Key::Named,
    },
    mouse::{self, Button},
    widget::{
        self,
        shader::{
            self, Event, Primitive,
            wgpu::{self},
        },
    },
};

use crate::render::{Ray, RenderConfig, Triangle, camera_data::CameraData, intersect_scene};

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
    cursor_move_start: Option<Point>,
    modifier_mode: ViewportModifierMode,
    orbiting_around: Option<OrbitingAround>,
}

#[derive(Default)]
enum ViewportModifierMode {
    #[default]
    Orbit,
    Zoom,
    Pan,
}

struct OrbitingAround {
    point: Vec3,
    cursor: Point, // For identifying when to reset
}

pub struct ViewportProgram<'a> {
    pub config: &'a RenderConfig,
}

impl ViewportProgram<'_> {
    pub fn view(config: &'_ RenderConfig) -> Element<'_, ViewportMessage> {
        widget::shader(ViewportProgram { config: &config })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl shader::Program<ViewportMessage> for ViewportProgram<'_> {
    type State = ViewportState;

    type Primitive = ViewportPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
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
            camera: self.config.scene.camera.clone(),
        }
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _shell: &mut Shell<'_, ViewportMessage>,
    ) -> (event::Status, Option<ViewportMessage>) {
        match event {
            Event::Keyboard(KeyPressed { key, .. }) => {
                state.modifier_mode = ViewportModifierMode::Orbit;
                if key == Named(iced::keyboard::key::Named::Super) {
                    //todoo handle non-apple
                    state.modifier_mode = ViewportModifierMode::Pan;
                }
                if key == Named(iced::keyboard::key::Named::Shift) {
                    state.modifier_mode = ViewportModifierMode::Zoom;
                }
            }
            Event::Keyboard(KeyReleased { key, modifiers, .. }) => {
                state.modifier_mode = ViewportModifierMode::Orbit;
                if key != Named(iced::keyboard::key::Named::Super) && modifiers.macos_command() {
                    state.modifier_mode = ViewportModifierMode::Zoom;
                }
                if key != Named(iced::keyboard::key::Named::Shift) && modifiers.shift() {
                    state.modifier_mode = ViewportModifierMode::Pan;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                if let Some(pos) = cursor.position()
                    && bounds.contains(pos)
                {
                    state.cursor_move_start = Some(pos);
                    return (event::Status::Captured, None);
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                state.cursor_move_start = None;
                return (
                    event::Status::Captured,
                    Some(ViewportMessage::CommitLookAround),
                );
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(orbit) = &state.orbiting_around
                    && orbit.cursor != position
                {
                    // Reset orbiting if cursor moved after the wheel event
                    state.orbiting_around = None;
                }

                if let Some(start) = state.cursor_move_start {
                    let delta = position - start;

                    const PAN_SENSITIVITY: f32 = 1.0;

                    let sensitivity = PAN_SENSITIVITY / self.config.resolution_y as f32;
                    let yaw = -delta.x * sensitivity;
                    let pitch = -delta.y * sensitivity;

                    let direction = self.config.scene.camera.direction();

                    // Yaw rotation around the up vector
                    let yaw_matrix = Mat4::from_axis_angle(Vec3::Y, yaw);
                    let new_direction = yaw_matrix.transform_vector3(direction);

                    // Pitch rotation around the right vector
                    let right = new_direction.cross(Vec3::Y).normalize();
                    let pitch_matrix = Mat4::from_axis_angle(right, pitch);
                    let final_direction = pitch_matrix.transform_vector3(new_direction).normalize();

                    return (
                        event::Status::Captured,
                        Some(ViewportMessage::LookAround(final_direction)),
                    );
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => match delta {
                mouse::ScrollDelta::Lines { .. } => todo!(),
                mouse::ScrollDelta::Pixels { x, y } => {
                    if let Some(pos) = cursor.position()
                        && bounds.contains(pos)
                    {
                        match state.modifier_mode {
                            ViewportModifierMode::Zoom => {
                                state.orbiting_around = None;

                                // Move camera forward/backward along its direction
                                let camera = &self.config.scene.camera;
                                let direction = camera.direction();
                                let position = camera.position + direction * y * 0.01;
                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::Move(position)),
                                );
                            }
                            ViewportModifierMode::Orbit => {
                                // Orbit around the look-at point
                                const SENSITIVITY: f32 = 0.0022;
                                let camera = &self.config.scene.camera;
                                let lens_center = camera.lens_center();
                                let direction = camera.direction();
                                let ray = Ray {
                                    origin: lens_center,
                                    direction: direction,
                                };
                                let orbit_center = match &state.orbiting_around {
                                    Some(center) => center.point,
                                    None => {
                                        let intersect =
                                            intersect_scene(&ray, &self.config.scene.objects);

                                        let orbit_distance = match intersect {
                                            crate::render::SceneIntersectResult::NoHit => {
                                                // Fallback to distance based on zoom
                                                lens_center.length()
                                            }
                                            crate::render::SceneIntersectResult::Hit {
                                                hit,
                                                ..
                                            } => hit.distance,
                                        };

                                        let orbit_center = lens_center + direction * orbit_distance;
                                        state.orbiting_around = Some(OrbitingAround {
                                            point: orbit_center,
                                            cursor: cursor.position().unwrap(),
                                        });
                                        orbit_center
                                    }
                                };

                                let direction = camera.position - orbit_center;
                                let orbited_direction = {
                                    let up = Vec3::Y;
                                    let yaw_matrix = Mat4::from_axis_angle(up, -x * SENSITIVITY);
                                    let with_yaw = yaw_matrix.transform_vector3(direction);

                                    let right = with_yaw.cross(Vec3::Y).normalize();
                                    let pitch_matrix =
                                        Mat4::from_axis_angle(right, y * SENSITIVITY);
                                    pitch_matrix.transform_vector3(with_yaw)
                                };
                                let position = orbit_center + orbited_direction;
                                let camera_rotation = -orbited_direction;

                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::Orbit {
                                        position,
                                        rotation: camera_rotation,
                                    }),
                                );
                            }
                            ViewportModifierMode::Pan => {
                                state.orbiting_around = None;

                                // Move camera position in the view plane

                                let camera = &self.config.scene.camera;
                                let direction = camera.direction();
                                let right = direction.cross(Vec3::Y).normalize();
                                let up = right.cross(direction).normalize();
                                let move_vec = right * -x + up * y;

                                let position = camera.position + move_vec * 0.005;
                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::Move(position)),
                                );
                            }
                        }
                    }
                }
            },
            _ => {}
        }

        (event::Status::Ignored, None)
    }
}

#[derive(Debug, Clone)]
pub enum ViewportMessage {
    LookAround(Vec3),
    CommitLookAround,
    Move(Vec3),
    Orbit { position: Vec3, rotation: Vec3 },
}
