use iced::widget::container::Style;
use iced::{Color, Theme, border, keyboard};
use iced::{Element, widget::row};

use crate::render::{Hit, Ray, SceneData, SceneObjectData, intersect_scene};
use crate::{Message, State, views::viewport::viewport_render::ViewportPrimitive};
use glam::{Mat4, Vec3};
use iced::{
    Length, Point, Rectangle,
    advanced::Shell,
    event,
    mouse::{self, Button},
    widget::{
        self, column, container,
        shader::{self, Event},
    },
};

pub fn viewport_tab(state: &'_ State) -> Element<'_, Message> {
    row![ViewportProgram::view(&state.scene, &state.viewport_state).map(Message::ViewportMessage),]
        .spacing(10)
        .into()
}

#[derive(Clone, Debug)]
pub enum ViewportModifierMode {
    DefaultOrbit { orbit: Option<OrbitingAround> },
    Zoom,
    Pan,
    LookAround,
}

#[derive(Debug, Clone)]
pub struct OrbitingAround {
    point: Vec3,
    cursor: Point, // For identifying when to reset
}

impl OrbitingAround {
    pub fn new(scene: &SceneData, cursor: Point) -> Self {
        let lens_center = scene.camera.lens_center();
        let direction = scene.camera.direction();
        let ray = Ray {
            origin: lens_center,
            direction: direction,
        };
        let intersect = get_orbit_point(&ray, &scene.objects);

        let point = match intersect {
            None => lens_center + direction * lens_center.length(), // Fallback to distance based on zoom
            Some(hit_position) => hit_position,
        };

        return OrbitingAround { point, cursor };
    }
}

pub struct ViewportProgram<'a> {
    pub scene: &'a SceneData,
    pub viewport_state: &'a ViewportState,
}

#[derive(Debug, Clone)]
pub struct ViewportState {
    pub selected_object: Option<usize>,
    modifier_mode: ViewportModifierMode,
}

impl ViewportState {
    pub fn new() -> Self {
        ViewportState {
            selected_object: None,
            modifier_mode: ViewportModifierMode::DefaultOrbit { orbit: None },
        }
    }

    pub fn update(&mut self, message: &ViewportStateMessage) {
        match message {
            ViewportStateMessage::SelectObject { id } => {
                self.selected_object = *id;
            }
            ViewportStateMessage::SetModifierMode(viewport_modifier_mode) => {
                self.modifier_mode = viewport_modifier_mode.clone();
            }
        }
    }

    pub fn update_orbit(&mut self, orbit: OrbitingAround) {
        self.modifier_mode = ViewportModifierMode::DefaultOrbit { orbit: Some(orbit) };
    }
}

impl ViewportProgram<'_> {
    pub fn view<'a>(
        scene: &'a SceneData,
        viewport_state: &'a ViewportState,
    ) -> Element<'a, ViewportMessage> {
        row![
            column![
                widget::shader(ViewportProgram {
                    scene,
                    viewport_state,
                })
                .width(Length::Fill)
                .height(Length::Fill),
                container(match viewport_state.modifier_mode {
                    ViewportModifierMode::DefaultOrbit { .. } => widget::text("Orbiting"),
                    ViewportModifierMode::Zoom => widget::text("Zooming"),
                    ViewportModifierMode::Pan => widget::text("Panning"),
                    ViewportModifierMode::LookAround => widget::text("Looking Around"),
                },)
            ]
            .spacing(3),
            // Sidebar
            container(column![
                column(
                    scene
                        .objects
                        .iter()
                        .enumerate()
                        .map(|(index, object)| {
                            container(widget::text(format!(
                                "{} {}",
                                index,
                                match object.type_ {
                                    crate::render::SceneObject::Sphere { .. } => "Sphere",
                                    crate::render::SceneObject::Mesh { .. } => "Mesh",
                                }
                            )))
                            .style(if viewport_state.selected_object == Some(index) {
                                |theme: &Theme| Style {
                                    background: Some(theme.palette().primary.into()),
                                    text_color: Some(Color::WHITE),
                                    ..Default::default()
                                }
                            } else {
                                |_: &Theme| Style::default()
                            })
                            .into()
                        })
                        .collect::<Vec<_>>()
                )
                .spacing(10),
                // Divider
                container(widget::text("")).style(|theme: &Theme| Style {
                    border: border::color(Color {
                        a: 0.5,
                        ..theme.palette().text
                    })
                    .width(1)
                    .rounded(0),
                    ..Default::default()
                }),
                column![widget::text(format!(
                    "Selected Object: {}",
                    match viewport_state.selected_object {
                        Some(id) => id.to_string(),
                        None => "None".to_string(),
                    }
                )),]
                .spacing(10),
            ],)
            .style(|theme: &Theme| Style {
                border: border::color(Color {
                    a: 0.5,
                    ..theme.palette().text
                })
                .width(1)
                .rounded(5),
                ..Default::default()
            })
            .padding(10)
            .width(250)
            .height(Length::Fill),
        ]
        .spacing(10)
        .into()
    }
}

impl shader::Program<ViewportMessage> for ViewportProgram<'_> {
    type State = ();

    type Primitive = ViewportPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        ViewportPrimitive {
            scene: self.scene.clone(),
            viewport_state: self.viewport_state.clone(),
        }
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _shell: &mut Shell<'_, ViewportMessage>,
    ) -> (event::Status, Option<ViewportMessage>) {
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                let modifier_mode = if modifiers.shift() {
                    if modifiers.macos_command() {
                        ViewportModifierMode::LookAround
                    } else {
                        ViewportModifierMode::Zoom
                    }
                } else if modifiers.macos_command() {
                    ViewportModifierMode::Pan
                } else {
                    ViewportModifierMode::DefaultOrbit { orbit: None }
                };
                return (
                    event::Status::Captured,
                    Some(ViewportMessage::StateMessage(
                        ViewportStateMessage::SetModifierMode(modifier_mode),
                    )),
                );
            }
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                if let Some(pos) = cursor.position()
                    && bounds.contains(pos)
                {
                    // state.cursor_move_start = Some(pos);
                    let camera = &self.scene.camera;
                    let aspect_ratio = bounds.width as f32 / bounds.height as f32;
                    let view_proj = camera.get_view_projection(aspect_ratio);

                    let x_adj = (pos.x - bounds.x) / bounds.width * 2.0 - 1.0;
                    let y_adj = (bounds.height - pos.y + bounds.y) / bounds.height * 2.0 - 1.0;
                    let screen_space_vec = Vec3::new(x_adj, y_adj, 1.0);
                    let inv_view_proj = view_proj.inverse();
                    let world_space_vec = inv_view_proj.project_point3(screen_space_vec);
                    let ray = Ray {
                        origin: camera.lens_center(),
                        direction: (world_space_vec - camera.position).normalize(),
                    };
                    let msg = ViewportMessage::StateMessage(ViewportStateMessage::SelectObject {
                        id: intersect_scene(&ray, &self.scene.objects).map(|hit| hit.object_id),
                    });
                    return (event::Status::Captured, Some(msg));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                // state.cursor_move_start = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let ViewportModifierMode::DefaultOrbit { orbit: Some(orbit) } =
                    &self.viewport_state.modifier_mode
                    && orbit.cursor != position
                {
                    // Reset orbiting if cursor moved after the wheel event
                    return (
                        event::Status::Captured,
                        Some(ViewportMessage::StateMessage(
                            ViewportStateMessage::SetModifierMode(
                                ViewportModifierMode::DefaultOrbit { orbit: None },
                            ),
                        )),
                    );
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => match delta {
                mouse::ScrollDelta::Lines { .. } => todo!(),
                mouse::ScrollDelta::Pixels { x, y } => {
                    if let Some(position) = cursor.position()
                        && bounds.contains(position)
                    {
                        match &self.viewport_state.modifier_mode {
                            ViewportModifierMode::Zoom => {
                                // Move camera forward/backward along its direction
                                let camera = &self.scene.camera;
                                let direction = camera.direction();
                                let magnitude = camera.position.length() * 0.002;
                                let position = camera.position + direction * y * magnitude;
                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::Move(position)),
                                );
                            }
                            ViewportModifierMode::DefaultOrbit { orbit } => {
                                // Orbit around the look-at point
                                const SENSITIVITY: f32 = 0.0018;
                                let camera = &self.scene.camera;

                                let mut update_orbit = None;

                                let orbit = match orbit {
                                    Some(orbit) => orbit.clone(),
                                    None => {
                                        let new_orbit = OrbitingAround::new(
                                            self.scene,
                                            cursor.position().unwrap(),
                                        );
                                        update_orbit = Some(new_orbit.clone());
                                        new_orbit
                                    }
                                };
                                let direction = camera.position - orbit.point;
                                let orbited_direction = {
                                    let up = Vec3::Y;
                                    let yaw_matrix = Mat4::from_axis_angle(up, -x * SENSITIVITY);
                                    let with_yaw = yaw_matrix.transform_vector3(direction);

                                    let right = with_yaw.cross(Vec3::Y).normalize();
                                    let pitch_matrix =
                                        Mat4::from_axis_angle(right, y * SENSITIVITY);
                                    pitch_matrix.transform_vector3(with_yaw)
                                };
                                let position = orbit.point + orbited_direction;
                                let camera_rotation = -orbited_direction;

                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::Orbit {
                                        position,
                                        rotation: camera_rotation,
                                        update_orbit,
                                    }),
                                );
                            }
                            ViewportModifierMode::Pan => {
                                // Move camera position in the view plane

                                let camera = &self.scene.camera;
                                let direction = camera.direction();
                                let right = direction.cross(Vec3::Y).normalize();
                                let up = right.cross(direction).normalize();
                                let move_vec = right * -x + up * y;

                                let magnitude = camera.position.length() * 0.0002;
                                let position = camera.position + move_vec * magnitude;
                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::Move(position)),
                                );
                            }
                            ViewportModifierMode::LookAround => {
                                const LOOK_AROUND_SENSITIVITY: f32 = 1.0;

                                let sensitivity = LOOK_AROUND_SENSITIVITY / bounds.height as f32;
                                let yaw = -x * sensitivity;
                                let pitch = -y * sensitivity;

                                let direction = self.scene.camera.direction();

                                // Yaw rotation around the up vector
                                let yaw_matrix = Mat4::from_axis_angle(Vec3::Y, yaw);
                                let new_direction = yaw_matrix.transform_vector3(direction);

                                // Pitch rotation around the right vector
                                let right = new_direction.cross(Vec3::Y).normalize();
                                let pitch_matrix = Mat4::from_axis_angle(right, pitch);
                                let final_direction =
                                    pitch_matrix.transform_vector3(new_direction).normalize();

                                return (
                                    event::Status::Captured,
                                    Some(ViewportMessage::LookAround(final_direction)),
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
    Move(Vec3),
    Orbit {
        position: Vec3,
        rotation: Vec3,
        update_orbit: Option<OrbitingAround>,
    },
    StateMessage(ViewportStateMessage),
}

#[derive(Debug, Clone)]
pub enum ViewportStateMessage {
    SelectObject { id: Option<usize> },
    SetModifierMode(ViewportModifierMode),
}

/// Gets point around which to orbit by finding the closest object by its bounding box,
///  and then checking for the actual intersection.
/// This is done, so a mesh in the screen center can be orbited around, even if there is
///  no triangle exactly in the center of the screen.
fn get_orbit_point(ray: &Ray, scene_objects: &Vec<SceneObjectData>) -> Option<Vec3> {
    let mut min_intersect: Option<Hit> = None;

    for i in (0..scene_objects.len()).rev() {
        let scene_object = &scene_objects[i];
        let intersect_bounds = scene_object.intersect_bounds(ray);
        match intersect_bounds {
            None => (),
            Some(hit_bounds) => {
                let hit_object = scene_object.intersect(ray);
                // Use the actual object hit if available, otherwise use the bounding box hit
                let new_hit = match hit_object {
                    Some(hit) => hit,
                    None => hit_bounds,
                };

                match &min_intersect {
                    Some(hit) => {
                        if new_hit.distance < hit.distance {
                            min_intersect = Some(new_hit);
                        }
                    }
                    None => {
                        min_intersect = Some(new_hit);
                    }
                }
            }
        }
    }
    return min_intersect.map(|hit| hit.intersection);
}
