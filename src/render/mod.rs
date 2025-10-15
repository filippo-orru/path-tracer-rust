mod load_off;
pub mod scenes;

#[cfg(test)]
mod test;

use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{Hash, Hasher},
    io::Write,
    process::exit,
    sync::{atomic, mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

use glam::Vec3;
use iced::futures::{self, channel::mpsc::SendError, Sink, SinkExt};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use scenes::load_scenes;

const USE_CULLING: bool = false;
const PI: f32 = 3.141592653589793;

/// If true, render with a fixed sequence of random numbers.
const MOCK_RANDOM: bool = false;
const MOCK_RANDOMS: [f32; 9] = [
    0.75902418061906407,
    0.023879213030728041,
    0.21016190197770457,
    0.78814922184253244,
    0.56819568237964491,
    0.7689823904006352,
    0.16910304067812287,
    0.54519597695203492,
    0.63614169009490062,
];
const MOCK_RANDOMS_LEN: usize = MOCK_RANDOMS.len();
static MOCK_RANDOMS_INDEX: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

// uniform double random generator function
fn rand01() -> f32 {
    if MOCK_RANDOM {
        let i = MOCK_RANDOMS_INDEX.fetch_add(1, atomic::Ordering::Relaxed) % MOCK_RANDOMS_LEN;
        return MOCK_RANDOMS[i];
    } else {
        return rand::random::<f32>();
    }
}

pub fn gamma_correction(x: f32) -> f32 {
    return x.clamp(0.0, 1.0).powf(1.0 / 2.2);
}

pub fn to_int_with_gamma_correction(x: f32) -> usize {
    return (255.0 * gamma_correction(x) + 0.5) as usize;
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
}

#[derive(Clone, Debug)]
pub enum ReflectType {
    Diffuse,
    Specular,
    Refract,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub color: Vec3,
    pub emmission: Vec3,
    pub reflect_type: ReflectType,
}

#[derive(Clone, Debug)]
pub struct SceneData {
    pub id: String,
    pub objects: Vec<SceneObjectData>,
    pub camera: CameraData,
}

#[derive(Clone, Copy, Debug)]
pub struct CameraData {
    pub position: Vec3,
    /// normal to sensor plane
    pub direction: Vec3,
    /// in meters
    pub focal_length: f32,
}

#[derive(Clone, Debug)]
pub struct SceneObjectData {
    pub type_: SceneObject,
    pub position: Vec3,
    pub material: Material,
}

impl SceneObjectData {
    fn intersect(&self, ray: &Ray) -> IntersectResult {
        return match &self.type_ {
            SceneObject::Sphere { radius } => intersect_sphere(self.position, *radius, ray),

            SceneObject::Mesh(mesh) => match intersect_sphere(
                mesh.bounding_sphere.position + self.position,
                mesh.bounding_sphere.radius,
                ray,
            ) {
                IntersectResult::NoHit => IntersectResult::NoHit,
                IntersectResult::Hit(_) => {
                    for original_tri in mesh.triangles.iter() {
                        let tri = original_tri.transformed(&self.position);
                        let va_vb = tri.b - tri.a;
                        let va_vc = tri.c - tri.a;

                        let pvec = ray.direction.cross(va_vc);
                        let determinant = va_vb.dot(pvec);

                        if USE_CULLING {
                            if determinant < 1e-4 {
                                continue;
                            }
                        } else {
                            if determinant.abs() < 1e-4 {
                                continue;
                            }
                        }

                        let inv_determinant = 1.0 / determinant;
                        let tvec = ray.origin - tri.a;
                        let u: f32 = tvec.dot(pvec) * inv_determinant;
                        if u < 0.0 || u > 1.0 {
                            continue;
                        }

                        let qvec = tvec.cross(va_vb);
                        let v: f32 = ray.direction.dot(qvec) * inv_determinant;
                        if v < 0.0 || (u + v) > 1.0 {
                            continue;
                        }

                        let distance: f32 = va_vb.dot(qvec) * inv_determinant;
                        let intersection = ray.direction * distance;
                        let normal = va_vb.cross(va_vc).normalize();

                        return IntersectResult::Hit(Hit {
                            distance,
                            intersection,
                            normal,
                        });
                    }
                    return IntersectResult::NoHit;
                }
            },
        };
    }

    pub fn to_triangles(&self) -> &[Triangle] {
        return self.type_.to_triangles();
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SceneObject {
    Sphere { radius: f32 },
    Mesh(Mesh),
}

impl SceneObject {
    fn to_triangles(&self) -> &[Triangle] {
        match self {
            SceneObject::Sphere { .. } => &[],
            SceneObject::Mesh(mesh) => &mesh.triangles,
        }
    }
}

#[derive(Clone, Debug)]
struct StandaloneSphere {
    position: Vec3,
    radius: f32,
}

fn intersect_sphere(position: Vec3, radius: f32, ray: &Ray) -> IntersectResult {
    let op: Vec3 = position - ray.origin;
    let eps: f32 = 1e-4;
    let b = op.dot(ray.direction);
    let mut det = b.powi(2) - op.dot(op) + radius.powi(2);
    if det < 0.0 {
        return IntersectResult::NoHit;
    } else {
        det = det.sqrt();
    }
    let t = if b - det >= eps {
        b - det
    } else if b + det >= eps {
        b + det
    } else {
        return IntersectResult::NoHit;
    };

    let xmin = ray.origin + ray.direction * t;
    let nmin = (xmin - position).normalize();

    return IntersectResult::Hit(Hit {
        distance: t,
        intersection: xmin,
        normal: nmin,
    });
}

#[derive(Clone, Debug)]
struct Mesh {
    triangles: Vec<Triangle>,
    bounding_sphere: StandaloneSphere,
}

#[derive(Clone, Debug)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    fn transformed(&self, v: &Vec3) -> Triangle {
        Triangle {
            a: self.a + *v,
            b: self.b + *v,
            c: self.c + *v,
        }
    }
}

#[derive(PartialEq, Debug)]
struct Hit {
    distance: f32,
    intersection: Vec3,
    normal: Vec3,
}

enum IntersectResult {
    NoHit,
    Hit(Hit),
}

#[derive(PartialEq, Debug)]
enum SceneIntersectResult {
    NoHit,
    Hit { object_id: usize, hit: Hit },
}

fn intersect_scene(ray: &Ray, scene_objects: &Vec<SceneObjectData>) -> SceneIntersectResult {
    let mut min_intersect: SceneIntersectResult = SceneIntersectResult::NoHit;

    for i in (0..scene_objects.len()).rev() {
        let scene_object = &scene_objects[i];
        let intersect = scene_object.intersect(ray);
        match (intersect, &min_intersect) {
            (IntersectResult::NoHit, _) => (),
            (IntersectResult::Hit(new_hit), SceneIntersectResult::NoHit) => {
                min_intersect = SceneIntersectResult::Hit {
                    object_id: i,
                    hit: new_hit,
                };
            }
            (IntersectResult::Hit(new_hit), SceneIntersectResult::Hit { hit, .. }) => {
                if new_hit.distance < hit.distance {
                    min_intersect = SceneIntersectResult::Hit {
                        object_id: i,
                        hit: new_hit,
                    };
                }
            }
        }
    }
    return min_intersect;
}

const MAX_DEPTH: usize = 12;
fn radiance(ray: &Ray, depth: usize, scene_objects: &Vec<SceneObjectData>) -> Vec3 {
    return match intersect_scene(&ray, scene_objects) {
        SceneIntersectResult::NoHit => Vec3::default(),
        SceneIntersectResult::Hit { object_id, hit } => {
            let object = &scene_objects[object_id];
            let mut color: Vec3 = object.material.color;
            let max_reflection = color.x.max(color.y.max(color.z));
            let normal_towards_ray = if hit.normal.dot(ray.direction) < 0.0 {
                hit.normal
            } else {
                hit.normal * -1.0
            };

            //--- Russian Roulette Ray termination
            let new_depth = depth + 1;
            if new_depth > 5 {
                if rand01() < max_reflection && new_depth < MAX_DEPTH {
                    color = color * (1.0 / max_reflection);
                } else {
                    return object.material.emmission;
                }
            }

            object.material.emmission
                + match object.material.reflect_type {
                    ReflectType::Diffuse => {
                        // Ideal DIFFUSE reflection

                        // cosinus-weighted importance sampling
                        let r1: f32 = 2.0 * PI * rand01();
                        let r2: f32 = rand01();
                        let r2s: f32 = r2.sqrt();
                        let w: Vec3 = normal_towards_ray;
                        let u = (if w.x.abs() > 0.1 {
                            Vec3::new(0.0, 1.0, 0.0)
                        } else {
                            Vec3::new(1.0, 0.0, 0.0)
                        })
                        .cross(w)
                        .normalize();
                        let v = w.cross(u);
                        let d = (u * r1.cos() * r2s + v * r1.sin() * r2s + w * (1.0 - r2).sqrt())
                            .normalize();

                        color
                            * radiance(
                                &Ray {
                                    origin: hit.intersection,
                                    direction: d,
                                },
                                new_depth,
                                scene_objects,
                            )
                    }
                    ReflectType::Specular => {
                        // Ideal SPECULAR reflection
                        color
                            * radiance(
                                &Ray {
                                    origin: hit.intersection,
                                    direction: ray.direction
                                        - hit.normal * 2.0 * hit.normal.dot(ray.direction),
                                },
                                new_depth,
                                scene_objects,
                            )
                    }
                    ReflectType::Refract => {
                        // Ideal dielectric REFRACTION
                        let refl_ray = Ray {
                            origin: hit.intersection,
                            direction: ray.direction
                                - hit.normal * 2.0 * hit.normal.dot(ray.direction),
                        };
                        let into = hit.normal.dot(normal_towards_ray) > 0.0; // Ray from outside going in?
                        let nc = 1.0; // Index of refraction air
                        let nt = 1.5; // Index of refraction glass
                        let nnt: f32 = if into { nc / nt } else { nt / nc };
                        let ddn = ray.direction.dot(normal_towards_ray);
                        let cos2t = 1.0 - nnt.powi(2) * (1.0 - ddn.powi(2));

                        if cos2t < 0.0 {
                            color * radiance(&refl_ray, new_depth, scene_objects)
                        } else {
                            let tdir = (ray.direction * nnt
                                - hit.normal
                                    * (if into { 1.0 } else { -1.0 } * (ddn * nnt + cos2t.sqrt())))
                            .normalize();
                            let a = nt - nc;
                            let b = nt + nc;
                            let r0 = a * a / (b * b);
                            let c = 1.0 - (if into { -ddn } else { tdir.dot(hit.normal) });
                            let re = r0 + (1.0 - r0) * c.powi(5);
                            let tr = 1.0 - re;
                            let p = 0.25 + 0.5 * re;
                            let rp = re / p;
                            let tp = tr / (1.0 - p);

                            if new_depth > 2 {
                                if rand01() < p {
                                    color * radiance(&refl_ray, new_depth, scene_objects) * rp
                                } else {
                                    color
                                        * radiance(
                                            &Ray {
                                                origin: hit.intersection,
                                                direction: tdir,
                                            },
                                            new_depth,
                                            scene_objects,
                                        )
                                        * tp
                                }
                            } else {
                                color
                                    * (radiance(&refl_ray, new_depth, scene_objects) * re
                                        + radiance(
                                            &Ray {
                                                origin: hit.intersection,
                                                direction: tdir,
                                            },
                                            new_depth,
                                            scene_objects,
                                        ) * tr)
                            }
                        }
                    }
                }
        }
    };
}

pub(crate) struct RenderConfig {
    pub samples_per_pixel: usize,
    pub resolution_y: usize,
    pub scene: SceneData,
}

impl Default for RenderConfig {
    fn default() -> Self {
        let scenes = load_scenes();
        Self {
            samples_per_pixel: 400,
            resolution_y: 300,
            scene: scenes.into_iter().next().unwrap(),
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct RenderUpdate {
    pub progress: f64,
    pub image: Image,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub pixels: Vec<Vec3>,
    pub resolution: (usize, usize),
    pub hash: u64,
}
impl Image {
    fn new(pixels: Vec<Vec3>, resolution: (usize, usize)) -> Self {
        Self {
            hash: hash_vec_of_vectors(&pixels),
            pixels,
            resolution,
        }
    }
}

fn benchmark_function<T, F: FnOnce() -> T>(func: F) -> T {
    let start = std::time::Instant::now();
    let t = func();
    println!("Elapsed time: {:.2?}", start.elapsed());
    return t;
}

pub fn hash_vec_of_vectors(vectors: &[Vec3]) -> u64 {
    benchmark_function(|| {
        let mut hasher = DefaultHasher::new();
        for v in vectors {
            v.x.to_bits().hash(&mut hasher);
            v.y.to_bits().hash(&mut hasher);
            v.z.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    })
}

pub fn render(
    render_config: RenderConfig,
    send_update_progress: &(impl Sink<RenderUpdate, Error = SendError> + Unpin + Clone + Sync + Send),
) -> Image {
    let image = thread::scope(move |s| {
        let resy = render_config.resolution_y;
        let resx: usize = resy * 3 / 2;

        let scene = render_config.scene;
        let time_start = Instant::now();
        let scene_objects = scene.objects.clone();

        //-- setup sensor
        let sensor_origin: Vec3 = scene.camera.position;
        let sensor_view_direction: Vec3 = scene.camera.direction.normalize();
        let sensor_width: f32 = 0.036;
        let sensor_height: f32 = sensor_width * 2.0 / 3.0;
        let focal_length: f32 = scene.camera.focal_length;
        // lens center (pinhole)
        let lens_center = sensor_origin + sensor_view_direction * focal_length;

        //-- orthogonal axes spanning the sensor plane
        let su: Vec3 = sensor_view_direction
            .cross(if sensor_view_direction.y.abs() < 0.9 {
                Vec3::new(0.0, 1.0, 0.0)
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            })
            .normalize();
        let sv: Vec3 = su.cross(sensor_view_direction);

        let grid_size = resx * resy;

        println!(
            "Scene {} ({} objects), {} samples per pixel, {}x{} resolution{}",
            scene.id,
            scene_objects.len(),
            render_config.samples_per_pixel,
            render_config.resolution_y * 3 / 2,
            render_config.resolution_y,
            if MOCK_RANDOM { " (mock random)" } else { "" }
        );

        // let last_progress_print_time = atomic::AtomicU64::new(0);
        // let max_time_between_progress_prints = 1000;
        let processed_pixel_count = Arc::new(atomic::AtomicUsize::new(0));
        let get_processed_pixel_count = processed_pixel_count.clone();

        // TODO use better concurrent vec
        let pixels = Arc::new(std::sync::Mutex::new(vec![Vec3::default(); grid_size]));
        let get_pixels = pixels.clone();

        // Start thread that sends regular progress updates
        let (stop_background_thread, should_stop) = mpsc::channel();
        let resolution = (resx.clone(), resy.clone());
        s.spawn(move || loop {
            let mut send_update_progress = send_update_progress.clone();
            let processed_percentage = get_processed_pixel_count.load(atomic::Ordering::Relaxed)
                as f64
                / (grid_size) as f64;
            let _ = futures::executor::block_on(send_update_progress.send(RenderUpdate {
                progress: processed_percentage,
                image: Image::new(get_pixels.lock().unwrap().clone(), resolution),
            }));
            thread::sleep(Duration::from_millis(500));
            if let Ok(_) = should_stop.try_recv() {
                break;
            }
        });

        let render_thread_handle = s.spawn(move || {
            // Pure function for rendering a single pixel
            let render_pixel = |pixel_index: usize| -> Vec3 {
                let y = resy - 1 - pixel_index / resx;
                let x = pixel_index % resx;

                let mut radiance_v: Vec3 = Vec3::default();

                for s in 0..render_config.samples_per_pixel {
                    // map to 2x2 subpixel rows and cols
                    let ysub: f32 = ((s / 2) % 2) as f32;
                    let xsub: f32 = (s % 2) as f32;

                    // sample sensor subpixel in [-1,1]
                    let r1: f32 = 2.0 * rand01();
                    let r2: f32 = 2.0 * rand01();
                    let xfilter: f32 = if r1 < 1.0 {
                        // TODO not sure what this is
                        r1.sqrt() - 1.0
                    } else {
                        1.0 - (2.0 - r1).sqrt()
                    };
                    let yfilter: f32 = if r2 < 1.0 {
                        r2.sqrt() - 1.0
                    } else {
                        1.0 - (2.0 - r2).sqrt()
                    };

                    // x and y sample position on sensor plane
                    let sx: f32 = ((x as f32 + 0.5 * (0.5 + xsub + xfilter)) / resx as f32 - 0.5)
                        * sensor_width;
                    let sy: f32 = ((y as f32 + 0.5 * (0.5 + ysub + yfilter)) / resy as f32 - 0.5)
                        * sensor_height;

                    // 3d sample position on sensor
                    let sensor_pos = sensor_origin + su * sx + sv * sy;
                    let ray_direction = (lens_center - sensor_pos).normalize();
                    // ray through pinhole
                    let ray = Ray {
                        origin: lens_center,
                        direction: ray_direction,
                    };

                    // evaluate radiance from this ray and accumulate
                    radiance_v = radiance_v + radiance(&ray, 0, &scene_objects);
                }
                // normalize radiance by number of samples
                radiance_v = radiance_v / render_config.samples_per_pixel as f32;
                processed_pixel_count.fetch_add(1, atomic::Ordering::Relaxed);

                Vec3::new(
                    radiance_v.x.clamp(0.0, 1.0),
                    radiance_v.y.clamp(0.0, 1.0),
                    radiance_v.z.clamp(0.0, 1.0),
                )
            };

            let render_pixel_to_vec = |pixel_index: usize| {
                let pixel_value = render_pixel(pixel_index);
                let mut pixels = pixels.lock().unwrap();
                pixels[pixel_index] = pixel_value;
            };

            if MOCK_RANDOM {
                (0..grid_size).into_iter().for_each(render_pixel_to_vec);
            } else {
                // Use rayon to parallelize rendering
                let mut indices: Vec<usize> = (0..grid_size).collect();
                indices.shuffle(&mut rand::thread_rng());
                indices.into_par_iter().for_each(render_pixel_to_vec);
            };
            let _ = stop_background_thread.send(());

            // print_progress();
            // println!();

            // Create directory if it does not exist
            std::fs::create_dir_all("out").unwrap();

            // Write .ppm file
            let path = format!(
                "out/{}-scene-{}-spp{}-res{}-.ppm",
                chrono::Local::now().format("%Y-%m-%d_%H:%M:%S").to_string(),
                scene.id,
                render_config.samples_per_pixel,
                render_config.resolution_y,
            );
            let mut file = std::fs::File::create(path.clone()).unwrap();
            file.write_all(b"P3\n").unwrap();
            file.write_all(
                format!(
                    "# samplesPerPixel: {}, resolution_y: {}, scene_id: {}\n",
                    render_config.samples_per_pixel, render_config.resolution_y, scene.id
                )
                .as_bytes(),
            )
            .unwrap();
            file.write_all(
                format!(
                    "# rendering time: {} s\n",
                    std::time::Instant::now()
                        .duration_since(time_start)
                        .as_secs()
                )
                .as_bytes(),
            )
            .unwrap();
            file.write_all(format!("{} {}\n{}\n", resx, resy, 255).as_bytes())
                .unwrap();
            let pixels = pixels.lock().unwrap().to_vec();
            for pixel in pixels.iter().rev() {
                file.write_all(
                    format!(
                        "{} {} {} ",
                        to_int_with_gamma_correction(pixel.x),
                        to_int_with_gamma_correction(pixel.y),
                        to_int_with_gamma_correction(pixel.z)
                    )
                    .as_bytes(),
                )
                .unwrap();
            }

            // Create symlink for easy access to newest image
            std::fs::remove_file("latest.ppm").unwrap_or_default();
            match std::os::unix::fs::symlink(path.clone(), "latest.ppm") {
                Ok(_) => (),
                Err(_) => {
                    println!(
                        "Could not create symlink to latest image. You can find it at {}",
                        path
                    );
                }
            }

            return Image::new(pixels, (resx, resy));
        });

        let image = render_thread_handle.join().unwrap();
        return image;
    });

    return image;
}
