struct Frame {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    key_light: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(2) @binding(0) var<storage, read> skin_matrices: array<mat4x4<f32>>;

struct SkinnedInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) bones: vec4<u32>,
    @location(4) weights: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_skinned(input: SkinnedInput) -> VertexOutput {
    var position = input.position;
    var normal = input.normal;
    let total = input.weights.x + input.weights.y + input.weights.z + input.weights.w;
    if total > 0.0 {
        let blended = (skin_matrices[input.bones.x] * input.weights.x
            + skin_matrices[input.bones.y] * input.weights.y
            + skin_matrices[input.bones.z] * input.weights.z
            + skin_matrices[input.bones.w] * input.weights.w) * (1.0 / total);
        position = (blended * vec4<f32>(input.position, 1.0)).xyz;
        normal = (blended * vec4<f32>(input.normal, 0.0)).xyz;
    }
    var output: VertexOutput;
    output.clip = frame.view_proj * vec4<f32>(position, 1.0);
    output.world = position;
    output.normal = normal;
    output.uv = input.uv;
    return output;
}
