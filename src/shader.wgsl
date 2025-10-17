struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

@export struct MyUniforms {
	view_proj: mat4x4<f32>,
    resolution: vec2f,
}
// light: Light,

@group(0) @binding(0) var<uniform> uniforms: MyUniforms;

@export struct Vertex {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
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

// Hard-coded values, should pass these as uniforms
const light_position = vec3<f32>(1.0, -5.0, 5.0);
const light_color = vec3<f32>(1.0, 1.0, 1.0);
const ambient_strength = 0.1;
const ambient = ambient_strength * light_color;
const specular_strength = 0.5;
const shininess = 32.0;

@fragment
fn fragment_main(
    fragData: VertexOut,
) -> @location(0) vec4<f32>
{
    let normal = fragData.normal;

    
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

