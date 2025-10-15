struct Uniforms {
    // offset: vec2f,
	view_proj: mat4x4<f32>,

	// center: vec2f,
	// scale: f32,
	// max_iter: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
	@builtin(position) position: vec4f,
    @location(0) color : vec4<f32>,
}

struct Vertex {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
    @builtin(vertex_index) vertexIndex : u32,
};

@vertex
fn vertex_main(
    vert: Vertex,
) -> VertexOut {
    var position = vert.position;

    // Projection matrix
    position = uniforms.view_proj * position;

    var vsOutput: VertexOut;
    vsOutput.position = position;
    vsOutput.color = vert.color;
    return vsOutput;
}

@fragment
fn fragment_main(
    fragData: VertexOut,
) -> @location(0) vec4<f32>
{
    return fragData.color;
} 

