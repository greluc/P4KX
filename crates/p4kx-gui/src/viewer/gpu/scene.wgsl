struct Uniforms {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    key_light: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Uniforms;
@group(1) @binding(0) var base_texture: texture_2d<f32>;
@group(1) @binding(1) var base_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

const ALBEDO: vec3<f32> = vec3<f32>(0.62, 0.64, 0.66);

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip = frame.view_proj * vec4<f32>(input.position, 1.0);
    output.world = input.position;
    output.normal = input.normal;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(base_texture, base_sampler, input.uv);
    let slope = cross(dpdx(input.world), dpdy(input.world));
    let flat_normal = slope / max(length(slope), 1.0e-20);
    let stored = length(input.normal) > 0.5;
    var normal = select(flat_normal, normalize(input.normal), stored);
    let to_eye = normalize(frame.eye.xyz - input.world);
    if dot(normal, to_eye) < 0.0 {
        normal = -normal;
    }
    let head = max(dot(normal, to_eye), 0.0);
    let key = max(dot(normal, frame.key_light.xyz), 0.0);
    let sky = mix(0.18, 0.42, 0.5 + 0.5 * normal.z);
    let light = sky + 0.30 * head + 0.50 * key;
    let colour = ALBEDO * texel.rgb * light;
    return vec4<f32>(clamp(colour, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
