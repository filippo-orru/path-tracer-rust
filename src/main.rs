use std::{
    f32::consts::PI,
    io::Write,
    ops::{Add, Div, Mul, Sub},
};

// uniform double random generator function
fn rand01() -> f32 {
    return rand::random::<f32>();
}

fn to_int_with_gamma_correction(x: f32) -> usize {
    return (255.0 * x.clamp(0.0, 1.0).powf(1.0 / 2.2) * 0.5) as usize;
}

#[derive(Clone, Copy, Debug)]
struct Vector {
    x: f32,
    y: f32,
    z: f32,
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

impl Mul<f32> for Vector {
    type Output = Self;

    fn mul(mut self, v: f32) -> Self::Output {
        self.x *= v;
        self.y *= v;
        self.z *= v;
        return self;
    }
}

impl Div<f32> for Vector {
    type Output = Self;

    fn div(mut self, v: f32) -> Self::Output {
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

    fn from(a: f32, b: f32, c: f32) -> Self {
        Vector { x: a, y: b, z: c }
    }

    fn uniform(u: f32) -> Self {
        Vector { x: u, y: u, z: u }
    }

    fn normalize(&mut self) -> &mut Self {
        let f = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        self.x /= f;
        self.y /= f;
        self.z /= f;
        return self;
    }

    fn normalized(&self) -> Self {
        let f = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        return self.clone() / f;
    }

    fn dot(&self, other: &Vector) -> f32 {
        return self.x * other.x + self.y * other.y + self.z * other.z;
    }

    fn cross(&self, other: &Vector) -> Vector {
        return Vector {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        };
    }

    fn length(&self) -> f32 {
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
    Sphere { position: Vector, radius: f32 },
}

struct Hit {
    distance: f32,
    xmin: Vector,
    nmin: Vector,
}

enum IntersectResult {
    NoHit,
    Hit(Hit),
}

enum SceneIntersectResult {
    NoHit,
    Hit { object_id: usize, hit: Hit },
}

impl SceneObjectType {
    fn intersect(&self, ray: &Ray) -> IntersectResult {
        match *self {
            SceneObjectType::Sphere { position, radius } => {
                let op: Vector = position - ray.origin;
                let eps: f32 = 1e-4;
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
                let mut nmin = xmin - position;
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
fn radiance(ray: Ray, depth: usize, scene_objects: &Vec<SceneObject>) -> Vector {
    return match intersect_scene(&ray, scene_objects) {
        SceneIntersectResult::NoHit => Vector::zero(),
        SceneIntersectResult::Hit { object_id, hit } => {
            let object = &scene_objects[object_id];
            let mut f: Vector = object.material.color;
            let max_reflection = f.x.max(f.y.max(f.z));
            let normal_towards_ray = if hit.nmin.dot(&ray.direction) < 0.0 {
                hit.nmin
            } else {
                hit.nmin * -1.0
            };

            //--- Russian Roulette Ray termination
            let new_depth = depth + 1;
            if new_depth > 5 {
                if rand01() < max_reflection && new_depth < MAX_DEPTH {
                    f = f * (1.0 / max_reflection);
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
                        let w: Vector = normal_towards_ray;
                        let u = (if w.x.abs() > 0.1 {
                            Vector::from(0.0, 1.0, 0.0)
                        } else {
                            Vector::from(1.0, 0.0, 0.0)
                        })
                        .cross(&w)
                        .normalized();
                        let v = w.cross(&u);
                        let d = (u * r1.cos() * r2s + v * r1.sin() * r2s + w * (1.0 - r2).sqrt())
                            .normalized();
                        // TODO consider using .normalize() for performance

                        let radiance_recursed = radiance(
                            Ray {
                                origin: hit.xmin,
                                direction: d,
                            },
                            new_depth,
                            scene_objects,
                        );

                        f * radiance_recursed
                    }
                    ReflectType::Specular => {
                        // Ideal SPECULAR reflection
                        let radiance_recursed = radiance(
                            Ray {
                                origin: hit.xmin,
                                direction: ray.direction
                                    - hit.nmin * 2.0 * hit.nmin.dot(&ray.direction),
                            },
                            new_depth,
                            scene_objects,
                        );

                        f * radiance_recursed
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
                        let nnt: f32 = if into { nc / nt } else { nt / nc };
                        let ddn = ray.direction.dot(&normal_towards_ray);
                        let cos2t = 1.0 - nnt.powi(2) * (1.0 - ddn.powi(2));

                        if cos2t < 0.0 {
                            object.material.emmission
                                + f * radiance(refl_ray, new_depth, scene_objects)
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
                                    f * radiance(refl_ray, new_depth, scene_objects) * rp
                                } else {
                                    f * radiance(
                                        Ray {
                                            origin: hit.xmin,
                                            direction: tdir,
                                        },
                                        new_depth,
                                        scene_objects,
                                    ) * tp
                                }
                            } else {
                                f * (radiance(refl_ray, new_depth, scene_objects) * re
                                    + radiance(
                                        Ray {
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
    num_threads: usize,
    scene_id: usize,
}

impl RenderConfig {
    fn from(v: Vec<String>) -> Option<Self> {
        return match v.len() {
            5 => Some(RenderConfig {
                samples_per_pixel: v.get(1)?.parse().ok()?,
                resolution_y: v.get(2)?.parse().ok()?,
                num_threads: v.get(3)?.parse().ok()?,
                scene_id: v.get(4)?.parse().ok()?,
            }),
            1 => Some(RenderConfig::default()),
            _ => None,
        };
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            samples_per_pixel: 4000,
            resolution_y: 600,
            num_threads: std::thread::available_parallelism().unwrap().get(),
            scene_id: 1,
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

    let scene_objects: Vec<SceneObject> = vec![
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
                emmission: Vector::from(0.98 * 15.0, 15.0, 0.9 * 15.0),
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
                color: Vector::from(0.75, 0.75, 0.75),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Objects
        // mirroring
        SceneObject {
            type_: SceneObjectType::Sphere {
                position: Vector::from(-1.3, -BOX_DIMENSIONS.y + 0.8, -1.3),
                radius: 8.0,
            },
            material: Material {
                color: Vector::uniform(0.999),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Specular,
            },
        },
        // refracting
        SceneObject {
            type_: SceneObjectType::Sphere {
                position: Vector::from(1.3, -BOX_DIMENSIONS.y + 0.8, -0.2),
                radius: 8.0,
            },
            material: Material {
                color: Vector::uniform(0.999),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Refract,
            },
        },
        // The ceiling area light source (slightly yellowish color)
        SceneObject {
            type_: SceneObjectType::Sphere {
                position: Vector::from(0.0, BOX_DIMENSIONS.y + 10.0 - 0.04, 0.0),
                radius: 10.0,
            },
            material: Material {
                color: Vector::zero(),
                emmission: Vector::from(0.98 * 15.0, 15.0, 0.9 * 15.0),
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let maybe_render_config = RenderConfig::from(std::env::args().collect());
    match maybe_render_config {
        None => {
            println!(
                "Run with `cargo run <samplesPerPixel = 4000> <y-resolution = 600> <num-threads = {}> <scene = 1>", 
                std::thread::available_parallelism().unwrap().get()
            );
        }
        Some(render_config) => {
            //-- setup sensor
            let sensor_origin: Vector =
                Vector::from(0.0, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0);
            // normal to sensor plane
            let sensor_view_direction: Vector = Vector::from(0.0, -0.06, -1.0).normalized();
            let sensor_width: f32 = 0.036;
            let sensor_height: f32 = 0.024;
            // in meters
            let focal_length: f32 = 0.035;
            let resy = render_config.resolution_y;
            let resx: usize = resy * 3 / 2;
            let mut pixels: Vec<Vector> = vec![Vector::zero(); resx * resy];

            //-- orthogonal axes spanning the sensor plane
            let su: Vector = sensor_view_direction
                .cross(&if sensor_view_direction.y.abs() < 0.9 {
                    Vector::from(0.0, 1.0, 0.0)
                } else {
                    Vector::from(0.0, 0.0, 1.0)
                })
                .normalized();
            let sv: Vector = su.cross(&sensor_view_direction);

            for y in 0..resy {
                println!("Progress: {:3.1}%", (y * 100) as f64 / (resy - 1) as f64);

                for x in 0..resx {
                    let mut radiance_v: Vector = Vector::zero();

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
                        let yfilter: f32 = if r1 < 1.0 {
                            r2.sqrt() - 1.0
                        } else {
                            1.0 - (2.0 - r2).sqrt()
                        };

                        // x and y sample position on sensor plane
                        let sx: f32 = ((x as f32 + 0.5 * (0.5 + xsub + xfilter)) / resx as f32
                            - 0.5)
                            * sensor_width;
                        let sy: f32 = ((y as f32 + 0.5 * (0.5 + ysub + yfilter)) / resy as f32
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

                        radiance_v = radiance_v + radiance(ray, 0, &scene_objects);
                        // evaluate radiance from this ray and accumulate
                    }
                    radiance_v = radiance_v / render_config.samples_per_pixel as f32; // normalize radiance by number of samples

                    let i: usize = (resy - y - 1) * resx + x; // buffer location of this pixel
                    let clamped_radiance = Vector::from(
                        radiance_v.x.clamp(0.0, 1.0),
                        radiance_v.y.clamp(0.0, 1.0),
                        radiance_v.z.clamp(0.0, 1.0),
                    );
                    pixels[i] = pixels[i] + clamped_radiance;
                }
            }

            // Create directory if it does not exist
            std::fs::create_dir_all("out").unwrap();
            
            // Write .ppm file
            let mut file = std::fs::File::create(format!(
                "out/{}-scene{}-spp{}-res{}-.ppm",
                chrono::Local::now().format("%Y-%m-%d_%H:%M:%S").to_string(),
                render_config.scene_id,
                render_config.samples_per_pixel,
                render_config.resolution_y,
            ))
            .unwrap();
            file.write_all(b"P3\n").unwrap();
            file.write_all(
                format!(
                    "# samplesPerPixel: {}, resolution_y: {}, num_threads: {}, scene_id: {}\n",
                    render_config.samples_per_pixel,
                    render_config.resolution_y,
                    render_config.num_threads,
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
