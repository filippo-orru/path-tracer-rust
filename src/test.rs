use super::*;

#[test]
fn test_vector_operations() {
    let v1 = Vector::from(1.0, 2.0, 3.0);
    let v2 = Vector::from(2.0, 3.0, 4.0);
    let v3 = Vector::from(3.0, 4.0, 5.0);

    assert_eq!(v1 + v2, Vector::from(3.0, 5.0, 7.0));
    assert_eq!(v3 - v2, Vector::from(1.0, 1.0, 1.0));
    assert_eq!(v1 * v2, Vector::from(2.0, 6.0, 12.0));
    assert_eq!(v1 * 2.0, Vector::from(2.0, 4.0, 6.0));
    assert_eq!(v2 / 2.0, Vector::from(1.0, 1.5, 2.0));

    assert_eq!(v1.dot(&v2), 20.0);
    assert_eq!(v1.cross(&v2), Vector::from(-1.0, 2.0, -1.0));
    assert_eq!(
        Vector::from(1.0, 0.0, 0.0).normalized(),
        Vector::from(1.0, 0.0, 0.0)
    );
    assert_eq!(
        Vector::from(1.0, 1.0, 0.0).normalized(),
        Vector::from(0.7071067811865475, 0.7071067811865475, 0.0)
    );
    assert_eq!(
        Vector::from(1.0, 1.0, 0.0).normalize(),
        Vector::from(0.7071067811865475, 0.7071067811865475, 0.0)
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

#[test]
fn test_intersect_scene() {
    let ray = Ray {
        direction: Vector::from(0.0, 0.0, -1.0),
        origin: Vector::from(0.0, 0.0, 0.0),
    };

    let scene = vec![SceneObject {
        type_: SceneObjectType::Sphere {
            position: Vector::from(0.0, 0.0, -3.0),
            radius: 1.0,
        },
        material: Material {
            color: Vector::from(1.0, 0.0, 0.0),
            emmission: Vector::from(0.0, 0.0, 0.0),
            reflect_type: ReflectType::Diffuse,
        },
    }];

    let intersection = intersect_scene(&ray, &scene);

    assert_eq!(
        intersection,
        SceneIntersectResult::Hit {
            object_id: 0,
            hit: Hit {
                distance: 2.0,
                xmin: Vector::from(0.0, 0.0, -2.0),
                nmin: Vector::from(0.0, 0.0, 1.0),
            }
        }
    );
}

#[test]
fn test_radiance() {
    let scene = vec![
        SceneObject {
            type_: SceneObjectType::Sphere {
                position: Vector::from(0.0, 0.0, -3.0),
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
                emmission: Vector::from(50.0, 50.0, 50.0),
                reflect_type: ReflectType::Diffuse,
            },
        },
    ];

    let ray = Ray {
        direction: Vector::from(0.0, 0.0, -1.0),
        origin: Vector::from(0.0, 0.0, 0.0),
    };

    let mut radiance_v = Vector::zero();
    let sample_count = 10_000;

    for _ in 0..sample_count {
        radiance_v = radiance_v + radiance(&ray, 0, &scene);
    }
    radiance_v = radiance_v / sample_count as f64;

    assert!(radiance_v.x > 0.3, "radiance_v.x = {}", radiance_v.x);
}
