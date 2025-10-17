@export struct Uniforms {
	top_color: vec3<f32>,
    bottom_color: vec3<f32>,
    resolution: vec2f,
    camera_direction: vec3f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@export struct Vertex {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
};

@vertex
fn vertex_main(
    vert: Vertex,
) -> VertexOut {
    var vsOutput: VertexOut;
    vsOutput.position = vert.position;
    return vsOutput;
}

struct VertexOut {
	@builtin(position) position: vec4f,
}

struct FragIn {
	@builtin(position) position: vec4f,
}

@fragment
fn fragment_main(
    fragData: FragIn,
) -> @location(0) vec4<f32>
{
    // Calculate sky color based on uniforms.camera_direction and fragment position
    let uv = fragData.position.xy / uniforms.resolution;
    let normalized_y = uv.y;
    
    // Create gradient between top and bottom color
    // Lower values of y are higher in the sky (top of screen)
    var color = mix(uniforms.top_color, uniforms.bottom_color, normalized_y);
    
    // Optional: Add subtle variation based on camera direction
    // This makes the sky change slightly as camera moves
    let camera_factor = dot(normalize(uniforms.camera_direction), vec3<f32>(0.0, 1.0, 0.0)) * 0.2;
    color = mix(color, color * (1.0 + camera_factor), 0.5);
    
    return vec4<f32>(color, 1.0);   
}

