struct Uniforms {
    offset: vec2f,
	projection: mat4x4<f32>,

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
    var position = vert.position + vec4f(uniforms.offset, 0.0, 0.0);

    // Projection matrix
    position = uniforms.projection * position;

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

// @fragment
// fn fs_main(in: VertexOut) -> @location(0) vec4f {
// 	let p0 = uniforms.center + (in.position.xy - uniforms.resolution * .5) * uniforms.scale;
// 	var p = p0;
// 	var i: u32 = 0;
// 	for (; i < uniforms.max_iter; i = i + 1) {
// 		let d = p * p;
// 		if (d.x + d.y > 4.) {
// 			break;
// 		}

// 		p = vec2f(d.x - d.y + p0.x, 2. * p.x * p.y + p0.y);
// 	}

// 	if (i >= uniforms.max_iter) {
// 		return vec4f(vec3f(0.), 1.);
// 	} else {
// 		return vec4f(vec3f(f32(i) / f32(uniforms.max_iter)), 1.);
// 	}
// }
