@export struct Uniforms {
	top_color: vec3<f32>,
    bottom_color: vec3<f32>,
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

@fragment
fn fragment_main(
    fragData: VertexOut,
) -> @location(0) vec4<f32>
{
    let t = fragData.position.y * 0.5 + 0.5;
    let color = mix(uniforms.bottom_color, uniforms.top_color, t);
    return vec4<f32>(color, 1.0);   
}

