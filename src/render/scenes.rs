use glam::Vec3;

use super::{
    load_off::load_off, CameraData, Material, ReflectType, SceneData, SceneObject, SceneObjectData,
};

pub fn load_scenes() -> Vec<SceneData> {
    // Set up scene
    const BOX_DIMENSIONS: Vec3 = Vec3 {
        x: 2.6,
        y: 2.0,
        z: 2.8,
    };

    let cornell_box = vec![
        // Cornell Box centered in the origin (0, 0, 0)
        // Left
        SceneObjectData {
            position: Vec3::new(-1e5 - BOX_DIMENSIONS.x, 0.0, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vec3::new(0.85, 0.25, 0.25),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Right
        SceneObjectData {
            position: Vec3::new(1e5 + BOX_DIMENSIONS.x, 0.0, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vec3::new(0.25, 0.35, 0.85),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Top
        SceneObjectData {
            position: Vec3::new(0.0, 1e5 + BOX_DIMENSIONS.y, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vec3::new(0.75, 0.75, 0.75),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Bottom
        SceneObjectData {
            position: Vec3::new(0.0, -1e5 - BOX_DIMENSIONS.y, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vec3::new(0.75, 0.75, 0.75),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Back
        SceneObjectData {
            position: Vec3::new(0.0, 0.0, -1e5 - BOX_DIMENSIONS.z),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vec3::new(0.75, 0.75, 0.75),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Front
        SceneObjectData {
            position: Vec3::new(0.0, 0.0, 1e5 + 3.0 * BOX_DIMENSIONS.z - 0.5),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vec3::default(),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // The ceiling area light source (slightly yellowish color)
        SceneObjectData {
            position: Vec3::new(0.0, BOX_DIMENSIONS.y + 10.0 - 0.04, 0.0),
            type_: SceneObject::Sphere { radius: 10.0 },
            material: Material {
                color: Vec3::default(),
                // emmission: Vector::from(0.98 * 2.0, 2.0, 0.9 * 2.0),
                emmission: Vec3::new(0.98, 1.0, 0.9) * 15.0,
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let default_camera = CameraData {
        position: Vec3::new(0.0, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0),
        direction: Vec3::new(0.0, -0.06, -1.0),
        focal_length: 0.035,
    };

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
            camera: default_camera,
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
            camera: default_camera,
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
            camera: default_camera,
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
            ]
            .into_iter()
            .chain(cornell_box.clone())
            .collect(),
            camera: default_camera,
        },
        SceneData {
            id: "mesh".to_owned(),
            objects: vec![SceneObjectData {
                position: Vec3::new(-0.8, -BOX_DIMENSIONS.y + 0.5, 0.0),
                type_: SceneObject::Mesh(load_off("meshes/mctri.off", 0.16).unwrap()),
                material: Material {
                    color: Vec3::new(234.0 / 255.0, 1.0, 0.0),
                    emmission: Vec3::default(),
                    reflect_type: ReflectType::Diffuse,
                },
            }]
            .into_iter()
            .chain(cornell_box.clone())
            .collect(),
            camera: CameraData {
                position: Vec3::new(0.9, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0),
                direction: Vec3::new(-0.09, -0.06, -1.0),
                focal_length: 0.035,
            },
        },
    ];
}
