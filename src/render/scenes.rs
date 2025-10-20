use glam::Vec3;

use crate::render::{
    Mesh, MeshFileDescriptor, SceneDescriptor, SceneObjectDescriptor, SceneObjectDescriptorType,
    Triangle, camera_data::CameraData,
};

use super::{Material, ReflectType};

pub fn load_scene_ids() -> Vec<String> {
    let mut scene_ids = std::fs::read_dir("scenes")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        if let Some(stem) = path.file_stem() {
                            return stem.to_str().map(|s| s.to_owned());
                        }
                    }
                }
            }
            None
        })
        .collect::<Vec<String>>();

    if scene_ids.is_empty() {
        // If no scenes were loaded, fall back to hardcoded scenes
        let scenes = setup_scenes();
        for scene in &scenes {
            if let Err(err) = scene.save() {
                eprintln!("Failed to save scene '{}': {}", scene.id, err);
            }
        }
        scene_ids = scenes.into_iter().map(|s| s.id).collect();
    }

    return scene_ids;
}

fn setup_scenes() -> Vec<SceneDescriptor> {
    // Set up scene
    const BOX: Vec3 = Vec3 {
        x: 2.6,
        y: 2.0,
        z: 8.8,
    };

    let cornell_box = vec![
        // Cornell Box centered in the origin (0, 0, 0)
        // Right wall - Red
        SceneObjectDescriptor {
            position: Vec3::new(BOX.x, 0.0, 0.0),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.y, BOX.z, 0, true)),
            material: Material {
                color: Vec3::new(0.85, 0.25, 0.25),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Left wall - Blue
        SceneObjectDescriptor {
            position: Vec3::new(-BOX.x, 0.0, 0.0),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.y, BOX.z, 0, false)),
            material: Material {
                color: Vec3::new(0.25, 0.35, 0.85),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Top wall - White
        SceneObjectDescriptor {
            position: Vec3::new(0.0, BOX.y, 0.0),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.z, BOX.x, 1, true)),
            material: Material {
                color: Vec3::splat(0.8),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Bottom wall - White
        SceneObjectDescriptor {
            position: Vec3::new(0.0, -BOX.y, 0.0),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.z, BOX.x, 1, false)),
            material: Material {
                color: Vec3::splat(0.7),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Back wall - White
        SceneObjectDescriptor {
            position: Vec3::new(0.0, 0.0, -BOX.z),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.x, BOX.y, 2, true)),
            material: Material {
                color: Vec3::splat(0.95),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // Front wall - Invisible/Black
        SceneObjectDescriptor {
            position: Vec3::new(0.0, 0.0, BOX.z),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.x, BOX.y, 2, true)),
            material: Material {
                color: Vec3::splat(0.05),
                emmission: Vec3::default(),
                reflect_type: ReflectType::Diffuse,
            },
        },
        // The ceiling area light source (slightly yellowish color)
        SceneObjectDescriptor {
            position: Vec3::new(0.0, BOX.y - 0.04, 0.0),
            type_: SceneObjectDescriptorType::Mesh(single_quad_mesh(BOX.z, BOX.x, 1, true)),
            material: Material {
                color: Vec3::new(0.98, 1.0, 0.9),
                emmission: Vec3::new(0.98, 1.0, 0.9) * 0.9,
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let default_camera = CameraData::new(
        Vec3::new(0.0, -BOX.y + 1.8, BOX.z - 1.0),
        Vec3::new(0.0, -0.06, -1.0),
    );

    // scene_id to scene_objects
    return vec![
        SceneDescriptor {
            id: "single-sphere".to_owned(),
            objects: vec![SceneObjectDescriptor {
                position: Vec3::new(0.0, 0.0, 0.0),
                type_: SceneObjectDescriptorType::Sphere { radius: 1.0 },
                material: Material {
                    color: Vec3::new(1.0, 1.0, 1.0),
                    emmission: Vec3::new(0.98 * 15.0, 15.0, 0.9 * 15.0),
                    reflect_type: ReflectType::Diffuse,
                },
            }],
            camera: default_camera.clone(),
        },
        SceneDescriptor {
            id: "cartesian".to_owned(),
            objects: vec![
                SceneObjectDescriptor {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.3 },
                    material: Material {
                        color: Vec3::new(0.9, 0.9, 0.9),
                        emmission: Vec3::ZERO,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectDescriptor {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.3 },
                    material: Material {
                        color: Vec3::new(0.8, 0.0, 0.0),
                        emmission: Vec3::ZERO,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectDescriptor {
                    position: Vec3::new(-1.0, 0.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.3 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.8),
                        emmission: Vec3::ZERO,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectDescriptor {
                    position: Vec3::new(0.0, 1.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.3 },
                    material: Material {
                        color: Vec3::new(0.0, 0.8, 0.0),
                        emmission: Vec3::ZERO,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
            camera: default_camera.clone(),
        },
        SceneDescriptor {
            id: "two-spheres".to_owned(),
            objects: vec![
                SceneObjectDescriptor {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(1.0, 0.0, 0.0),
                        emmission: Vec3::new(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectDescriptor {
                    position: Vec3::new(0.0, 0.0, 10.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.0),
                        emmission: Vec3::splat(10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
            camera: default_camera.clone(),
        },
        SceneDescriptor {
            id: "three-spheres".to_owned(),
            objects: vec![
                SceneObjectDescriptor {
                    position: Vec3::new(0.0, 0.0, -3.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(1.0, 0.2, 0.2),
                        emmission: Vec3::new(0.0, 0.0, 0.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectDescriptor {
                    position: Vec3::new(4.0, 2.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.0),
                        emmission: Vec3::new(20.0, 10.0, 10.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                SceneObjectDescriptor {
                    position: Vec3::new(-6.0, -2.0, 0.0),
                    type_: SceneObjectDescriptorType::Sphere { radius: 1.0 },
                    material: Material {
                        color: Vec3::new(0.0, 0.0, 0.0),
                        emmission: Vec3::new(5.0, 9.0, 20.0),
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ],
            camera: default_camera.clone(),
        },
        SceneDescriptor {
            id: "cornell".to_owned(),
            objects: vec![
                // Objects
                // mirroring
                SceneObjectDescriptor {
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.8 },
                    position: Vec3::new(-1.3, -BOX.y + 0.8, -1.3),
                    material: Material {
                        color: Vec3::splat(0.999),
                        emmission: Vec3::default(),
                        reflect_type: ReflectType::Specular,
                    },
                },
                // refracting
                SceneObjectDescriptor {
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.8 },
                    position: Vec3::new(1.3, -BOX.y + 0.8, -0.2),
                    material: Material {
                        color: Vec3::splat(0.999),
                        emmission: Vec3::default(),
                        reflect_type: ReflectType::Refract,
                    },
                },
                // emmission
                SceneObjectDescriptor {
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.5 },
                    position: Vec3::new(0.08, -BOX.y + 0.8, -0.8),
                    material: Material {
                        color: Vec3::splat(0.999),
                        emmission: Vec3::new(0.98, 1.0, 0.9) * 2.0,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
                // diffuse
                SceneObjectDescriptor {
                    type_: SceneObjectDescriptorType::Sphere { radius: 0.5 },
                    position: Vec3::new(-0.08, -BOX.y + 0.8, 0.7),
                    material: Material {
                        color: Vec3::new(0.4, 0.9, 0.49),
                        emmission: Vec3::ZERO,
                        reflect_type: ReflectType::Diffuse,
                    },
                },
            ]
            .into_iter()
            .chain(cornell_box.clone())
            .collect(),
            camera: default_camera.clone(),
        },
        SceneDescriptor {
            id: "mesh".to_owned(),
            objects: vec![SceneObjectDescriptor {
                position: Vec3::new(-0.8, -BOX.y + 0.5, 0.0),
                type_: SceneObjectDescriptorType::MeshFile(MeshFileDescriptor {
                    path: "meshes/mctri.off".to_owned(),
                    scale: 0.16,
                }),
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
                Vec3::new(0.9, -BOX.y + 1.8, BOX.z - 1.0),
                Vec3::new(-0.09, -0.06, -1.0),
            ),
        },
    ];
}

// Helper function to create a quad (rectangle) from two triangles
fn single_quad_mesh(size_x: f32, size_y: f32, axis: usize, flip: bool) -> Mesh {
    // Create a quad along the specified axis (0=X, 1=Y, 2=Z)
    // with the normal pointing in the positive direction
    // flip=true will make normal point in negative direction

    let mut vertices = Vec::with_capacity(4);

    for i in 0..2 {
        for j in 0..2 {
            let mut pos = [0.0, 0.0, 0.0];
            let idx1 = (axis + 1) % 3;
            let idx2 = (axis + 2) % 3;
            pos[idx1] = if i == 0 { -size_x } else { size_x };
            pos[idx2] = if j == 0 { -size_y } else { size_y };

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
