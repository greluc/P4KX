// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::num::NonZeroU64;
use std::sync::Arc;

use super::GpuReport;
use super::limits::check_buffer;
use super::pipelines::{COLOUR_FORMAT, DEPTH_FORMAT, layout};
use super::resources::{VERTEX_STRIDE, filled};
use crate::rigid::IDENTITY_MATRIX;
use crate::scene::{SKIN_STRIDE, Scene};
use crate::skeleton::Pose;

const LOOK_LEN: usize = 32;
pub(super) const LOOK_BYTES: u64 = 32;

const SKINNED_SHADER: &str = include_str!("skinned.wgsl");
const SKELETON_SHADER: &str = include_str!("skeleton.wgsl");
const MATRIX_BYTES: u64 = 64;
const BONE_BYTES: u64 = 16;
const SKIN_STRIDE_BYTES: u64 = 12;

/// The pipelines that skin the model and draw its skeleton, and the layout of the group both read.
#[derive(Debug)]
pub(super) struct SkeletonPipelines {
    pub(super) layout: wgpu::BindGroupLayout,
    pub(super) skinned: wgpu::RenderPipeline,
    pub(super) bones: wgpu::RenderPipeline,
}

impl SkeletonPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        (frame, material): (&wgpu::BindGroupLayout, &wgpu::BindGroupLayout),
        scene_module: &wgpu::ShaderModule,
        samples: u32,
    ) -> Self {
        let group = layout(
            device,
            "p4kx-viewer-skeleton",
            &[
                storage_entry(0, MATRIX_BYTES),
                storage_entry(1, BONE_BYTES),
                look_entry(2),
            ],
        );
        let skinned = skinned_pipeline(device, [frame, material, &group], scene_module, samples);
        let bones = bones_pipeline(device, [frame, &group], samples);
        Self {
            layout: group,
            skinned,
            bones,
        }
    }
}

const fn storage_entry(binding: u32, element: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(element),
        },
        count: None,
    }
}

const fn look_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(LOOK_BYTES),
        },
        count: None,
    }
}

fn module(device: &wgpu::Device, label: &str, source: &'static str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
    })
}

fn depth(compare: wgpu::CompareFunction, write: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

const fn target() -> [Option<wgpu::ColorTargetState>; 1] {
    [Some(wgpu::ColorTargetState {
        format: COLOUR_FORMAT,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })]
}

fn skinned_pipeline(
    device: &wgpu::Device,
    layouts: [&wgpu::BindGroupLayout; 3],
    scene_module: &wgpu::ShaderModule,
    samples: u32,
) -> wgpu::RenderPipeline {
    let vertex_module = module(device, "p4kx-viewer-skinned", SKINNED_SHADER);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("p4kx-viewer-skinned"),
        bind_group_layouts: &layouts.map(Some),
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("p4kx-viewer-skinned"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_module,
            entry_point: Some("vs_skinned"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                    ],
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: SKIN_STRIDE_BYTES,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![3 => Uint16x4, 4 => Unorm8x4],
                }),
            ],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(depth(wgpu::CompareFunction::Less, true)),
        multisample: multisample(samples),
        fragment: Some(wgpu::FragmentState {
            module: scene_module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &target(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn bones_pipeline(
    device: &wgpu::Device,
    layouts: [&wgpu::BindGroupLayout; 2],
    samples: u32,
) -> wgpu::RenderPipeline {
    let module = module(device, "p4kx-viewer-skeleton", SKELETON_SHADER);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("p4kx-viewer-skeleton"),
        bind_group_layouts: &layouts.map(Some),
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("p4kx-viewer-skeleton"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth(wgpu::CompareFunction::Always, false)),
        multisample: multisample(samples),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &target(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

const fn multisample(samples: u32) -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: samples,
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}

/// Returns whether a device with `limits` holds the matrices of `bones` bones and the weights of
/// `scene`, and reads two storage buffers in the vertex stage.
fn fits_device(limits: &wgpu::Limits, scene: &Scene, bones: usize) -> bool {
    let fits = |bytes: Option<u64>, limit: u64| {
        bytes.is_some_and(|bytes| check_buffer(bytes, limit).is_ok())
    };
    let matrix_bytes = u64::try_from(bones)
        .ok()
        .and_then(|bones| bones.checked_mul(MATRIX_BYTES));
    let skin_bytes = scene.skin().map(|skin| {
        skin.len()
            .checked_mul(SKIN_STRIDE)
            .and_then(|bytes| u64::try_from(bytes).ok())
    });
    fits(matrix_bytes, limits.max_storage_buffer_binding_size)
        && fits(matrix_bytes, limits.max_buffer_size)
        && skin_bytes.is_none_or(|bytes| fits(bytes, limits.max_buffer_size))
        && limits.max_storage_buffers_per_shader_stage >= 2
}

/// The buffers that pose and draw the skeleton of a scene, destroyed when dropped: the bone
/// matrices written every frame, the bind positions and parents, the look of the overlay and,
/// for a skinned scene, the per-vertex bone weights.
#[derive(Debug)]
pub(super) struct SkeletonBuffers {
    pub(super) skin: Option<wgpu::Buffer>,
    matrices: wgpu::Buffer,
    bones: wgpu::Buffer,
    pub(super) look: wgpu::Buffer,
    pub(super) group: wgpu::BindGroup,
    pub(super) bone_count: u32,
    report: Arc<GpuReport>,
}

impl SkeletonBuffers {
    /// Uploads the skeleton of `scene` at rest, or returns `None` when the scene has none or the
    /// device cannot hold it; the scene is then drawn without it.
    pub(super) fn upload(
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        layout: &wgpu::BindGroupLayout,
        scene: &Scene,
        report: &Arc<GpuReport>,
    ) -> Option<Self> {
        let skeleton = scene.skeleton()?;
        let count = skeleton.bone_count();
        if !fits_device(&device.limits(), scene, count) {
            return None;
        }
        let bone_count = u32::try_from(count).ok()?;
        let rest = vec![IDENTITY_MATRIX; count];
        let matrices = filled(
            (device, queue),
            ("p4kx-viewer-bone-matrices", wgpu::BufferUsages::STORAGE),
            bytemuck::cast_slice(&rest),
        );
        let bones = filled(
            (device, queue),
            ("p4kx-viewer-bones", wgpu::BufferUsages::STORAGE),
            bytemuck::cast_slice(&skeleton.static_bones()),
        );
        let look = filled(
            (device, queue),
            ("p4kx-viewer-bone-look", wgpu::BufferUsages::UNIFORM),
            &[0_u8; LOOK_LEN],
        );
        let skin = scene.skin().map(|skin| {
            filled(
                (device, queue),
                ("p4kx-viewer-skin", wgpu::BufferUsages::VERTEX),
                bytemuck::cast_slice(skin),
            )
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p4kx-viewer-skeleton"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bones.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: look.as_entire_binding(),
                },
            ],
        });
        report.buffer_created(3 + usize::from(skin.is_some()));
        Some(Self {
            skin,
            matrices,
            bones,
            look,
            group,
            bone_count,
            report: Arc::clone(report),
        })
    }

    /// Writes the matrices of `pose` when it holds one per bone.
    pub(super) fn pose(&self, queue: &wgpu::Queue, pose: &Pose) {
        if pose.matrices().len() == self.bone_count as usize {
            queue.write_buffer(&self.matrices, 0, bytemuck::cast_slice(pose.matrices()));
        }
    }
}

impl Drop for SkeletonBuffers {
    fn drop(&mut self) {
        self.matrices.destroy();
        self.bones.destroy();
        self.look.destroy();
        if let Some(skin) = &self.skin {
            skin.destroy();
        }
        self.report
            .buffer_destroyed(3 + usize::from(self.skin.is_some()));
    }
}
