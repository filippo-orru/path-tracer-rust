use async_std::task;
use iced::Alignment;
use iced::widget::text;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;

use iced::Element;
use iced::Length;
use iced::futures;
use iced::futures::SinkExt;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;
use iced::stream::channel;
use iced::widget::combo_box;
use iced::widget::row;
use iced::widget::{button, column, container};
use iced::window::{Position, Settings};
use iced::{Point, Size, Subscription, application};

use crate::render::Image;
use crate::render::RenderConfig;
use crate::render::RenderUpdate;
use crate::render::SceneData;
use crate::render::render;
use crate::render::scenes::load_scenes;
use crate::render_tab::render_tab;
use crate::viewport::ViewportMessage;
use crate::viewport_tab::viewport_tab;

mod render;
mod render_tab;
mod viewport;
mod viewport_tab;

fn main() -> iced::Result {
    let application = application("Renderer", update, view);
    let settings = Settings {
        size: Size::new(1028.0 * 16.0 / 10.0 - 300.0, 1028.0),
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
    selected_scene: Arc<SceneData>,

    resolution_y_text: String,
    samples_text: String,

    config_has_error: Option<String>,

    render_config: RenderConfig,
    rendering: RenderState,
    empty_image: Image,

    tab: Tab,
}

impl Default for State {
    fn default() -> Self {
        let scenes = load_scenes();
        let initial_id = "mesh";
        let mesh = scenes.iter().find(|s| s.id == initial_id).unwrap().clone();
        let selected_scene = Arc::new(mesh.clone());
        Self {
            renderer_channel: None,
            scene_ids_selector: combo_box::State::with_selection(
                scenes.iter().map(|scene| scene.id.clone()).collect(),
                None, // Some(selected_scene_id.clone()).as_ref(),
            ),
            selected_scene,
            scenes,
            resolution_y_text: "300".to_owned(),
            samples_text: "100".to_owned(),
            config_has_error: None,
            render_config: RenderConfig {
                samples_per_pixel: 100,
                resolution_y: 300,
                scene: mesh.clone(),
            },
            rendering: RenderState::NotRendering,
            empty_image: Image {
                pixels: vec![],
                resolution: (0, 0),
                hash: 0,
            },
            tab: Tab::Viewport,
        }
    }
}

enum RenderState {
    NotRendering,
    Pending,
    Rendering {
        update: RenderUpdate,
        stopping: bool,
    },
    Done(Image),
}

#[derive(Debug, Clone)]
enum Message {
    StartRender,
    StopRender,
    SwitchTab(Tab),
    RenderWorkerMessage(RenderWorkerMessage),
    SelectScene(String),
    UpdateResolutionY(String),
    UpdateSamplesPerPixel(String),
    ViewportMessage(ViewportMessage),
}

#[derive(Debug, Clone)]
enum Tab {
    Viewport,
    Render,
}

fn update(state: &mut State, message: Message) {
    fn stop_render(state: &mut State) {
        if let RenderState::Rendering { update, .. } = &state.rendering {
            state.rendering = RenderState::Rendering {
                stopping: true,
                update: update.clone(),
            };
            if let Some(channel) = &mut state.renderer_channel {
                let _ = channel.try_send(RendererInput::StopRendering);
            }
        }
    }

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
        Message::StopRender => stop_render(state),
        Message::SelectScene(id) => {
            if let Some(scene) = state.scenes.iter().find(|s| s.id == id) {
                state.selected_scene = Arc::new(scene.clone());
                state.render_config.scene = scene.clone();
                state.rendering = RenderState::NotRendering;
            } else {
                state.config_has_error = Some(format!("Scene with id '{}' not found", id));
            }
        }
        Message::UpdateResolutionY(value) => {
            state.render_config.resolution_y = value.parse::<usize>().unwrap_or(300);
            state.resolution_y_text = value;
        }
        Message::UpdateSamplesPerPixel(value) => {
            state.render_config.samples_per_pixel = value.parse::<usize>().unwrap_or(100);
            state.samples_text = value;
        }
        Message::ViewportMessage(viewport_message) => {
            // println!("Viewport message: {:?}", viewport_message);
            match viewport_message {
                ViewportMessage::LookAround(direction) => {
                    state
                        .render_config
                        .scene
                        .camera
                        .set_updating_direction(direction);
                }
                ViewportMessage::CommitLookAround => {
                    state
                        .render_config
                        .scene
                        .camera
                        .set_direction(state.render_config.scene.camera.get_current_direction());
                }
                ViewportMessage::Move(position) => {
                    state.render_config.scene.camera.position = position;
                }
                ViewportMessage::Orbit { position, rotation } => {
                    state.render_config.scene.camera.position = position;
                    state.render_config.scene.camera.set_direction(rotation);
                }
            }
        }
        Message::RenderWorkerMessage(render_msg) => match render_msg {
            RenderWorkerMessage::RenderingDone(image) => state.rendering = RenderState::Done(image),
            RenderWorkerMessage::LinkSender(sender) => state.renderer_channel = Some(sender),
            RenderWorkerMessage::RenderingProgress(update) => {
                state.rendering = RenderState::Rendering {
                    update,
                    stopping: false,
                }
            }
        },
        Message::SwitchTab(tab) => state.tab = tab,
    };
}

fn view(state: &State) -> Element<'_, Message> {
    return container(
        column![
            row![
                row![
                    button("Viewport")
                        .style(if matches!(state.tab, Tab::Viewport) {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .padding([2, 4])
                        .on_press(Message::SwitchTab(Tab::Viewport)),
                    button("Render")
                        .style(if matches!(state.tab, Tab::Render) {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .padding([2, 4])
                        .on_press(Message::SwitchTab(Tab::Render)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                container(
                    row![
                        text("Scene"),
                        combo_box(
                            &state.scene_ids_selector,
                            "Select scene",
                            Some(&state.selected_scene.id),
                            Message::SelectScene
                        )
                        .width(120),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(4)
                )
                .width(Length::Fill),
            ]
            .spacing(24)
            .align_y(Alignment::Center)
            .padding([2, 4]),
            container(match state.tab {
                Tab::Viewport => {
                    viewport_tab(state)
                }
                Tab::Render => {
                    render_tab(state)
                }
            },)
            .width(Length::Fill)
            .height(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(6)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
}

enum RendererInput {
    StartRendering { config: RenderConfig },
    StopRendering,
}

struct ActiveRender {
    cancel_render: Arc<AtomicBool>,
    handles: Vec<task::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
enum RenderWorkerMessage {
    LinkSender(mpsc::Sender<RendererInput>),
    RenderingProgress(RenderUpdate),
    RenderingDone(Image),
}

fn render_worker() -> impl Stream<Item = RenderWorkerMessage> {
    channel(100, |mut output| async move {
        // Create channel
        let (sender, mut receiver) = mpsc::channel(100);

        // Send the sender back to the application
        let _ = output.send(RenderWorkerMessage::LinkSender(sender)).await;

        let mut active_render: Option<ActiveRender> = None;

        loop {
            // Read next input sent from `Application`
            let input = receiver.select_next_some().await;

            match input {
                RendererInput::StartRendering { config } => {
                    let (progress_sender, mut progress_receiver) =
                        mpsc::channel::<RenderUpdate>(100);
                    let cancel_render_arc = Arc::new(AtomicBool::new(false));

                    let mut output_clone = output.clone();
                    let cancel_render = cancel_render_arc.clone();
                    let render_handle = task::spawn(async move {
                        let mut sender = progress_sender;
                        let result = render(config, &mut sender, cancel_render.clone());
                        let _ = output_clone
                            .send(RenderWorkerMessage::RenderingDone(result))
                            .await;
                    });
                    let mut output_clone = output.clone();
                    let forward_updates_handle = task::spawn(async move {
                        // Forward updates to the main output
                        while let Some(update) = progress_receiver.next().await {
                            let _ = output_clone
                                .send(RenderWorkerMessage::RenderingProgress(update))
                                .await;
                        }
                    });

                    let cancel_render = cancel_render_arc.clone();
                    active_render = Some(ActiveRender {
                        cancel_render,
                        handles: vec![render_handle, forward_updates_handle],
                    });
                }
                RendererInput::StopRendering => {
                    // Handle stop rendering
                    if let Some(render) = active_render.take() {
                        render.cancel_render.store(true, atomic::Ordering::Relaxed);
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
    Subscription::run(render_worker).map(|message| Message::RenderWorkerMessage(message))
}
