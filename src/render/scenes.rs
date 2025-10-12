use super::{
    load_off::load_off, CameraData, Material, ReflectType, SceneData, SceneObject, SceneObjectData,
    Vector,
};

pub fn load_scenes() -> Vec<SceneData> {
    // Set up scene
    const BOX_DIMENSIONS: Vector = Vector {
        x: 2.6,
        y: 2.0,
        z: 2.8,
    };

    let cornell_box = vec![
        // Cornell Box centered in the origin (0, 0, 0)
        // Left
        SceneObjectData {
            position: Vector::from(-1e5 - BOX_DIMENSIONS.x, 0.0, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vector::from(0.85, 0.25, 0.25),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Right
        SceneObjectData {
            position: Vector::from(1e5 + BOX_DIMENSIONS.x, 0.0, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vector::from(0.25, 0.35, 0.85),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Top
        SceneObjectData {
            position: Vector::from(0.0, 1e5 + BOX_DIMENSIONS.y, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vector::from(0.75, 0.75, 0.75),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Bottom
        SceneObjectData {
            position: Vector::from(0.0, -1e5 - BOX_DIMENSIONS.y, 0.0),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vector::from(0.75, 0.75, 0.75),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Back
        SceneObjectData {
            position: Vector::from(0.0, 0.0, -1e5 - BOX_DIMENSIONS.z),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vector::from(0.75, 0.75, 0.75),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Front
        SceneObjectData {
            position: Vector::from(0.0, 0.0, 1e5 + 3.0 * BOX_DIMENSIONS.z - 0.5),
            type_: SceneObject::Sphere { radius: 1e5 },
            material: Material {
                color: Vector::zero(),
                emmission: Vector::zero(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // The ceiling area light source (slightly yellowish color)
        SceneObjectData {
            position: Vector::from(0.0, BOX_DIMENSIONS.y + 10.0 - 0.04, 0.0),
            type_: SceneObject::Sphere { radius: 10.0 },
            material: Material {
                color: Vector::zero(),
                // emmission: Vector::from(0.98 * 2.0, 2.0, 0.9 * 2.0),
                emmission: Vector::from(0.98, 1.0, 0.9) * 15.0,
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let default_camera = CameraData {
        position: Vector::from(0.0, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0),
        direction: Vector::from(0.0, -0.06, -1.0),
        focal_length: 0.035,
    };

    // scene_id to scene_objects
    return vec![
        SceneData {
            id: "single-sphere".to_owned(),
            objects: vec![SceneObjectData {
                position: Vector::from(0.0, 0.0, 0.0),
                type_: SceneObject::Sphere { radius: 1.0 },
                material: Material {
                    color: Vector::from(1.0, 1.0, 1.0),
                    emmission: Vector::from(0.98 * 15.0, 15.0, 0.9 * 15.0),
                    reflect_type: ReflectType::Diffuse,
                },
            }],
            camera: default_camera,
        },
        SceneData {
            id: "two-spheres".to_owned(),
            objects: vec![
                SceneObjectData {
                    position: Vector::from(0.0, 0.0, 0.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vector::from(1.0, 0.0, 0.0),
                        emmission: Vector::from(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectData {
                    position: Vector::from(0.0, 0.0, 10.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vector::from(0.0, 0.0, 0.0),
                        emmission: Vector::uniform(10.0),
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
                    position: Vector::from(0.0, 0.0, -3.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vector::from(1.0, 0.2, 0.2),
                        emmission: Vector::from(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectData {
                    position: Vector::from(4.0, 2.0, 0.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vector::from(0.0, 0.0, 0.0),
                        emmission: Vector::from(20.0, 10.0, 10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectData {
                    position: Vector::from(-6.0, -2.0, 0.0),
                    type_: SceneObject::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vector::from(0.0, 0.0, 0.0),
                        emmission: Vector::from(5.0, 9.0, 20.0),
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
                    position: Vector::from(-1.3, -BOX_DIMENSIONS.y + 0.8, -1.3),
                    material: Material {
                        color: Vector::uniform(0.999),
                        emmission: Vector::zero(),
                        reflect_type: ReflectType::Specular,
                    },
                },
                // refracting
                SceneObjectData {
                    type_: SceneObject::Sphere { radius: 0.8 },
                    position: Vector::from(1.3, -BOX_DIMENSIONS.y + 0.8, -0.2),
                    material: Material {
                        color: Vector::uniform(0.999),
                        emmission: Vector::zero(),
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
                position: Vector::from(-0.8, -BOX_DIMENSIONS.y + 0.5, 0.0),
                type_: SceneObject::Mesh(load_off("meshes/mctri.off", 0.16).unwrap()),
                material: Material {
                    color: Vector::from(234.0 / 255.0, 1.0, 0.0),
                    emmission: Vector::zero(),
                    reflect_type: ReflectType::Diffuse,
                },
            }]
            .into_iter()
            .chain(cornell_box.clone())
            .collect(),
            camera: CameraData {
                position: Vector::from(0.9, 0.26 * BOX_DIMENSIONS.y, 3.0 * BOX_DIMENSIONS.z - 1.0),
                direction: Vector::from(-0.09, -0.06, -1.0),
                focal_length: 0.035,
            },
        },
    ];
}
