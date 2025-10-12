use iced::alignment::Horizontal;
use iced::futures::channel::mpsc;
use iced::futures::channel::mpsc::SendError;
use iced::futures::future;
use iced::futures::SinkExt;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::stream::channel;
use iced::widget::combo_box;
use iced::widget::{button, canvas, column, container, text};
use iced::window::{Position, Settings};
use iced::Length::Fill;
use iced::{application, Color, Element, Point, Size, Subscription};
use iced::{mouse, Rectangle, Renderer, Theme};

use crate::render::gamma_correction;
use crate::render::render;
use crate::render::RenderConfig;
use crate::render::RenderResult;
use crate::render::RenderUpdate;

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
    rendering: RenderState,
}

enum RenderState {
    NotRendering,
    Rendering { progress: f64 },
    Done { result: RenderResult },
}

#[derive(Debug, Clone)]
enum Message {
    LinkSender(mpsc::Sender<RendererInput>),
    StartRender,
    RenderingProgress(f64),
    RenderingDone(RenderResult),
}

impl Default for State {
    fn default() -> Self {
        Self {
            renderer_channel: None,
            rendering: RenderState::NotRendering,
        }
    }
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::StartRender => {
            state.rendering = RenderState::Rendering { progress: 0.0 };
            if let Some(channel) = &mut state.renderer_channel {
                let _ = channel.try_send(RendererInput::StartRendering);
            }
        }
        Message::RenderingDone(result) => {
            state.rendering = RenderState::Done { result };
        }
        Message::LinkSender(sender) => {
            state.renderer_channel = Some(sender);
        }
        Message::RenderingProgress(progress) => {
            state.rendering = RenderState::Rendering { progress };
        }
    }
}

fn view(state: &State) -> Element<'_, Message> {
    container(
        column![
            text(match &state.rendering {
                RenderState::NotRendering => "Not rendering".to_owned(),
                RenderState::Rendering { progress } =>
                    format!("Rendering... {:.2}%", progress * 100.0),
                RenderState::Done { result } => format!(
                    "Render done! ({}x{})",
                    result.resolution.0, result.resolution.1
                ),
            }),
            // combo_box(),
            match &state.rendering {
                RenderState::NotRendering => container(text("")),
                RenderState::Rendering { progress: _ } => container(text("")),
                RenderState::Done { result } => container(
                    canvas(CanvasState {
                        result: Some(result),
                    })
                    .width(result.resolution.0 as f32)
                    .height(result.resolution.1 as f32)
                )
                .padding(20),
            },
            button("Render").on_press(Message::StartRender)
        ]
        .align_x(Horizontal::Center),
    )
    .center(Fill)
    .into()
}

// Canvas
// First, we define the data we need for drawing
#[derive(Debug)]
struct CanvasState<'a> {
    result: Option<&'a RenderResult>,
}

struct CanvasCache {
    cache: canvas::Cache,
}

impl Default for CanvasCache {
    fn default() -> Self {
        Self {
            cache: canvas::Cache::new(),
        }
    }
}

// Then, we implement the `Program` trait
impl<Message> canvas::Program<Message> for CanvasState<'_> {
    // No internal state
    type State = CanvasCache;

    fn draw(
        &self,
        _state: &CanvasCache,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = _state.cache.draw(renderer, bounds.size(), |frame| {
            // First, we draw the background
            frame.fill_rectangle(
                Point { x: 0.0, y: 0.0 },
                bounds.size(),
                Color::from_rgb(0.0, 0.0, 0.0),
            );

            if let Some(result) = self.result {
                let (resx, resy) = result.resolution;
                let scale_x = bounds.width / resx as f32;
                let scale_y = bounds.height / resy as f32;

                for y in 0..resy {
                    for x in 0..resx {
                        let color = result.pixels[y * resx + x];
                        let red = gamma_correction(color.x);
                        let green = gamma_correction(color.y);
                        let blue = gamma_correction(color.z);
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
            } else {
                // Background
                frame.fill_rectangle(
                    Point { x: 0.0, y: 0.0 },
                    bounds.size(),
                    Color::from_rgb(0.1, 0.1, 0.1),
                );
            }
        });

        vec![geometry]
    }
}

enum RendererInput {
    StartRendering,
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
                RendererInput::StartRendering => {
                    // let (render_sender, mut render_receiver) = mpsc::channel::<RenderUpdate>(100);
                    let render_info_sender = output.clone().with(|render_update: RenderUpdate| {
                        future::ready(Ok::<Message, SendError>(Message::RenderingProgress(
                            render_update.progress,
                        )))
                    });
                    let result = render(RenderConfig::default(), render_info_sender);
                    let _ = output.send(Message::RenderingDone(result)).await;
                }
            }
        }
    })
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::run(render_worker)
}
