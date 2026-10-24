struct Frame {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    key_light: vec4<f32>,
}

struct Bone {
    position: vec3<f32>,
    parent: u32,
}

struct Look {
    colour: vec4<f32>,
    view: vec4<f32>,
}

const NO_PARENT: u32 = 0xffffffffu;

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<storage, read> skin_matrices: array<mat4x4<f32>>;
@group(1) @binding(1) var<storage, read> bones: array<Bone>;
@group(1) @binding(2) var<uniform> look: Look;

fn posed(index: u32) -> vec4<f32> {
    let at = skin_matrices[index] * vec4<f32>(bones[index].position, 1.0);
    return frame.view_proj * at;
}

@vertex
fn vs_main(
    @builtin(vertex_index) corner: u32,
    @builtin(instance_index) bone: u32,
) -> @builtin(position) vec4<f32> {
    let parent = bones[bone].parent;
    let hidden = vec4<f32>(0.0, 0.0, 2.0, 1.0);
    if parent == NO_PARENT {
        return hidden;
    }
    let tail = posed(parent);
    let head = posed(bone);
    if tail.w <= 1.0e-4 || head.w <= 1.0e-4 {
        return hidden;
    }
    let size = look.view.xy;
    let start = tail.xy / tail.w;
    let end = head.xy / head.w;
    let along = (end - start) * size * 0.5;
    let length_px = length(along);
    var direction = vec2<f32>(1.0, 0.0);
    if length_px > 1.0e-4 {
        direction = along / length_px;
    }
    let side = select(-1.0, 1.0, corner == 1u || corner == 4u || corner == 5u);
    let at_end = corner >= 2u && corner != 4u;
    let across = vec2<f32>(-direction.y, direction.x) * side * look.view.z / size;
    let base = select(start, end, at_end);
    let depth = select(tail.z / tail.w, head.z / head.w, at_end);
    return vec4<f32>(base + across, depth, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return look.colour;
}
