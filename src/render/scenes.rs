use glam::Vec3;

use crate::render::{Mesh, Triangle, camera_data::CameraData};

use super::{
    Material, ReflectType, SceneData, SceneObject, SceneObjectData, camera_data, load_off::load_off,
};

// Helper function to create a quad (rectangle) from two triangles
fn single_quad_mesh(size: f32, axis: usize, flip: bool) -> Mesh {
    // Create a quad along the specified axis (0=X, 1=Y, 2=Z)
    // with the normal pointing in the positive direction
    // flip=true will make normal point in negative direction

    let half_size = size / 2.0;
    let mut vertices = Vec::with_capacity(4);

    for i in 0..2 {
        for j in 0..2 {
            let mut pos = [0.0, 0.0, 0.0];
            let idx1 = (axis + 1) % 3;
            let idx2 = (axis + 2) % 3;
            pos[idx1] = if i == 0 { -half_size } else { half_size };
            pos[idx2] = if j == 0 { -half_size } else { half_size };

            vertices.push(Vec3::new(pos[0], pos[1], pos[2]));
        }
    }

    // Create two triangles from the four vertices
    let mut triangles = Vec::with_capacity(2);
    if flip {
        triangles.push(Triangle {
            a: vertices[0],
            b: vertices[1],
            c: vertices[2],
        });
        triangles.push(Triangle {
            a: vertices[2],
            b: vertices[1],
            c: vertices[3],
        });
    } else {
        triangles.push(Triangle {
            a: vertices[0],
            b: vertices[2],
            c: vertices[1],
        });
        triangles.push(Triangle {
            a: vertices[1],
            b: vertices[2],
            c: vertices[3],
        });
    }

    Mesh::new(triangles)
}

pub fn load_scenes() -> Vec<SceneData> {
    // Set up scene
    const BOX_DIMENSIONS: Vec3 = Vec3 {
        x: 2.6,
        y: 2.0,
        z: 2.8,
    };

    const WALL_SIZE: f32 = 15.0; // Size of the quad walls

    let cornell_box = vec![
        // Cornell Box centered in the origin (0, 0, 0)
        // Left wall - Red
        SceneObjectData {
            position: Vec3::new(-BOX_DIMENSIONS.x, 0.0, 0.0),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 0, false)),
            material: Material {
                color: Vec3::new(0.85, 0.25, 0.25),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Right wall - Blue
        SceneObjectData {
            position: Vec3::new(BOX_DIMENSIONS.x, 0.0, 0.0),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 0, true)),
            material: Material {
                color: Vec3::new(0.25, 0.35, 0.85),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Top wall - White
        SceneObjectData {
            position: Vec3::new(0.0, BOX_DIMENSIONS.y, 0.0),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 1, true)),
            material: Material {
                color: Vec3::splat(0.8),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Bottom wall - White
        SceneObjectData {
            position: Vec3::new(0.0, -BOX_DIMENSIONS.y, 0.0),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 1, false)),
            material: Material {
                color: Vec3::splat(0.7),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Back wall - White
        SceneObjectData {
            position: Vec3::new(0.0, 0.0, -BOX_DIMENSIONS.z),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 2, false)),
            material: Material {
                color: Vec3::splat(0.75),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Front wall - Invisible/Black
        SceneObjectData {
            position: Vec3::new(0.0, 0.0, 3.0 * BOX_DIMENSIONS.z - 0.5),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 2, true)),
            material: Material {
                color: Vec3::default(),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // The ceiling area light source (slightly yellowish color)
        SceneObjectData {
            position: Vec3::new(0.0, BOX_DIMENSIONS.y + 10.0 - 0.04, 0.0),
            type_: SceneObject::Mesh(single_quad_mesh(WALL_SIZE, 1, true)),
            material: Material {
                color: Vec3::new(0.98, 1.0, 0.9),
                emmission: Vec3::new(0.98, 1.0, 0.9) * 15.0,
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let default_camera = CameraData::new(
        Vec3::new(0.0, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0),
        Vec3::new(0.0, -0.06, -1.0),
    );

    // scene_id to scene_objects
    return vec![
        SceneData {
            id: "single-sphere".to_owned(),
            objects: vec![SceneObjectData {
                position: Vec3::new(0.0, 0.0, 0.0),
                type_: SceneObject::Sphere { radius: 1.0 },
                material: Material {
                    color: Vec3::new(1.0, 1.0, 1.0),
                    emmission: Vec3::new(0.98 * 15.0, 15.0, 0.9 * 15.0),
                    reflect_type: ReflectType::Diffuse,
                },
            }],
            camera: default_camera.clone(),
        },
        SceneData {
            id: "two-spheres".to_owned(),
            objects: vec![
                SceneObjectData {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(1.0, 0.0, 0.0),
                        emmission: Vec3::new(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectData {
                    position: Vec3::new(0.0, 0.0, 10.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.0),
                        emmission: Vec3::splat(10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
            camera: default_camera.clone(),
        },
        SceneData {
            id: "three-spheres".to_owned(),
            objects: vec![
                SceneObjectData {
                    position: Vec3::new(0.0, 0.0, -3.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(1.0, 0.2, 0.2),
                        emmission: Vec3::new(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectData {
                    position: Vec3::new(4.0, 2.0, 0.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.0),
                        emmission: Vec3::new(20.0, 10.0, 10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectData {
                    position: Vec3::new(-6.0, -2.0, 0.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.0),
                        emmission: Vec3::new(5.0, 9.0, 20.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
            camera: default_camera.clone(),
        },
        SceneData {
            id: "cornell".to_owned(),
            objects: vec![
                // Objects
                // mirroring
                SceneObjectData {
                    type_: SceneObject::Sphere { radius: 0.8 },
                    position: Vec3::new(-1.3, -BOX_DIMENSIONS.y + 0.8, -1.3),
                    material: Material {
                        color: Vec3::splat(0.999),
                        emmission: Vec3::default(),
                        reflect_type: ReflectType::Specular,
                    },
                },
                // refracting
                SceneObjectData {
                    type_: SceneObject::Sphere { radius: 0.8 },
                    position: Vec3::new(1.3, -BOX_DIMENSIONS.y + 0.8, -0.2),
                    material: Material {
                        color: Vec3::splat(0.999),
                        emmission: Vec3::default(),
                        reflect_type: ReflectType::Refract,
                    },
                },
                // emmission
                SceneObjectData {
                    type_: SceneObject::Sphere { radius: 0.5 },
                    position: Vec3::new(0.08, -BOX_DIMENSIONS.y + 0.8, -0.8),
                    material: Material {
                        color: Vec3::splat(0.999),
                        emmission: Vec3::new(0.98, 1.0, 0.9) * 2.0,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ]
            .into_iter()
            .chain(cornell_box.clone())
            .collect(),
            camera: default_camera.clone(),
        },
        SceneData {
            id: "mesh".to_owned(),
            objects: vec![SceneObjectData {
                position: Vec3::new(-0.8, -BOX_DIMENSIONS.y + 0.5, 0.0),
                type_: SceneObject::Mesh(load_off("meshes/mctri.off", 0.16).unwrap()),
                material: Material {
                    color: Vec3::new(234.0 / 255.0, 1.0, 0.0),
                    emmission: Vec3::default(),
                    // emmission: Vec3::new(0.98, 1.0, 0.9) * 15.0,
                    reflect_type: ReflectType::Diffuse,
                },
            }]
            .into_iter()
            .chain(cornell_box.clone())
            .collect(),
            camera: CameraData::new(
                Vec3::new(0.9, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0),
                Vec3::new(-0.09, -0.06, -1.0),
            ),
        },
    ];
}
