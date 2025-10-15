use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use super::{Mesh, Triangle, Vec3};

pub(crate) fn load_off(path: &str, scale: f32) -> Result<Mesh, std::io::Error> {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    let mut get_line = || -> Result<String, std::io::Error> {
        let mut line = String::new();
        while line.len() <= 0 || line.starts_with("#") {
            line.clear();
            reader.read_line(&mut line)?;
            line = line.trim().to_owned();
        }
        Ok(line)
    };

    let bad_data =
        |reason: &str| Result::Err(std::io::Error::new(std::io::ErrorKind::InvalidData, reason));

    // Read header
    if get_line()? != "OFF" {
        return bad_data("Invalid header");
    }

    // Read triangles
    let counts = get_line()?
        .split_whitespace()
        .map(|s| s.parse::<usize>().ok())
        .collect::<Vec<_>>();
    if counts.len() != 3 {
        return bad_data("Invalid element counts");
    }
    let (vertex_count, face_count, _) =
        (counts[0].unwrap(), counts[1].unwrap(), counts[2].unwrap());

    let mut vertices = Vec::with_capacity(vertex_count);

    for _ in 0..vertex_count {
        let line = get_line()?;
        let coords = line
            .split_whitespace()
            .map(|s| s.parse::<f32>().ok())
            .collect::<Vec<_>>();
        if coords.len() != 3 {
            return bad_data("Invalid vertex coordinates");
        }
        let vert = Vec3::new(coords[0].unwrap(), coords[1].unwrap(), coords[2].unwrap()) * scale;
        vertices.push(vert);
    }

    let mut triangles: Vec<Triangle> = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        let line = get_line()?;
        let indices = line
            .split_whitespace()
            .map(|s| s.parse::<usize>().ok())
            .collect::<Vec<_>>();
        if indices.len() < 4 {
            return bad_data(format!("Invalid face: {}", line).as_str());
        }
        let (count, a, b, c) = (
            indices[0].unwrap(),
            indices[1].unwrap(),
            indices[2].unwrap(),
            indices[3].unwrap(),
        );
        // Optional: read color
        if count != 3 {
            // Only triangles are supported
            return bad_data(format!("Invalid face: {}", line).as_str());
        }
        triangles.push(Triangle {
            a: vertices[a],
            b: vertices[b],
            c: vertices[c],
        });
    }

    return Ok(Mesh::new(triangles));
}
