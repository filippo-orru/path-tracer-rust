# Rust path tracer

To better understand the cgrpt path tracer, I ported it to Rust.

![cornell box](static/imgs/cornell-box.png)

![red sphere](static/imgs/red-sphere.png)

# Features

- Parallel path tracing using [rayon](https://crates.io/crates/rayon)
- Improved ergonomics, like estimated time to completion
- Idiomatic Rust constructs

# Performance

When testing render times with the default cornell box, the Rust version was around 10% faster than the C++ version. All while being *a lot* easier to read and memory safe :)