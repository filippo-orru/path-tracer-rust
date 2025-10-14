use std::cell::RefCell;

use iced::alignment::Horizontal;
use iced::futures::channel::mpsc;
use iced::futures::channel::oneshot;
use iced::futures::SinkExt;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::stream::channel;
use iced::widget::combo_box;
use iced::widget::text_input;
use iced::widget::{button, canvas, column, container, text};
use iced::window::{Position, Settings};
use iced::Length;
use iced::{application, Color, Element, Point, Size, Subscription};
use iced::{mouse, Rectangle, Renderer, Theme};

use crate::render::gamma_correction;
use crate::render::render;
use crate::render::scenes::load_scenes;
use crate::render::Image;
use crate::render::RenderConfig;
use crate::render::RenderUpdate;
use crate::render::SceneData;

mod render;

fn main() -> iced::Result {
    let application = application("A cool counter", update, view);
    let settings = Settings {
        size: Size::new(600.0, 1027.0),
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

    scenes: Vec<SceneData>,
    scene_ids_selector: combo_box::State<String>,
    selected_scene_id: String,

    resolution_y: String,
    samples_per_pixel: String,

    config_has_error: Option<String>,

    rendering: RenderState,
}

impl Default for State {
    fn default() -> Self {
        let scenes = load_scenes();
        let selected_scene_id = scenes.get(0).unwrap().id.clone();
        Self {
            renderer_channel: None,
            scene_ids_selector: combo_box::State::with_selection(
                scenes.iter().map(|scene| scene.id.clone()).collect(),
                None, // Some(selected_scene_id.clone()).as_ref(),
            ),
            selected_scene_id,
            scenes,
            resolution_y: "300".to_owned(),
            samples_per_pixel: "1000".to_owned(),
            config_has_error: None,
            rendering: RenderState::NotRendering,
        }
    }
}

enum RenderState {
    NotRendering,
    Pending,
    Rendering { update: RenderUpdate },
    Done { result: Image },
}

#[derive(Debug, Clone)]
enum Message {
    LinkSender(mpsc::Sender<RendererInput>),
    StartRender,
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
            if let Ok(res_y) = state.resolution_y.parse::<usize>() {
                if res_y == 0 || res_y > 2000 {
                    state.config_has_error =
                        Some("Resolution Y must be between 1 and 2000".to_owned());
                    return;
                }

                if let Ok(spp) = state.samples_per_pixel.parse::<usize>() {
                    if spp == 0 || spp > 10_000 {
                        state.config_has_error =
                            Some("Samples per pixel must be between 1 and 10000".to_owned());
                        return;
                    }

                    state.rendering = RenderState::Pending;
                    if let Some(channel) = &mut state.renderer_channel {
                        let _ = channel.try_send(RendererInput::StartRendering {
                            config: RenderConfig {
                                samples_per_pixel: spp,
                                resolution_y: res_y,
                                scene: state
                                    .scenes
                                    .iter()
                                    .find(|scene| scene.id == state.selected_scene_id)
                                    .unwrap()
                                    .clone(),
                            },
                        });
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
        Message::RenderingDone(result) => {
            state.rendering = RenderState::Done { result };
        }
        Message::LinkSender(sender) => {
            state.renderer_channel = Some(sender);
        }
        Message::RenderingProgress(update) => {
            state.rendering = RenderState::Rendering { update };
        }
        Message::SelectScene(id) => state.selected_scene_id = id,
        Message::UpdateResolutionY(value) => state.resolution_y = value,
        Message::UpdateSamplesPerPixel(value) => state.samples_per_pixel = value,
    }
}

fn view(state: &State) -> Element<'_, Message> {
    container(
        column![
            text(match &state.rendering {
                RenderState::NotRendering => "Not rendering".to_owned(),
                RenderState::Pending => "Pending...".to_owned(),
                RenderState::Rendering { update } =>
                    format!("Rendering... {:.2}%", update.progress * 100.0),
                RenderState::Done { result } => format!(
                    "Render done! ({}x{})",
                    result.resolution.0, result.resolution.1
                ),
            }),
            container(
                combo_box(
                    &state.scene_ids_selector,
                    "Select scene",
                    Some(&state.selected_scene_id),
                    Message::SelectScene
                )
                .width(250)
            )
            .padding(10)
            .center_x(Length::Shrink),
            text("Resolution Y"),
            container(
                text_input("Resolution Y", &state.resolution_y.to_string())
                    .on_input(Message::UpdateResolutionY)
                    .width(250)
            )
            .padding(10)
            .center_x(Length::Shrink),
            text("Samples per pixel"),
            container(
                text_input("Samples per pixel", &state.samples_per_pixel.to_string())
                    .on_input(Message::UpdateSamplesPerPixel)
                    .width(250)
            )
            .padding(10)
            .center_x(Length::Shrink),
            if let Some(err) = &state.config_has_error {
                text(err).color(Color::from_rgb(1.0, 0.0, 0.0))
            } else {
                text("")
            },
            {
                let image = match &state.rendering {
                    RenderState::NotRendering => None,
                    RenderState::Pending => None,
                    RenderState::Rendering { update } => Some(&update.image),
                    RenderState::Done { result } => Some(result),
                };
                match image {
                    None => container(text("")),
                    Some(image) => {
                        let (width, height) = image.resolution;
                        // let aspect_ratio = width as f32 / height as f32;

                        // Use Length::Fill to allow container to expand, but keep aspect ratio
                        container(
                            canvas(CanvasState { image })
                                .width(width as f32)
                                .height(height as f32),
                        )
                        .padding(20)
                    }
                }
            },
            button("Render").on_press(Message::StartRender)
        ]
        .align_x(Horizontal::Center),
    )
    .padding(20)
    .center(Length::Fill)
    .into()
}

// Canvas
#[derive(Debug)]
struct CanvasState<'a> {
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

impl<Message> canvas::Program<Message> for CanvasState<'_> {
    type State = CanvasCache;

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(_) => (),
            canvas::Event::Touch(_) => (),
            canvas::Event::Keyboard(_) => (),
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
                    let color = self.image.pixels[(resy - y - 1) * resx + x];
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
}

fn render_worker() -> impl Stream<Item = Message> {
    channel(100, |mut output| async move {
        // Create channel
        let (sender, mut receiver) = mpsc::channel(100);

        // Send the sender back to the application
        let _ = output.send(Message::LinkSender(sender)).await;

        loop {
            // Read next input sent from `Application`
            let input = receiver.select_next_some().await;

            match input {
                RendererInput::StartRendering { config } => {
                    let (progress_sender, mut progress_receiver) =
                        mpsc::channel::<RenderUpdate>(100);
                    let (result_sender, result_receiver) = oneshot::channel();
                    std::thread::spawn(move || {
                        let mut sender = progress_sender;
                        let result = render(config, &mut sender);
                        let _ = result_sender.send(result);
                    });
                    while let Some(update) = progress_receiver.next().await {
                        let _ = output.send(Message::RenderingProgress(update)).await;
                    }
                    if let Ok(result) = result_receiver.await {
                        let _ = output.send(Message::RenderingDone(result)).await;
                    }
                }
            }
        }
    })
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::run(render_worker)
}
