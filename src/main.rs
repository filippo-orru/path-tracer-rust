use async_std::task;
use iced::futures::future::JoinAll;
use std::cell::RefCell;
use std::thread;

use iced::Length;
use iced::alignment::Horizontal;
use iced::futures;
use iced::futures::SinkExt;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;
use iced::futures::channel::oneshot;
use iced::futures::channel::oneshot::Sender;
use iced::stream::channel;
use iced::widget::combo_box;
use iced::widget::row;
use iced::widget::shader;
use iced::widget::text_input;
use iced::widget::{button, canvas, column, container, text};
use iced::window::{Position, Settings};
use iced::{Color, Element, Point, Size, Subscription, application};
use iced::{Rectangle, Renderer, Theme, mouse};

use crate::render::Image;
use crate::render::Ray;
use crate::render::RenderConfig;
use crate::render::RenderUpdate;
use crate::render::SceneData;
use crate::render::SceneIntersectResult;
use crate::render::gamma_correction;
use crate::render::intersect_scene;
use crate::render::render;
use crate::render::scenes::load_scenes;
use crate::viewport::ViewportProgram;

mod render;
mod viewport;

fn main() -> iced::Result {
    let application = application("A cool counter", update, view);
    let settings = Settings {
        size: Size::new(950.0, 1027.0),
        position: Position::SpecificWith(|window, display| Point {
            x: display.width - window.width,
            y: 0.0,
        }),
        ..Default::default()
    };
    application
        .subscription(subscription)
        .window(settings)
        .run()
}

struct State {
    renderer_channel: Option<mpsc::Sender<RendererInput>>,

    scenes: Vec<RefCell<SceneData>>,
    scene_ids_selector: combo_box::State<String>,
    selected_scene: RefCell<SceneData>,

    resolution_y_text: String,
    samples_text: String,

    config_has_error: Option<String>,

    render_config: RenderConfig,
    rendering: RenderState,
    empty_image: Image,
}

impl Default for State {
    fn default() -> Self {
        let scenes = load_scenes()
            .into_iter()
            .map(RefCell::new)
            .collect::<Vec<_>>();
        let initial_id = "mesh";
        let mesh = scenes
            .iter()
            .find(|s| s.borrow().id == initial_id)
            .unwrap()
            .clone();
        Self {
            renderer_channel: None,
            scene_ids_selector: combo_box::State::with_selection(
                scenes
                    .iter()
                    .map(|scene| (*scene.borrow()).id.clone())
                    .collect(),
                None, // Some(selected_scene_id.clone()).as_ref(),
            ),
            selected_scene: mesh.clone(),
            scenes,
            resolution_y_text: "300".to_owned(),
            samples_text: "100".to_owned(),
            config_has_error: None,
            render_config: RenderConfig {
                samples_per_pixel: 100,
                resolution_y: 300,
                scene: mesh.borrow().clone(),
            },
            rendering: RenderState::NotRendering,
            empty_image: Image {
                pixels: vec![],
                resolution: (0, 0),
                hash: 0,
            },
        }
    }
}

enum RenderState {
    NotRendering,
    Pending,
    Rendering { update: RenderUpdate },
    Done { result: Image },
    Stopping,
}

#[derive(Debug, Clone)]
enum Message {
    LinkSender(mpsc::Sender<RendererInput>),
    StartRender,
    StopRender,
    RenderingProgress(RenderUpdate),
    RenderingDone(Image),
    SelectScene(String),
    UpdateResolutionY(String),
    UpdateSamplesPerPixel(String),
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::StartRender => {
            if let RenderState::Rendering { .. } = state.rendering {
                return;
            }
            if let Ok(res_y) = state.resolution_y_text.parse::<usize>() {
                if res_y == 0 || res_y > 2000 {
                    state.config_has_error =
                        Some("Resolution Y must be between 1 and 2000".to_owned());
                    return;
                }

                if let Ok(spp) = state.samples_text.parse::<usize>() {
                    if spp == 0 || spp > 10_000 {
                        state.config_has_error =
                            Some("Samples per pixel must be between 1 and 10000".to_owned());
                        return;
                    }

                    state.rendering = RenderState::Pending;
                    let config = state.render_config.clone();
                    if let Some(channel) = &mut state.renderer_channel {
                        let _ = channel.try_send(RendererInput::StartRendering { config });
                    }
                } else {
                    state.config_has_error = Some("Samples per pixel must be a number".to_owned());
                    return;
                }
            } else {
                state.config_has_error = Some("Resolution Y must be a number".to_owned());
                return;
            }
        }
        Message::StopRender => {
            if let RenderState::NotRendering | RenderState::Done { .. } = state.rendering {
                return;
            }
            state.rendering = RenderState::Stopping;
            if let Some(channel) = &mut state.renderer_channel {
                let _ = channel.try_send(RendererInput::StopRendering);
            }
        }
        Message::RenderingDone(result) => {
            state.rendering = RenderState::Done { result };
        }
        Message::LinkSender(sender) => {
            state.renderer_channel = Some(sender);
        }
        Message::RenderingProgress(update) => {
            state.rendering = RenderState::Rendering { update };
        }
        Message::SelectScene(id) => {
            if let Some(scene) = state.scenes.iter().find(|s| s.borrow().id == id) {
                state.selected_scene = scene.clone();
                state.rendering = RenderState::NotRendering;
            } else {
                state.config_has_error = Some(format!("Scene with id '{}' not found", id));
            }
        }
        Message::UpdateResolutionY(value) => state.resolution_y_text = value,
        Message::UpdateSamplesPerPixel(value) => state.samples_text = value,
    };
    state.render_config = RenderConfig {
        samples_per_pixel: state.samples_text.parse::<usize>().unwrap_or(100),
        resolution_y: state.resolution_y_text.parse::<usize>().unwrap_or(300),
        scene: state.selected_scene.borrow().clone(),
    }
}

fn view(state: &State) -> Element<'_, Message> {
    return container(
        column![
            column![
                row![
                    column![
                        text("Scene"),
                        container(
                            combo_box(
                                &state.scene_ids_selector,
                                "Select scene",
                                Some(&state.selected_scene.borrow().id),
                                Message::SelectScene
                            )
                            .width(250)
                        )
                    ]
                    .spacing(2),
                    column![
                        text("Resolution Y"),
                        container(
                            text_input("Resolution Y", &state.resolution_y_text.to_string())
                                .on_input(Message::UpdateResolutionY)
                                .width(250)
                        )
                    ]
                    .spacing(2),
                    column![
                        text("Samples per pixel"),
                        container(
                            text_input("Samples per pixel", &state.samples_text.to_string())
                                .on_input(Message::UpdateSamplesPerPixel)
                                .width(250)
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
            shader(ViewportProgram {
                config: &state.render_config,
                // controls: Controls::default()
            })
            .width(state.render_config.resolution_x() as f32)
            .height(state.render_config.resolution_y as f32),
            {
                let image = match &state.rendering {
                    RenderState::NotRendering | RenderState::Pending | RenderState::Stopping => {
                        &state.empty_image
                    }
                    RenderState::Rendering { update } => &update.image,
                    RenderState::Done { result } => &result,
                };
                // let (width, height) = image.resolution;
                // let aspect_ratio = width as f32 / height as f32;
                container(
                    canvas(CanvasState {
                        image,
                        config: &state.render_config,
                    })
                    .width(state.render_config.resolution_x() as f32)
                    .height(state.render_config.resolution_y as f32),
                )
            },
            row![
                button("Stop").on_press_maybe(match &state.rendering {
                    RenderState::NotRendering
                    | RenderState::Pending
                    | RenderState::Done { result: _ }
                    | RenderState::Stopping => None,
                    RenderState::Rendering { update: _ } => Some(Message::StopRender),
                }),
                button(text(match &state.rendering {
                    RenderState::NotRendering | RenderState::Done { result: _ } =>
                        "Render".to_owned(),
                    RenderState::Pending
                    | RenderState::Rendering { update: _ }
                    | RenderState::Stopping => "Rendering...".to_owned(),
                }))
                .on_press_maybe(match &state.rendering {
                    RenderState::NotRendering | RenderState::Done { result: _ } =>
                        Some(Message::StartRender),
                    RenderState::Pending
                    | RenderState::Rendering { update: _ }
                    | RenderState::Stopping => None,
                }),
                text(match &state.rendering {
                    RenderState::NotRendering => "".to_owned(),
                    RenderState::Pending => "...".to_owned(),
                    RenderState::Stopping => "Stopping...".to_owned(),
                    RenderState::Rendering { update } => format!("{:.2}%", update.progress * 100.0),
                    RenderState::Done { result } =>
                        format!("Done! ({}x{})", result.resolution.0, result.resolution.1),
                }),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10),
        ]
        .spacing(10)
        .align_x(Horizontal::Center),
    )
    .padding(20)
    .center(Length::Fill)
    .into();
}

// Canvas
#[derive(Debug)]
struct CanvasState<'a> {
    config: &'a RenderConfig,
    image: &'a Image,
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

impl<Message> canvas::Program<Message> for CanvasState<'_> {
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
                Color::from_rgb(0.0, 0.0, 0.0),
            );

            let (resx, resy) = self.image.resolution;
            let scale_x = bounds.width / resx as f32;
            let scale_y = bounds.height / resy as f32;

            for y in 0..resy {
                for x in 0..resx {
                    let color = self.image.pixels[(resy - y) * resx - x - 1];
                    let red = gamma_correction(color.x);
                    let green = gamma_correction(color.y);
                    let blue = gamma_correction(color.z);
                    // println!("Pixel ({}, {}) = ({}, {}, {})", x, y, red, green, blue);
                    frame.fill_rectangle(
                        Point {
                            x: x as f32 * scale_x,
                            y: y as f32 * scale_y,
                        },
                        Size {
                            width: scale_x + 1.0,
                            height: scale_y + 1.0,
                        },
                        Color::from_rgb(red as f32, green as f32, blue as f32),
                    );
                }
            }
        });

        vec![geometry]
    }
}

enum RendererInput {
    StartRendering { config: RenderConfig },
    StopRendering,
}

struct ActiveRender {
    stop_sender: Sender<()>,
    handles: Vec<task::JoinHandle<()>>,
}

fn render_worker() -> impl Stream<Item = Message> {
    channel(100, |mut output| async move {
        // Create channel
        let (sender, mut receiver) = mpsc::channel(100);

        // Send the sender back to the application
        let _ = output.send(Message::LinkSender(sender)).await;

        let mut active_render: Option<ActiveRender> = None;

        loop {
            // Read next input sent from `Application`
            let input = receiver.select_next_some().await;

            match input {
                RendererInput::StartRendering { config } => {
                    let (progress_sender, mut progress_receiver) =
                        mpsc::channel::<RenderUpdate>(100);
                    let (stop_sender, stop_receiver) = oneshot::channel();

                    let mut output_clone = output.clone();
                    let render_handle = task::spawn(async move {
                        let mut sender = progress_sender;
                        let result = render(config, &mut sender, stop_receiver);
                        let _ = output_clone.send(Message::RenderingDone(result)).await;
                    });
                    let mut output_clone = output.clone();
                    let forward_updates_handle = task::spawn(async move {
                        // Forward updates to the main output
                        while let Some(update) = progress_receiver.next().await {
                            let _ = output_clone.send(Message::RenderingProgress(update)).await;
                        }
                    });
                    active_render = Some(ActiveRender {
                        stop_sender,
                        handles: vec![render_handle, forward_updates_handle],
                    });
                }
                RendererInput::StopRendering => {
                    // Handle stop rendering
                    if let Some(render) = active_render.take() {
                        let _ = render.stop_sender.send(());
                        futures::future::join_all(render.handles).await;
                    } else {
                        println!("No active render to stop.");
                    }
                }
            }
        }
    })
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::run(render_worker)
}
