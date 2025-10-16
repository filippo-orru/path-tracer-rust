use iced::Point;
use iced::Size;
use iced::widget::canvas;
use iced::widget::text;
use iced::{
    Alignment, Color, Element, Length, Theme, border,
    widget::{
        self,
        button::{self, Status},
        column,
        container::Style,
        row, stack, text_input,
    },
};
use iced::{Rectangle, Renderer, mouse};
use std::cell::RefCell;

use crate::render::Image;
use crate::render::Ray;
use crate::render::RenderConfig;
use crate::render::SceneIntersectResult;
use crate::render::gamma_correction;
use crate::render::intersect_scene;

use crate::{Message, RenderState, State};

pub fn render_tab(state: &'_ State) -> Element<'_, Message> {
    column![
        row![
            match &state.rendering {
                RenderState::NotRendering | RenderState::Pending | RenderState::Done(_) =>
                    widget::container(text("")),
                RenderState::Rendering { .. } =>
                    widget::container(widget::button("Stop").on_press(Message::StopRender)),
            },
            widget::button(text(match &state.rendering {
                RenderState::NotRendering | RenderState::Done(_) => "▶ Render".to_owned(),
                RenderState::Pending | RenderState::Rendering { .. } => "Rendering...".to_owned(),
            }))
            .style(|theme: &Theme, status: Status| {
                match &state.rendering {
                    RenderState::NotRendering | RenderState::Done(_) => {
                        button::primary(theme, status)
                    }
                    RenderState::Pending | RenderState::Rendering { .. } => {
                        button::secondary(theme, status)
                    }
                }
            })
            .on_press_maybe(match &state.rendering {
                RenderState::NotRendering | RenderState::Done(_) => Some(Message::StartRender),
                RenderState::Pending | RenderState::Rendering { .. } => None,
            }),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
        row![
            stack![
                {
                    let image = match &state.rendering {
                        RenderState::NotRendering | RenderState::Pending => &state.empty_image,
                        RenderState::Rendering { update, .. } => &update.image,
                        RenderState::Done(image) => &image,
                    };
                    let progress: Option<f32> = match &state.rendering {
                        RenderState::NotRendering => None,
                        RenderState::Pending => Some(0.0),
                        RenderState::Rendering { update, .. } => Some(update.progress),
                        RenderState::Done(_) => Some(1.0),
                    };
                    RenderView::view(&state.render_config, image, progress)
                },
                widget::container(text(match &state.rendering {
                    RenderState::NotRendering => "".to_owned(),
                    RenderState::Pending => "...".to_owned(),
                    RenderState::Rendering { update, stopping } =>
                        if *stopping {
                            "Stopping...".to_owned()
                        } else {
                            format!("{:.2}%", update.progress * 100.0)
                        },
                    RenderState::Done(image) =>
                        format!("Done! ({}x{})", image.resolution.0, image.resolution.1),
                }))
                .padding(10)
                .width(Length::Fill)
                .align_x(Alignment::Center),
            ],
            widget::container(
                column![
                    column![
                        column![
                            text("Resolution Y"),
                            widget::container(
                                text_input("Resolution Y", &state.resolution_y_text.to_string())
                                    .on_input(Message::UpdateResolutionY)
                                    .width(Length::Fill)
                            )
                        ]
                        .spacing(2),
                        column![
                            text("Samples per pixel"),
                            widget::container(
                                text_input("Samples per pixel", &state.samples_text.to_string())
                                    .on_input(Message::UpdateSamplesPerPixel)
                                    .width(Length::Fill)
                            )
                        ]
                        .spacing(2),
                    ]
                    .spacing(10),
                    if let Some(err) = &state.config_has_error {
                        text(err).color(Color::from_rgb(1.0, 0.0, 0.0))
                    } else {
                        text("")
                    },
                ]
                .spacing(10),
            )
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
        .spacing(10),
    ]
    .align_x(Alignment::Center)
    .spacing(10)
    .into()
}

#[derive(Debug)]
struct RenderView<'a> {
    config: &'a RenderConfig,
    image: &'a Image,
    progress: Option<f32>,
}

impl<'a> RenderView<'a> {
    fn view(
        config: &'a RenderConfig,
        image: &'a Image,
        progress: Option<f32>,
    ) -> Element<'a, Message> {
        canvas(RenderView {
            config,
            image,
            progress,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

struct CanvasCache {
    cache: canvas::Cache,
    last_hash: RefCell<u64>,
}

impl Default for CanvasCache {
    fn default() -> Self {
        Self {
            cache: canvas::Cache::new(),
            last_hash: RefCell::new(0),
        }
    }
}
fn test_scene_ray(relative_position: Point, config: &RenderConfig) {
    let sensor_origin = config.scene.camera.position;
    let lens_center = config.scene.camera.lens_center();

    let sx: f32 = 1.0 - relative_position.x * 2.0;
    let sy: f32 = relative_position.y * 2.0 - 1.0;

    let (su, sv) = config.scene.camera.orthogonals();

    // 3d sample position on sensor
    let sensor_pos = sensor_origin + su * sx + sv * sy;
    let ray_direction = (lens_center - sensor_pos).normalize();
    // ray through pinhole
    let ray = Ray {
        origin: lens_center,
        direction: ray_direction,
    };
    match intersect_scene(&ray, &config.scene.objects) {
        SceneIntersectResult::Hit { object_id, hit } => {
            println!(
                "Hit {:?} object at distance {}",
                config.scene.objects[object_id].material, hit.distance
            );
        }
        SceneIntersectResult::NoHit => {
            println!("No hit");
        }
    }
}

impl<Message> canvas::Program<Message> for RenderView<'_> {
    type State = CanvasCache;

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position().unwrap();
                if bounds.contains(position) {
                    let relative_position = Point {
                        x: (position.x - bounds.x) / bounds.width,
                        y: (position.y - bounds.y) / bounds.height,
                    };
                    // println!(
                    //     "Canvas clicked, {} {}",
                    //     relative_position.x, relative_position.y
                    // );
                    test_scene_ray(relative_position, &self.config);

                    return (canvas::event::Status::Captured, None);
                }
            }
            _ => (),
        }

        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        state: &CanvasCache,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if self.image.hash != state.last_hash.borrow().clone() {
            // println!(
            //     "Canvas hash: {} != {}",
            //     self.image.hash,
            //     state.last_hash.borrow()
            // );
            *state.last_hash.borrow_mut() = self.image.hash;
            state.cache.clear();
            // println!("Canvas data changed, clearing cache");
        }

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            // background
            frame.fill_rectangle(
                Point { x: 0.0, y: 0.0 },
                bounds.size(),
                Color::from_rgb(0.04, 0.04, 0.04),
            );

            let (resx, resy) = self.image.resolution;
            let scale_x = bounds.width / resx as f32;
            let scale_y = bounds.height / resy as f32;

            let scale = scale_x.min(scale_y);

            let offset = Point {
                x: (bounds.width - resx as f32 * scale) / 2.0,
                y: (bounds.height - resy as f32 * scale) / 2.0,
            };

            for y in 0..resy {
                for x in 0..resx {
                    let color = self.image.pixels[(resy - y) * resx - x - 1];
                    let red = gamma_correction(color.x);
                    let green = gamma_correction(color.y);
                    let blue = gamma_correction(color.z);
                    // println!("Pixel ({}, {}) = ({}, {}, {})", x, y, red, green, blue);
                    frame.fill_rectangle(
                        Point {
                            x: x as f32 * scale + offset.x,
                            y: y as f32 * scale + offset.y,
                        },
                        Size {
                            width: scale + 1.0,
                            height: scale + 1.0,
                        },
                        Color::from_rgb(red as f32, green as f32, blue as f32),
                    );
                }
            }

            // Progress bar
            if let Some(progress) = self.progress {
                let bar_height = 4.0;

                // Background
                frame.fill_rectangle(
                    Point { x: 0.0, y: 0.0 },
                    Size {
                        width: bounds.width,
                        height: bar_height,
                    },
                    Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                );

                // Foreground
                frame.fill_rectangle(
                    Point { x: 0.0, y: 0.0 },
                    Size {
                        width: bounds.width * progress,
                        height: bar_height,
                    },
                    Color::from_rgb(0.0, 0.2, 0.6),
                );
            }
        });

        vec![geometry]
    }
}
