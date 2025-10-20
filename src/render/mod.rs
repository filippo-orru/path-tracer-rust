mod load_off;
pub mod scenes;

#[cfg(test)]
mod test;

use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{Hash, Hasher},
    io::Write,
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
    thread,
    time::{Duration, Instant},
};

use glam::Vec3;
use iced::futures::{self, Sink, SinkExt, channel::mpsc::SendError};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::render::camera_data::CameraData;

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

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReflectType {
    Diffuse,
    Specular,
    Refract,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub color: Vec3,
    pub emmission: Vec3,
    pub reflect_type: ReflectType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneDescriptor {
    pub id: String,
    pub objects: Vec<SceneObjectDescriptor>,
    pub camera: CameraData,
}

impl SceneDescriptor {
    pub fn load(id: &str) -> std::io::Result<SceneDescriptor> {
        let filename = format!("scenes/{}.json", id);
        let json = std::fs::read_to_string(filename)?;
        let scene: SceneDescriptor = serde_json::from_str(&json).unwrap();
        Ok(scene)
    }

    pub fn to_data(self) -> SceneData {
        SceneData {
            id: self.id,
            objects: self
                .objects
                .into_iter()
                .map(SceneObjectDescriptor::to_scene_object)
                .collect(),
            camera: self.camera,
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self).unwrap();
        let filename = format!("scenes/{}.json", self.id);
        std::fs::write(filename, json)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SceneData {
    pub id: String,
    pub objects: Vec<SceneObjectData>,
    pub camera: CameraData,
}
impl SceneData {
    pub fn to_descriptor(&self) -> SceneDescriptor {
        SceneDescriptor {
            id: self.id.clone(),
            objects: self
                .objects
                .iter()
                .map(|obj| SceneObjectDescriptor {
                    type_: match &obj.type_ {
                        SceneObject::Sphere { radius } => {
                            SceneObjectDescriptorType::Sphere { radius: *radius }
                        }
                        SceneObject::Mesh { mesh, file } => match file {
                            Some(file) => SceneObjectDescriptorType::MeshFile(file.clone()),
                            None => SceneObjectDescriptorType::Mesh(mesh.clone()),
                        },
                    },
                    position: obj.position,
                    material: obj.material.clone(),
                })
                .collect(),
            camera: self.camera.clone(),
        }
    }
}

impl Display for SceneData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.id)
    }
}

pub mod camera_data {
    use glam::Vec3;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CameraData {
        pub position: Vec3,

        /// normal to sensor plane
        direction: Vec3,
        updating_direction: Option<Vec3>,

        /// in meters
        pub focal_length: f32,

        /// in meters
        pub sensor_width: f32,

        pub aspect_ratio: f32,
    }
    impl CameraData {
        pub fn new(position: Vec3, direction: Vec3) -> Self {
            Self {
                position,
                direction: direction.normalize(),
                updating_direction: None,
                focal_length: 0.035,
                sensor_width: 0.036,
                aspect_ratio: 3.0 / 2.0,
            }
        }

        pub fn direction(&self) -> Vec3 {
            self.direction
        }
        pub fn set_direction(&mut self, direction: Vec3) {
            self.direction = direction.normalize();
            self.updating_direction = None;
        }

        pub fn get_current_direction(&self) -> Vec3 {
            self.updating_direction.unwrap_or(self.direction)
        }
        pub fn set_updating_direction(&mut self, direction: Vec3) {
            self.updating_direction = Some(direction.normalize());
            // println!("Current direction: {:?}", self.get_current_direction());
        }

        pub fn sensor_height(&self) -> f32 {
            self.sensor_width / self.aspect_ratio
        }

        /// lens center (pinhole)
        pub fn lens_center(&self) -> Vec3 {
            self.position + self.get_current_direction() * self.focal_length
        }

        /// Returns (su, sv), two orthogonal vectors spanning the sensor plane, scaled by the sensor dimensions.
        pub fn orthogonals(&self) -> (Vec3, Vec3) {
            let direction = self.get_current_direction();
            let su = direction
                .cross(if direction.y.abs() < 0.9 {
                    Vec3::new(0.0, 1.0, 0.0)
                } else {
                    Vec3::new(0.0, 0.0, 1.0)
                })
                .normalize();
            let sv = su.cross(direction);
            return (su * self.sensor_width, sv * self.sensor_height());
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneObjectDescriptor {
    pub type_: SceneObjectDescriptorType,
    pub position: Vec3,
    pub material: Material,
}

impl SceneObjectDescriptor {
    fn to_scene_object(self) -> SceneObjectData {
        SceneObjectData {
            type_: self.type_.to_scene_object(),
            position: self.position,
            material: self.material,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SceneObjectData {
    pub type_: SceneObject,
    pub position: Vec3,
    pub material: Material,
}

impl SceneObjectData {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        return match &self.type_ {
            SceneObject::Sphere { radius } => intersect_sphere(self.position, *radius, ray),

            SceneObject::Mesh { mesh, file: _ } => {
                // Performance: first test against bounding sphere
                if intersect_sphere(
                    mesh.bounding_sphere.position + self.position,
                    mesh.bounding_sphere.radius,
                    ray,
                )
                .is_some()
                {
                    // Initialize variables to track closest hit
                    let mut closest_hit: Option<Hit> = None;

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

                        let distance: f32 = va_vc.dot(qvec) * inv_determinant;

                        // Skip negative distances (hits behind the ray origin)
                        if distance <= 0.0 {
                            continue;
                        }

                        // Only update if this hit is closer than the previous closest
                        let is_closest_hit = match &closest_hit {
                            Some(hit) => distance < hit.distance,
                            None => true,
                        };

                        if is_closest_hit {
                            // Calculate proper intersection point using ray equation
                            let intersection = ray.origin + ray.direction * distance;
                            let normal = va_vb.cross(va_vc).normalize();

                            closest_hit = Some(Hit {
                                distance,
                                intersection,
                                normal,
                            });
                        }
                    }

                    // Return the closest hit, or NoHit if none was found
                    closest_hit
                } else {
                    None
                }
            }
        };
    }

    pub fn to_triangles(&self) -> Vec<Triangle> {
        return self.type_.to_triangles();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SceneObjectDescriptorType {
    Sphere { radius: f32 },
    MeshFile(MeshFileDescriptor),
    Mesh(Mesh),
}

impl SceneObjectDescriptorType {
    pub fn to_scene_object(self) -> SceneObject {
        match self {
            SceneObjectDescriptorType::Sphere { radius } => SceneObject::Sphere { radius },
            SceneObjectDescriptorType::MeshFile(mesh_file) => {
                let mesh = load_off::load_off(&mesh_file.path, mesh_file.scale).unwrap();
                SceneObject::Mesh {
                    mesh,
                    file: Some(mesh_file),
                }
            }
            SceneObjectDescriptorType::Mesh(mesh) => SceneObject::Mesh { mesh, file: None },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshFileDescriptor {
    path: String,
    scale: f32,
}

#[derive(Clone, Debug)]
pub enum SceneObject {
    Sphere {
        radius: f32,
    },
    Mesh {
        mesh: Mesh,
        file: Option<MeshFileDescriptor>,
    },
}

impl SceneObject {
    fn to_triangles(&self) -> Vec<Triangle> {
        match self {
            SceneObject::Sphere { radius } => sphere_to_triangles(*radius),
            SceneObject::Mesh { mesh, file: _ } => mesh.triangles.clone(),
        }
    }
}

fn sphere_to_triangles(radius: f32) -> Vec<Triangle> {
    let mut triangles: Vec<Triangle> = vec![];
    let steps = 16;
    for i in 0..steps {
        let theta1 = PI * (i as f32) / (steps as f32);
        let theta2 = PI * ((i + 1) as f32) / (steps as f32);
        for j in 0..(steps * 2) {
            let phi1 = 2.0 * PI * (j as f32) / ((steps * 2) as f32);
            let phi2 = 2.0 * PI * ((j + 1) as f32) / ((steps * 2) as f32);

            let p1 = Vec3::new(
                radius * theta1.sin() * phi1.cos(),
                radius * theta1.cos(),
                radius * theta1.sin() * phi1.sin(),
            );
            let p2 = Vec3::new(
                radius * theta2.sin() * phi1.cos(),
                radius * theta2.cos(),
                radius * theta2.sin() * phi1.sin(),
            );
            let p3 = Vec3::new(
                radius * theta2.sin() * phi2.cos(),
                radius * theta2.cos(),
                radius * theta2.sin() * phi2.sin(),
            );
            let p4 = Vec3::new(
                radius * theta1.sin() * phi2.cos(),
                radius * theta1.cos(),
                radius * theta1.sin() * phi2.sin(),
            );

            if i == 0 {
                triangles.push(Triangle {
                    a: p1,
                    b: p3,
                    c: p4,
                });
            } else if i + 1 == steps {
                triangles.push(Triangle {
                    a: p1,
                    b: p2,
                    c: p3,
                });
            } else {
                triangles.push(Triangle {
                    a: p1,
                    b: p2,
                    c: p4,
                });
                triangles.push(Triangle {
                    a: p2,
                    b: p3,
                    c: p4,
                });
            }
        }
    }
    return triangles;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StandaloneSphere {
    position: Vec3,
    radius: f32,
}

fn intersect_sphere(position: Vec3, radius: f32, ray: &Ray) -> Option<Hit> {
    let op: Vec3 = position - ray.origin;
    let eps: f32 = 1e-4;
    let b = op.dot(ray.direction);
    let mut det = b.powi(2) - op.dot(op) + radius.powi(2);
    if det < 0.0 {
        return None;
    } else {
        det = det.sqrt();
    }
    let t = if b - det >= eps {
        b - det
    } else if b + det >= eps {
        b + det
    } else {
        return None;
    };

    let xmin = ray.origin + ray.direction * t;
    let nmin = (xmin - position).normalize();

    return Some(Hit {
        distance: t,
        intersection: xmin,
        normal: nmin,
    });
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mesh {
    triangles: Vec<Triangle>,

    // TODO
    // these fields can be computed instead of serialize + deserialize. Maybe use some `lazy` thing
    bounding_sphere: StandaloneSphere,
    bounding_box: Vec<Triangle>,
}

impl Mesh {
    fn new(triangles: Vec<Triangle>) -> Mesh {
        let mut min_vert = Vec3::splat(f32::INFINITY);
        let mut max_vert = Vec3::splat(f32::NEG_INFINITY);

        for tri in triangles.iter() {
            for vert in [&tri.a, &tri.b, &tri.c] {
                if vert.x < min_vert.x {
                    min_vert.x = vert.x;
                }
                if vert.y < min_vert.y {
                    min_vert.y = vert.y;
                }
                if vert.z < min_vert.z {
                    min_vert.z = vert.z;
                }

                if vert.x > max_vert.x {
                    max_vert.x = vert.x;
                }
                if vert.y > max_vert.y {
                    max_vert.y = vert.y;
                }
                if vert.z > max_vert.z {
                    max_vert.z = vert.z;
                }
            }
        }
        let bounding_sphere_pos = Vec3 {
            x: min_vert.x + max_vert.x * 0.5,
            y: min_vert.y + max_vert.y * 0.5,
            z: min_vert.z + max_vert.z * 0.5,
        };
        let bounding_sphere = StandaloneSphere {
            position: bounding_sphere_pos,
            radius: *vec![
                (min_vert - bounding_sphere_pos).length(),
                (max_vert - bounding_sphere_pos).length(),
            ]
            .iter()
            .max_by(|p1, p2| p1.partial_cmp(&p2).unwrap())
            .unwrap(),
        };
        Mesh {
            triangles,
            bounding_sphere,
            bounding_box: bounding_box_to_triangles((min_vert, max_vert)),
        }
    }
}

fn bounding_box_to_triangles(bounds: (Vec3, Vec3)) -> Vec<Triangle> {
    let (min, max) = bounds;
    let vertices = vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    let indices = vec![
        (0, 1, 2),
        (0, 2, 3), // front
        (4, 6, 5),
        (4, 7, 6), // back
        (0, 4, 5),
        (0, 5, 1), // bottom
        (3, 2, 6),
        (3, 6, 7), // top
        (1, 5, 6),
        (1, 6, 2), // right
        (0, 3, 7),
        (0, 7, 4), // left
    ];
    let mut triangles = vec![];
    for (i1, i2, i3) in indices {
        triangles.push(Triangle {
            a: vertices[i1],
            b: vertices[i2],
            c: vertices[i3],
        });
    }
    return triangles;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    pub fn transformed(&self, v: &Vec3) -> Triangle {
        Triangle {
            a: self.a + *v,
            b: self.b + *v,
            c: self.c + *v,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Hit {
    pub distance: f32,
    intersection: Vec3,
    normal: Vec3,
}

#[derive(PartialEq, Debug)]
pub struct SceneIntersectResult {
    pub object_id: usize,
    pub hit: Hit,
}

pub fn intersect_scene(
    ray: &Ray,
    scene_objects: &Vec<SceneObjectData>,
) -> Option<SceneIntersectResult> {
    let mut min_intersect: Option<SceneIntersectResult> = None;

    for i in (0..scene_objects.len()).rev() {
        let scene_object = &scene_objects[i];
        let intersect = scene_object.intersect(ray);
        match (intersect, &min_intersect) {
            (None, _) => (),
            (Some(new_hit), None) => {
                min_intersect = Some(SceneIntersectResult {
                    object_id: i,
                    hit: new_hit,
                });
            }
            (Some(new_hit), Some(SceneIntersectResult { hit, .. })) => {
                if new_hit.distance < hit.distance {
                    min_intersect = Some(SceneIntersectResult {
                        object_id: i,
                        hit: new_hit,
                    });
                }
            }
        }
    }
    return min_intersect;
}

const MAX_DEPTH: usize = 12;
fn radiance(ray: &Ray, depth: usize, scene_objects: &Vec<SceneObjectData>) -> Vec3 {
    return match intersect_scene(&ray, scene_objects) {
        None => Vec3::default(),
        Some(SceneIntersectResult { object_id, hit }) => {
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

fn render_pixel(
    config: &RenderConfig,
    pixel_index: usize,
    orthogonals: (Vec3, Vec3),
    lens_center: Vec3,
    processed_pixel_count: &Arc<atomic::AtomicUsize>,
) -> Vec3 {
    let res = &config.resolution;
    let scene_objects = &config.scene.objects;
    let sensor_origin = &config.scene.camera.position;

    let y = res.height - 1 - pixel_index / res.width;
    let x = pixel_index % res.width;

    let (su, sv) = orthogonals;

    let mut radiance_v: Vec3 = Vec3::default();

    for s in 0..config.samples_per_pixel {
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
        let sx: f32 = (x as f32 + 0.5 * (0.5 + xsub + xfilter)) / res.width as f32 - 0.5;
        let sy: f32 = (y as f32 + 0.5 * (0.5 + ysub + yfilter)) / res.height as f32 - 0.5;

        // 3d sample position on sensor
        let sensor_pos = sensor_origin + su * sx + sv * sy;
        let ray_direction = (lens_center - sensor_pos).normalize();
        // ray through pinhole
        let ray = Ray {
            origin: lens_center,
            direction: ray_direction,
        };

        // evaluate radiance from this ray and accumulate
        radiance_v = radiance_v + radiance(&ray, 0, scene_objects);
    }
    // normalize radiance by number of samples
    radiance_v = radiance_v / config.samples_per_pixel as f32;
    processed_pixel_count.fetch_add(1, atomic::Ordering::Relaxed);

    Vec3::new(
        radiance_v.x.clamp(0.0, 1.0),
        radiance_v.y.clamp(0.0, 1.0),
        radiance_v.z.clamp(0.0, 1.0),
    )
}

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub samples_per_pixel: usize,
    pub resolution: Resolution,
    pub scene: SceneData,
}

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub height: usize,
    pub width: usize,
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            height: 300,
            width: 300 * 3 / 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderUpdate {
    pub progress: f32,
    pub image: Image,
}

#[derive(Debug, Clone)]
pub struct RenderDone {
    pub image: Image,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub pixels: Vec<Vec3>,
    pub resolution: Resolution,
    pub hash: u64,
}
impl Image {
    fn new(pixels: Vec<Vec3>, resolution: Resolution) -> Self {
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
    send_update_progress: &mut (
             impl Sink<RenderUpdate, Error = SendError> + Unpin + Clone + Sync + Send
         ),
    cancel_render: Arc<AtomicBool>,
) -> RenderDone {
    thread::scope(move |s| {
        let time_start = Instant::now();

        let res = render_config.resolution;
        let grid_size = res.width * res.height;
        let pixels = Arc::new(std::sync::Mutex::new(vec![Vec3::default(); grid_size]));
        let get_pixels = pixels.clone();

        let stop_render = Arc::new(AtomicBool::new(false));

        // Start background thread that listens for stop signal
        let background_stop_render = stop_render.clone();
        s.spawn(move || {
            loop {
                if cancel_render.load(atomic::Ordering::Relaxed) == true {
                    println!("Canceling render prematurely");
                    background_stop_render.store(true, atomic::Ordering::Relaxed);
                }
                if background_stop_render.load(atomic::Ordering::Relaxed) == true {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        let processed_pixel_count = Arc::new(atomic::AtomicUsize::new(0));
        let get_processed_pixel_count = processed_pixel_count.clone();

        // Start background thread that sends regular progress updates
        let background_stop_render = stop_render.clone();
        s.spawn(move || {
            loop {
                if background_stop_render.load(atomic::Ordering::Relaxed) == true {
                    println!("Stopping background thread");
                    break;
                }

                let processed_percentage = get_processed_pixel_count.load(atomic::Ordering::Relaxed)
                    as f32
                    / (grid_size) as f32;
                let _ = futures::executor::block_on(send_update_progress.send(RenderUpdate {
                    progress: processed_percentage,
                    image: Image::new(get_pixels.lock().unwrap().clone(), res),
                }));

                std::thread::sleep(Duration::from_millis(500));
            }
        });

        let render_thread_handle = s.spawn(move || {
            let scene = &render_config.scene;

            println!(
                "Rendering scene {} ({} objects), {} samples per pixel, {}x{} resolution{}",
                scene.id,
                scene.objects.len(),
                render_config.samples_per_pixel,
                res.width,
                res.height,
                if MOCK_RANDOM { " (mock random)" } else { "" }
            );

            //-- setup sensor
            let lens_center = scene.camera.lens_center();
            let orthogonals = scene.camera.orthogonals();

            let render_pixel_to_vec = |pixel_index: usize| {
                let stop_render = stop_render.clone();
                if stop_render.load(atomic::Ordering::Relaxed) == true {
                    return;
                }
                let pixel_value = render_pixel(
                    &render_config,
                    pixel_index,
                    orthogonals,
                    lens_center,
                    &processed_pixel_count,
                );
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
            println!("Rendering complete");
            stop_render.store(true, atomic::Ordering::Relaxed);

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
                res.height,
            );
            let mut file = std::fs::File::create(path.clone()).unwrap();
            file.write_all(b"P3\n").unwrap();
            file.write_all(
                format!(
                    "# samplesPerPixel: {}, resolution_y: {}, scene_id: {}\n",
                    render_config.samples_per_pixel, res.height, scene.id
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
            file.write_all(format!("{} {}\n{}\n", res.width, res.height, 255).as_bytes())
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

            return Image::new(pixels, res);
        });

        let image = render_thread_handle.join().unwrap();
        RenderDone {
            image,
            duration: time_start.elapsed(),
        }
    })
}
