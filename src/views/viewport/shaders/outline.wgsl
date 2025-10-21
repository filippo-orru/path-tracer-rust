#import objects_types.wgsl as types;

@export struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_texture: texture_depth_2d;
@group(0) @binding(2) var depth_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4f,
}

@vertex
fn vertex_main(
    vert: types::Vertex,
) -> VertexOut {
    var vsOutput: VertexOut;
    vsOutput.position = uniforms.view_proj * vert.position;
    return vsOutput;
}

@fragment
fn fragment_main(
    fragData: VertexOut,
) -> @location(0) vec4<f32>
{
    // Render outline based on depth texture
    let uv = fragData.position.xy / vec2f(textureDimensions(depth_texture));
    let depth = textureSample(depth_texture, depth_sampler, uv);
    let currentDepth = fragData.position.z / fragData.pos   ition.w;

    // Apply a small threshold for better edge detection
    let depthDiff = abs(depth - currentDepth);
    
    if (depthDiff > 0.001 && depth < currentDepth) {
        // Pixel is at the edge of an object, render outline color
        return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red outline
    } else {
        // No outline, discard by setting alpha to 0
        discard;
    }
    
    // This line won't be reached due to discard, but needed for compilation
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

