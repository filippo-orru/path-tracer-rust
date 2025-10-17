use iced::{Element, widget::row};

use crate::render::{Ray, RenderConfig, intersect_scene};
use crate::{
    Message, State,
    views::viewport::viewport_render::{TriangleWithColor, ViewportPrimitive},
};
use glam::{Mat4, Vec3};
use iced::{
    Length, Point, Rectangle,
    advanced::Shell,
    event,
    keyboard::{
        Event::{KeyPressed, KeyReleased},
        Key::Named,
    },
    mouse::{self, Button},
    widget::{
        self,
        shader::{self, Event},
    },
};

pub fn viewport_tab(state: &'_ State) -> Element<'_, Message> {
    row![ViewportProgram::view(&state.render_config).map(Message::ViewportMessage),]
        .spacing(10)
        .into()
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
            config: self.config.clone(),
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
