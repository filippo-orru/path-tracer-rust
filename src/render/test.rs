use super::*;

#[test]
fn test_vector_operations() {
    let v1 = Vec3::from(1.0, 2.0, 3.0);
    let v2 = Vec3::from(2.0, 3.0, 4.0);
    let v3 = Vec3::from(3.0, 4.0, 5.0);

    assert_eq!(v1 + v2, Vec3::from(3.0, 5.0, 7.0));
    assert_eq!(v3 - v2, Vec3::from(1.0, 1.0, 1.0));
    assert_eq!(v1 * v2, Vec3::from(2.0, 6.0, 12.0));
    assert_eq!(v1 * 2.0, Vec3::from(2.0, 4.0, 6.0));
    assert_eq!(v2 / 2.0, Vec3::from(1.0, 1.5, 2.0));

    assert_eq!(v1.dot(&v2), 20.0);
    assert_eq!(v1.cross(&v2), Vec3::from(-1.0, 2.0, -1.0));
    assert_eq!(
        Vec3::from(1.0, 0.0, 0.0).normalize(),
        Vec3::from(1.0, 0.0, 0.0)
    );
    assert_eq!(
        Vec3::from(1.0, 1.0, 0.0).normalize(),
        Vec3::from(0.7071067811865475, 0.7071067811865475, 0.0)
    );

    assert_eq!(v1.magnitude(), 3.7416573867739413);
}

#[test]
fn test_helpers() {
    assert_eq!(to_int_with_gamma_correction(0.0), 0);
    assert_eq!(to_int_with_gamma_correction(0.5), 186);
    assert_eq!(to_int_with_gamma_correction(0.75), 224);
    assert_eq!(to_int_with_gamma_correction(1.0), 255);
}

const TEST_MAT: Material = Material {
    color: Vec3::from(1.0, 0.0, 0.0),
    emmission: Vec3::from(0.0, 0.0, 0.0),
    reflect_type: ReflectType::Diffuse,
};

#[test]
fn test_intersect_scene() {
    let ray = Ray {
        direction: Vec3::from(0.0, 0.0, -1.0),
        origin: Vec3::from(0.0, 0.0, 0.0),
    };

    let scene = vec![SceneObjectData {
        position: Vec3::from(0.0, 0.0, -3.0),
        type_: SceneObject::Sphere { radius: 1.0 },
        material: TEST_MAT,
    }];

    let intersection = intersect_scene(&ray, &scene);

    assert_eq!(
        intersection,
        SceneIntersectResult::Hit {
            object_id: 0,
            hit: Hit {
                distance: 2.0,
                intersection: Vec3::from(0.0, 0.0, -2.0),
                normal: Vec3::from(0.0, 0.0, 1.0),
            }
        }
    );
}

// Test a ray that misses the sphere
#[test]
fn test_ray_misses_sphere() {
    let ray = Ray {
        direction: Vec3::from(1.0, 0.0, -1.0).normalize(),
        origin: Vec3::from(2.0, 0.0, 0.0),
    };

    let scene = vec![SceneObjectData {
        position: Vec3::from(0.0, 0.0, -3.0),
        type_: SceneObject::Sphere { radius: 1.0 },
        material: TEST_MAT,
    }];

    let intersection = intersect_scene(&ray, &scene);
    assert_eq!(intersection, SceneIntersectResult::NoHit);
}

// Test a ray originating inside the sphere
#[test]
fn test_ray_inside_sphere() {
    let ray = Ray {
        direction: Vec3::from(0.0, 0.0, -1.0),
        origin: Vec3::from(0.0, 0.0, 0.0),
    };

    let scene = vec![SceneObjectData {
        position: Vec3::from(0.0, 0.0, 0.0),
        type_: SceneObject::Sphere { radius: 1.0 },
        material: TEST_MAT,
    }];

    let intersection = intersect_scene(&ray, &scene);
    // Expected result should account for intersection from inside the sphere
    assert_eq!(
        intersection,
        SceneIntersectResult::Hit {
            object_id: 0,
            hit: Hit {
                distance: 1.0,
                intersection: Vec3::from(0.0, 0.0, -1.0),
                normal: Vec3::from(0.0, 0.0, -1.0),
            }
        }
    );
}

// Test a ray that grazes the sphere tangentially
#[test]
fn test_ray_tangent_to_sphere() {
    let ray = Ray {
        direction: Vec3::from(0.0, 0.0, -1.0),
        origin: Vec3::from(0.0, 1.0, 0.0),
    };

    let scene = vec![SceneObjectData {
        position: Vec3::from(0.0, 0.0, -3.0),
        type_: SceneObject::Sphere { radius: 1.0 },
        material: TEST_MAT,
    }];

    let intersection = intersect_scene(&ray, &scene);
    assert_eq!(
        intersection,
        SceneIntersectResult::Hit {
            object_id: 0,
            hit: Hit {
                distance: 3.0,
                intersection: Vec3::from(0.0, 1.0, -3.0),
                normal: Vec3::from(0.0, 1.0, 0.0),
            }
        }
    );
}

#[test]
fn test_radiance() {
    let scene = vec![
        SceneObjectData {
            position: Vec3::from(0.0, 0.0, -3.0),
            type_: SceneObject::Sphere { radius: 1.0 },
            material: Material {
                color: Vec3::from(1.0, 0.0, 0.0),
                emmission: Vec3::from(0.0, 0.0, 0.0),
                reflect_type: ReflectType::Diffuse,
            },
        },
        SceneObjectData {
            position: Vec3::from(0.0, 0.0, 10.0),
            type_: SceneObject::Sphere { radius: 1.0 },
            material: Material {
                color: Vec3::from(0.0, 0.0, 0.0),
                emmission: Vec3::from(50.0, 50.0, 50.0),
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let ray = Ray {
        direction: Vec3::from(0.0, 0.0, -1.0),
        origin: Vec3::from(0.0, 0.0, 0.0),
    };

    let mut radiance_v = Vec3::zero();
    let sample_count = 10_000;

    for _ in 0..sample_count {
        radiance_v = radiance_v + radiance(&ray, 0, &scene);
    }
    radiance_v = radiance_v / sample_count as f64;

    assert!(radiance_v.x > 0.3, "radiance_v.x = {}", radiance_v.x);
}
