fn store_scenes_json() {
    let scenes = load_scenes();
    for scene in scenes {
        let json = serde_json::to_string_pretty(&scene).unwrap();
        let filename = format!("scenes/{}.json", scene.id);
        std::fs::write(filename, json).unwrap();
    }
}

fn sphere_bounding_box(radius: f32) -> (Vec3, Vec3) {
    let min = Vec3::new(-radius, -radius, -radius);
    let max = Vec3::new(radius, radius, radius);
    return (min, max);
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
