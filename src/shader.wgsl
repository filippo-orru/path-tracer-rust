struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

struct Uniforms {
    // offset: vec2f,
	view_proj: mat4x4<f32>,

	// center: vec2f,
	// scale: f32,
	// max_iter: u32,
    // light: Light,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

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
    vsOutput.world_position = vert.position;
    vsOutput.normal = normalize(vert.position.xyz); // Assuming the model is centered at the origin
    return vsOutput;
}

struct VertexOut {
	@builtin(position) position: vec4f,
    @location(0) color : vec4<f32>,
    @location(1) world_position : vec4<f32>,
    @location(2) normal : vec3<f32>,
}

@fragment
fn fragment_main(
    fragData: VertexOut,
) -> @location(0) vec4<f32>
{
    let normal = fragData.normal;
    
    // Hard-coded values (you should pass these as uniforms)
    let light_position = vec3<f32>(1.0, -5.0, 5.0);
    let light_color = vec3<f32>(1.0, 1.0, 1.0);
    let ambient_strength = 0.1;
    let specular_strength = 0.5;
    let shininess = 32.0;
    
    // Ambient component
    let ambient = ambient_strength * light_color;
    
    // Diffuse component
    let light_dir = normalize(light_position - fragData.world_position.xyz);
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = diff * light_color;
    
    // Specular component
    // Assuming view position is at 0,0,0 - adjust as needed
    let view_dir = normalize(-fragData.world_position.xyz);
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), shininess);
    let specular = specular_strength * spec * light_color;
    
    // Combine all components
    let result = (ambient + diffuse + specular) * fragData.color.rgb;
    
    return vec4<f32>(result, fragData.color.a);
}

