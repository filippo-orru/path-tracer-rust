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
