struct Blit {
    scale: vec2<f32>,
    linear_target: f32,
    unused: f32,
}

@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var picture_sampler: sampler;
@group(0) @binding(2) var<uniform> blit: Blit;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var output: VertexOutput;
    output.clip = vec4<f32>(corner * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
    output.uv = vec2<f32>(corner.x, 1.0 - corner.y) * blit.scale;
    return output;
}

fn encode(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, linear <= vec3<f32>(0.0031308));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let colour = textureSample(picture, picture_sampler, input.uv).rgb;
    return vec4<f32>(select(encode(colour), colour, blit.linear_target > 0.5), 1.0);
}
