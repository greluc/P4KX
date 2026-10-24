struct Uniforms {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    key_light: vec4<f32>,
    tint: vec4<f32>,
    shading: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) row0: vec4<f32>,
    @location(4) row1: vec4<f32>,
    @location(5) row2: vec4<f32>,
    @location(6) colour: vec4<f32>,
}

struct VertexOutput {
    @invariant @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct FragmentInput {
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let local = vec4<f32>(input.position, 1.0);
    let world = vec3<f32>(dot(input.row0, local), dot(input.row1, local), dot(input.row2, local));
    let has_normal = dot(input.normal, input.normal) > 0.25;
    let cofactor_x = cross(input.row1.xyz, input.row2.xyz);
    let cofactor_y = cross(input.row2.xyz, input.row0.xyz);
    let cofactor_z = cross(input.row0.xyz, input.row1.xyz);
    let turned = vec3<f32>(
        dot(cofactor_x, input.normal),
        dot(cofactor_y, input.normal),
        dot(cofactor_z, input.normal),
    );
    let unit = turned / max(length(turned), 1.0e-20);
    output.clip = frame.view_proj * vec4<f32>(world, 1.0);
    output.world = world;
    output.normal = select(vec3<f32>(0.0), unit, has_normal);
    return output;
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let slope = cross(dpdx(input.world), dpdy(input.world));
    let flat_normal = slope / max(length(slope), 1.0e-20);
    let stored = length(input.normal) > 0.5;
    let normal = select(flat_normal, normalize(input.normal), stored);
    let to_eye = normalize(frame.eye.xyz - input.world);
    let facing = abs(dot(normal, to_eye));
    let rim = pow(1.0 - facing, 2.0);
    let lit = 0.5 + 0.5 * abs(dot(normal, frame.key_light.xyz));
    let glow = frame.tint.a;
    let body = frame.shading.x * (0.45 + 0.55 * lit);
    let edge = 1.6 * glow * rim;
    let alpha = clamp(body + 0.9 * edge, 0.0, 1.0);
    let colour = frame.tint.rgb * (body + edge) + vec3<f32>(0.3 * edge * edge);
    return vec4<f32>(clamp(colour, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}

@fragment
fn fs_depth() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}
