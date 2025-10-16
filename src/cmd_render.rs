#[derive(Clone, Debug)]
enum SceneId {
    Int(usize),
    String(String),
}

impl Display for SceneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneId::Int(i) => f.write_str(&i.to_string()),
            SceneId::String(s) => f.write_str(s),
        }
    }
}

impl RenderConfig {
    fn from(args: Vec<String>) -> Option<Self> {
        return match args.len() {
            4 => {
                let scene_id_int: Option<usize> = args.get(3)?.parse().ok();
                let scene_id = match scene_id_int {
                    Some(int) => SceneId::Int(int),
                    None => SceneId::String(args.get(3)?.clone()),
                };
                let mut scenes = load_scenes().into_iter();
                let scene: SceneData = match scene_id.clone() {
                    SceneId::Int(i) => scenes.nth(i),
                    SceneId::String(s) => scenes.find(|scene| scene.id == s.as_str()),
                }
                .unwrap_or_else(|| {
                    // print_usage(&scenes);
                    exit(1);
                });
                Some(RenderConfig {
                    samples_per_pixel: args.get(1)?.parse().ok()?,
                    resolution_y: args.get(2)?.parse().ok()?,
                    scene,
                })
            }
            1 => Some(RenderConfig::default()),
            _ => None,
        };
    }
}

// fn print_usage(scenes: &Vec<SceneData>) {
//     println!(
//             "Run with:\ncargo run <samplesPerPixel = 4000> <y-resolution = 600> <scene = '{}'>\n\nScenes: {}",
//             RenderConfig::default().scene.id,
//             scenes.iter().enumerate().map(|(i, scene)| format!("{}: {}", i, scene.id)).collect::<Vec<_>>().join(", ")
//         );
// }

fn print_progress(
    processed_pixel_count: &atomic::AtomicUsize,
    grid_size: usize,
    time_start: Instant,
) {
    fn fmt(d: std::time::Duration) -> String {
        let seconds = d.as_secs() % 60;
        let minutes = (d.as_secs() / 60) % 60;
        let hours = (d.as_secs() / 60) / 60;
        if hours == 0 {
            return format!("{}m:{:0>2}s", minutes, seconds);
        }
        format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
    }
    let processed_percentage =
        processed_pixel_count.load(atomic::Ordering::Relaxed) as f64 / (grid_size) as f64;
    let elapsed = time_start.elapsed();
    print!(
        "\rRendering ... {:3.1}% ({} / {})",
        100.0 * processed_percentage,
        fmt(elapsed),
        fmt(Duration::from_secs(
            (elapsed.as_secs() as f64 * (1.0 / processed_percentage)) as u64
        ))
    );
    std::io::stdout().flush().unwrap();
}

fn load_render_config_from_args() -> RenderConfig {
    RenderConfig::from(std::env::args().collect()).unwrap()
}
