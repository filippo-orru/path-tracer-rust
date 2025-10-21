import super::objects_types::Vertex;

struct Uniforms {
    outline_color: vec3<f32>,
    // resolution: vec2f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_texture: texture_depth_2d;
// @group(0) @binding(2) var depth_sampler: sampler_comparison;
@group(0) @binding(2) var color_texture: texture_2d<f32>;
// @group(0) @binding(3) var color_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4f,
}

@vertex
fn vertex_main(
    vert: Vertex,
) -> VertexOut {
    var vsOutput: VertexOut;
    vsOutput.position = vert.position;
    return vsOutput;
}

@fragment
fn fragment_main(
    fragData: VertexOut,
) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(fragData.position.xy);
    let depth_raw = textureLoad(depth_texture, pixel, 0);
    let depth_visual = pow(depth_raw, 0.4);
    let depth_color = vec4<f32>(depth_visual, depth_visual, depth_visual, 1.0);

    let x_ = vec4<f32>(uniforms.outline_color, 1.0);
    let color = textureLoad(color_texture, pixel, 0);
    let height: f32 = f32(textureDimensions(color_texture, 0).y);

    var m = 0.0;
    if fragData.position.y / height > 0.5 {
        m = 1.0;
    }
    return mix(color, depth_color, m);
}

