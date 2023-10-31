use std::{
    fmt::Display,
    io::Write,
    ops::{Add, Div, Mul, Sub},
    process::exit,
    sync::atomic,
    time::Duration,
};

use rayon::prelude::*;

const PI: f64 = 3.141592653589793;

// uniform double random generator function
fn rand01() -> f64 {
    return rand::random::<f64>();
}

fn to_int_with_gamma_correction(x: f64) -> usize {
    return (255.0 * x.clamp(0.0, 1.0).powf(1.0 / 2.2) + 0.5) as usize;
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vector {
    x: f64,
    y: f64,
    z: f64,
}

impl Add<Self> for Vector {
    type Output = Self;

    fn add(mut self, other: Self) -> Self::Output {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
        return self;
    }
}

impl Sub<Self> for Vector {
    type Output = Self;

    fn sub(mut self, other: Self) -> Self::Output {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
        return self;
    }
}

impl Mul<f64> for Vector {
    type Output = Self;

    fn mul(mut self, v: f64) -> Self::Output {
        self.x *= v;
        self.y *= v;
        self.z *= v;
        return self;
    }
}

impl Div<f64> for Vector {
    type Output = Self;

    fn div(mut self, v: f64) -> Self::Output {
        self.x /= v;
        self.y /= v;
        self.z /= v;
        return self;
    }
}

impl Mul<Self> for Vector {
    type Output = Self;

    fn mul(mut self, other: Self) -> Self::Output {
        self.x *= other.x;
        self.y *= other.y;
        self.z *= other.z;
        return self;
    }
}

impl Vector {
    fn zero() -> Self {
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn from(a: f64, b: f64, c: f64) -> Self {
        Vector { x: a, y: b, z: c }
    }

    fn uniform(u: f64) -> Self {
        Vector { x: u, y: u, z: u }
    }

    fn normalize(mut self) -> Self {
        let f = self.magnitude();
        self.x /= f;
        self.y /= f;
        self.z /= f;
        return self;
    }

    fn normalized(&self) -> Self {
        return self.clone() / self.magnitude();
    }

    fn dot(&self, other: &Vector) -> f64 {
        return self.x * other.x + self.y * other.y + self.z * other.z;
    }

    fn cross(&self, other: &Vector) -> Vector {
        return Vector {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        };
    }

    fn magnitude(&self) -> f64 {
        return (self.x.powi(2) + self.y.powi(2) + self.z.powi(2)).sqrt();
    }
}

struct Ray {
    origin: Vector,
    direction: Vector,
}

enum ReflectType {
    Diffuse,
    Specular,
    Refract,
}

struct Material {
    color: Vector,
    emmission: Vector,
    reflect_type: ReflectType,
}

struct SceneObject {
    type_: SceneObjectType,
    material: Material,
}

enum SceneObjectType {
    Sphere { position: Vector, radius: f64 },
}

#[derive(PartialEq, Debug)]
struct Hit {
    distance: f64,
    xmin: Vector,
    nmin: Vector,
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

impl SceneObjectType {
    fn intersect(&self, ray: &Ray) -> IntersectResult {
        match *self {
            SceneObjectType::Sphere { position, radius } => {
                let op: Vector = position - ray.origin;
                let eps: f64 = 1e-4;
                let b = op.dot(&ray.direction);
                let mut det = b * b - op.dot(&op) + radius.powi(2);
                if det < 0.0 {
                    return IntersectResult::NoHit;
                } else {
                    det = det.sqrt();
                }
                if b - det < eps && b + det < eps {
                    return IntersectResult::NoHit;
                }
                let t = b - det;
                let xmin = ray.origin + ray.direction * t;
                let nmin = xmin - position;
                nmin.normalize();

                return IntersectResult::Hit(Hit {
                    distance: t,
                    xmin: xmin,
                    nmin: nmin,
                });
            }
        }
    }
}

fn intersect_scene(ray: &Ray, scene_objects: &Vec<SceneObject>) -> SceneIntersectResult {
    let mut min_intersect: SceneIntersectResult = SceneIntersectResult::NoHit;

    for i in (0..scene_objects.len()).rev() {
        let scene_object = &scene_objects[i];
        let intersect = scene_object.type_.intersect(ray);
        match (intersect, &min_intersect) {
            (IntersectResult::NoHit, _) => (),
            (IntersectResult::Hit(new_hit), SceneIntersectResult::NoHit) => {
                min_intersect = SceneIntersectResult::Hit {
                    object_id: i,
                    hit: new_hit,
                };
            }
            (IntersectResult::Hit(new_hit), SceneIntersectResult::Hit { object_id: _, hit }) => {
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
fn radiance(ray: &Ray, depth: usize, scene_objects: &Vec<SceneObject>) -> Vector {
    return match intersect_scene(&ray, scene_objects) {
        SceneIntersectResult::NoHit => Vector::zero(),
        SceneIntersectResult::Hit { object_id, hit } => {
            let object = &scene_objects[object_id];
            let mut color: Vector = object.material.color;
            let max_reflection = color.x.max(color.y.max(color.z));
            let normal_towards_ray = if hit.nmin.dot(&ray.direction) < 0.0 {
                hit.nmin
            } else {
                hit.nmin * -1.0
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
                        let r1: f64 = 2.0 * PI * rand01();
                        let r2: f64 = rand01();
                        let r2s: f64 = r2.sqrt();
                        let w: Vector = normal_towards_ray;
                        let u = (if w.x.abs() > 0.1 {
                            Vector::from(0.0, 1.0, 0.0)
                        } else {
                            Vector::from(1.0, 0.0, 0.0)
                        })
                        .cross(&w)
                        .normalize();
                        let v = w.cross(&u);
                        let d = (u * r1.cos() * r2s + v * r1.sin() * r2s + w * (1.0 - r2).sqrt())
                            .normalize();

                        color
                            * radiance(
                                &Ray {
                                    origin: hit.xmin,
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
                                    origin: hit.xmin,
                                    direction: ray.direction
                                        - hit.nmin * 2.0 * hit.nmin.dot(&ray.direction),
                                },
                                new_depth,
                                scene_objects,
                            )
                    }
                    ReflectType::Refract => {
                        // Ideal dielectric REFRACTION
                        let refl_ray = Ray {
                            origin: hit.xmin,
                            direction: ray.direction
                                - hit.nmin * 2.0 * hit.nmin.dot(&ray.direction),
                        };
                        let into = hit.nmin.dot(&normal_towards_ray) > 0.0; // Ray from outside going in?
                        let nc = 1.0; // Index of refraction air
                        let nt = 1.5; // Index of refraction glass
                        let nnt: f64 = if into { nc / nt } else { nt / nc };
                        let ddn = ray.direction.dot(&normal_towards_ray);
                        let cos2t = 1.0 - nnt.powi(2) * (1.0 - ddn.powi(2));

                        if cos2t < 0.0 {
                            color * radiance(&refl_ray, new_depth, scene_objects)
                        } else {
                            let tdir = (ray.direction * nnt
                                - hit.nmin
                                    * (if into { 1.0 } else { -1.0 } * (ddn * nnt + cos2t.sqrt())))
                            .normalized();
                            let a = nt - nc;
                            let b = nt + nc;
                            let r0 = a * a / (b * b);
                            let c = 1.0 - (if into { -ddn } else { tdir.dot(&hit.nmin) });
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
                                                origin: hit.xmin,
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
                                                origin: hit.xmin,
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

struct RenderConfig {
    samples_per_pixel: usize,
    resolution_y: usize,
    scene_id: SceneId,
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
                Some(RenderConfig {
                    samples_per_pixel: args.get(1)?.parse().ok()?,
                    resolution_y: args.get(2)?.parse().ok()?,
                    scene_id,
                })
            }
            1 => Some(RenderConfig::default()),
            _ => None,
        };
    }

    fn default() -> Self {
        Self {
            samples_per_pixel: 4000,
            resolution_y: 600,
            scene_id: SceneId::Int(0),
        }
    }
}

fn main() {
    let time_start = std::time::Instant::now();

    // Set up scene
    const BOX_DIMENSIONS: Vector = Vector {
        x: 2.6,
        y: 2.0,
        z: 2.8,
    };

    // scene_id to scene_objects
    let scenes: Vec<(&str, Vec<SceneObject>)> = [
        (
            "single-sphere",
            vec![SceneObject {
                type_: SceneObjectType::Sphere {
                    position: Vector::from(0.0, 0.0, 0.0),
                    radius: 1.0,
                },
                material: Material {
                    color: Vector::from(1.0, 1.0, 1.0),
                    emmission: Vector::from(0.98 * 15.0, 15.0, 0.9 * 15.0),
                    reflect_type: ReflectType::Diffuse,
                },
            }],
        ),
        (
            "two-spheres",
            vec![
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, 0.0, 0.0),
                        radius: 1.0,
                    },
                    material: Material {
                        color: Vector::from(1.0, 0.0, 0.0),
                        emmission: Vector::from(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, 0.0, 10.0),
                        radius: 1.0,
                    },
                    material: Material {
                        color: Vector::from(0.0, 0.0, 0.0),
                        emmission: Vector::uniform(10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
        ),
        (
            "three-spheres",
            vec![
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, 0.0, -3.0),
                        radius: 1.0,
                    },
                    material: Material {
                        color: Vector::from(1.0, 0.2, 0.2),
                        emmission: Vector::from(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(4.0, 2.0, 0.0),
                        radius: 1.0,
                    },
                    material: Material {
                        color: Vector::from(0.0, 0.0, 0.0),
                        emmission: Vector::from(20.0, 10.0, 10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(-6.0, -2.0, 0.0),
                        radius: 1.0,
                    },
                    material: Material {
                        color: Vector::from(0.0, 0.0, 0.0),
                        emmission: Vector::from(5.0, 9.0, 20.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
        ),
        (
            "cornell",
            vec![
                // Cornell Box centered in the origin (0, 0, 0)
                // Left
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(-1e5 - BOX_DIMENSIONS.x, 0.0, 0.0),
                        radius: 1e5,
                    },
                    material: Material {
                        color: Vector::from(0.85, 0.25, 0.25),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // Right
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(1e5 + BOX_DIMENSIONS.x, 0.0, 0.0),
                        radius: 1e5,
                    },
                    material: Material {
                        color: Vector::from(0.25, 0.35, 0.85),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // Top
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, 1e5 + BOX_DIMENSIONS.y, 0.0),
                        radius: 1e5,
                    },
                    material: Material {
                        color: Vector::from(0.75, 0.75, 0.75),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // Bottom
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, -1e5 - BOX_DIMENSIONS.y, 0.0),
                        radius: 1e5,
                    },
                    material: Material {
                        color: Vector::from(0.75, 0.75, 0.75),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // Back
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, 0.0, -1e5 - BOX_DIMENSIONS.z),
                        radius: 1e5,
                    },
                    material: Material {
                        color: Vector::from(0.75, 0.75, 0.75),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // Front
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, 0.0, 1e5 + 3.0 * BOX_DIMENSIONS.z - 0.5),
                        radius: 1e5,
                    },
                    material: Material {
                        color: Vector::zero(),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // Objects
                // mirroring
                // SceneObject {
                //     type_: SceneObjectType::Sphere {
                //         position: Vector::from(-1.3, -BOX_DIMENSIONS.y + 0.8, -1.3),
                //         radius: 0.8,
                //     },
                //     material: Material {
                //         color: Vector::uniform(0.999),
                //         emmission: Vector::zero(),
                //         reflect_type: ReflectType::Specular,
                //     },
                // },
                // // refracting
                // SceneObject {
                //     type_: SceneObjectType::Sphere {
                //         position: Vector::from(1.3, -BOX_DIMENSIONS.y + 0.8, -0.2),
                //         radius: 0.8,
                //     },
                //     material: Material {
                //         color: Vector::uniform(0.999),
                //         emmission: Vector::zero(),
                //         reflect_type: ReflectType::Refract,
                //     },
                // },
                // The ceiling area light source (slightly yellowish color)
                SceneObject {
                    type_: SceneObjectType::Sphere {
                        position: Vector::from(0.0, BOX_DIMENSIONS.y + 10.0 - 0.04, 0.0),
                        radius: 10.0,
                    },
                    material: Material {
                        color: Vector::zero(),
                        // emmission: Vector::from(0.98 * 2.0, 2.0, 0.9 * 2.0),
                        emmission: Vector::from(0.98, 1.0, 0.9) * 5.0,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
        ),
    ]
    .into();

    let print_usage = || {
        println!(
            "Run with:\ncargo run <samplesPerPixel = 4000> <y-resolution = 600> <scene = '{}'>\n\nScenes: {}",
            scenes.iter().next().unwrap().0,
            scenes.iter().enumerate().map(|(i, scene)| format!("{}: {}", i, scene.0)).collect::<Vec<_>>().join(", ")
        );
    };

    let maybe_render_config = RenderConfig::from(std::env::args().collect());
    match maybe_render_config {
        None => {
            print_usage();
            exit(1);
        }
        Some(render_config) => {
            let scene_objects: &Vec<SceneObject> = &match render_config.scene_id.clone() {
                SceneId::Int(i) => scenes.get(i),
                SceneId::String(s) => scenes.iter().find(|scene| scene.0 == s.as_str()),
            }
            .unwrap_or_else(|| {
                print_usage();
                exit(1);
            })
            .1;

            //-- setup sensor
            let sensor_origin: Vector =
                Vector::from(0.0, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0);
            // normal to sensor plane
            let sensor_view_direction: Vector = Vector::from(0.0, -0.06, -1.0).normalized();
            let sensor_width: f64 = 0.036;
            let sensor_height: f64 = 0.024;
            // in meters
            let focal_length: f64 = 0.035;

            //-- orthogonal axes spanning the sensor plane
            let su: Vector = sensor_view_direction
                .cross(&if sensor_view_direction.y.abs() < 0.9 {
                    Vector::from(0.0, 1.0, 0.0)
                } else {
                    Vector::from(0.0, 0.0, 1.0)
                })
                .normalized();
            let sv: Vector = su.cross(&sensor_view_direction);

            let resy = render_config.resolution_y;
            let resx: usize = resy * 3 / 2;
            let grid_size = resx * resy;

            let last_progress_print_time = atomic::AtomicU64::new(0);
            let max_time_between_progress_prints = 1000;
            let processed_pixel_count = atomic::AtomicUsize::new(0);

            let print_progress = || {
                fn fmt(d: std::time::Duration) -> String {
                    let seconds = d.as_secs() % 60;
                    let minutes = (d.as_secs() / 60) % 60;
                    let hours = (d.as_secs() / 60) / 60;
                    if hours == 0 {
                        return format!("{}m:{:0>2}s", minutes, seconds);
                    }
                    format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
                }
                let processed_percentage = processed_pixel_count.load(atomic::Ordering::Relaxed)
                    as f64
                    / (grid_size) as f64;
                let elapsed = time_start.elapsed();
                print!(
                    "Rendering ... {:3.1}% ({} / {})\r",
                    100.0 * processed_percentage,
                    fmt(elapsed),
                    fmt(Duration::from_secs(
                        (elapsed.as_secs() as f64 * (1.0 / processed_percentage)) as u64
                    ))
                );
                std::io::stdout().flush().unwrap();
                last_progress_print_time.store(
                    time_start.elapsed().as_millis() as u64,
                    atomic::Ordering::Relaxed,
                );
            };

            print_progress();

            // Use rayon to parallelize rendering
            let pixels: Vec<Vector> = (0..grid_size)
                .into_par_iter()
                .map_init(
                    || Vector::zero(),
                    |_, pixel_index| {
                        if last_progress_print_time.fetch_add(0, atomic::Ordering::Relaxed)
                            + max_time_between_progress_prints
                            < time_start.elapsed().as_millis() as u64
                        {
                            print_progress();
                        }

                        let y = resy - pixel_index / resx;
                        let x = pixel_index % resx;

                        let mut radiance_v: Vector = Vector::zero();

                        for s in 0..render_config.samples_per_pixel {
                            // map to 2x2 subpixel rows and cols
                            let ysub: f64 = ((s / 2) % 2) as f64;
                            let xsub: f64 = (s % 2) as f64;

                            // sample sensor subpixel in [-1,1]
                            let r1: f64 = 2.0 * rand01();
                            let r2: f64 = 2.0 * rand01();
                            let xfilter: f64 = if r1 < 1.0 {
                                // TODO not sure what this is
                                r1.sqrt() - 1.0
                            } else {
                                1.0 - (2.0 - r1).sqrt()
                            };
                            let yfilter: f64 = if r1 < 1.0 {
                                r2.sqrt() - 1.0
                            } else {
                                1.0 - (2.0 - r2).sqrt()
                            };

                            // x and y sample position on sensor plane
                            let sx: f64 = ((x as f64 + 0.5 * (0.5 + xsub + xfilter)) / resx as f64
                                - 0.5)
                                * sensor_width;
                            let sy: f64 = ((y as f64 + 0.5 * (0.5 + ysub + yfilter)) / resy as f64
                                - 0.5)
                                * sensor_height;

                            // 3d sample position on sensor
                            let sensor_pos = sensor_origin + su * sx + sv * sy;
                            // lens center (pinhole)
                            let lens_center = sensor_origin + sensor_view_direction * focal_length;
                            let ray_direction = (lens_center - sensor_pos).normalized();
                            // ray through pinhole
                            let ray = Ray {
                                origin: lens_center,
                                direction: ray_direction,
                            };

                            radiance_v = radiance_v + radiance(&ray, 0, &scene_objects);
                            // evaluate radiance from this ray and accumulate
                        }
                        radiance_v = radiance_v / render_config.samples_per_pixel as f64; // normalize radiance by number of samples

                        let clamped_radiance = Vector::from(
                            radiance_v.x.clamp(0.0, 1.0),
                            radiance_v.y.clamp(0.0, 1.0),
                            radiance_v.z.clamp(0.0, 1.0),
                        );

                        processed_pixel_count.fetch_add(1, atomic::Ordering::Relaxed);

                        clamped_radiance
                    },
                )
                .collect();
            print_progress();
            println!();

            // Create directory if it does not exist
            std::fs::create_dir_all("out").unwrap();

            // Write .ppm file
            let mut file = std::fs::File::create(format!(
                "out/{}-scene-{}-spp{}-res{}-.ppm",
                chrono::Local::now().format("%Y-%m-%d_%H:%M:%S").to_string(),
                render_config.scene_id,
                render_config.samples_per_pixel,
                render_config.resolution_y,
            ))
            .unwrap();
            file.write_all(b"P3\n").unwrap();
            file.write_all(
                format!(
                    "# samplesPerPixel: {}, resolution_y: {}, scene_id: {}\n",
                    render_config.samples_per_pixel,
                    render_config.resolution_y,
                    render_config.scene_id
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
        }
    }
}

#[cfg(test)]
mod test;
