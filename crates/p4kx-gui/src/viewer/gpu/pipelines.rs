// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::num::NonZeroU64;

use eframe::egui_wgpu::wgpu;

use super::resources::VERTEX_STRIDE;

pub(super) const COLOUR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const FRAME_BYTES: u64 = 96;
pub(super) const BLIT_BYTES: u64 = 16;

const SCENE_SHADER: &str = include_str!("scene.wgsl");
const BLIT_SHADER: &str = include_str!("blit.wgsl");

/// The shaders, pipelines and layouts of the viewer.
#[derive(Debug)]
pub(super) struct Pipelines {
    pub(super) scene: wgpu::RenderPipeline,
    pub(super) blit: wgpu::RenderPipeline,
    pub(super) frame_layout: wgpu::BindGroupLayout,
    pub(super) material_layout: wgpu::BindGroupLayout,
    pub(super) blit_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        samples: u32,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let frame_layout = layout(
            device,
            "p4kx-viewer-frame",
            &[buffer_entry(
                0,
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                FRAME_BYTES,
            )],
        );
        let material_layout = layout(
            device,
            "p4kx-viewer-material",
            &[texture_entry(0), sampler_entry(1)],
        );
        let blit_layout = layout(
            device,
            "p4kx-viewer-blit",
            &[
                texture_entry(0),
                sampler_entry(1),
                buffer_entry(2, wgpu::ShaderStages::VERTEX_FRAGMENT, BLIT_BYTES),
            ],
        );
        let scene = scene_pipeline(device, [&frame_layout, &material_layout], samples);
        let blit = blit_pipeline(device, &blit_layout, target_format);
        Self {
            scene,
            blit,
            frame_layout,
            material_layout,
            blit_layout,
        }
    }
}

fn layout(
    device: &wgpu::Device,
    label: &str,
    entries: &[wgpu::BindGroupLayoutEntry],
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries,
    })
}

const fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    bytes: u64,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(bytes),
        },
        count: None,
    }
}

const fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

const fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn shader(device: &wgpu::Device, label: &str, source: &'static str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
    })
}

fn scene_pipeline(
    device: &wgpu::Device,
    layouts: [&wgpu::BindGroupLayout; 2],
    samples: u32,
) -> wgpu::RenderPipeline {
    let module = shader(device, "p4kx-viewer-scene", SCENE_SHADER);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("p4kx-viewer-scene"),
        bind_group_layouts: &layouts.map(Some),
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("p4kx-viewer-scene"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3,
                    1 => Float32x3,
                    2 => Float32x2,
                ],
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOUR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn blit_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = shader(device, "p4kx-viewer-blit", BLIT_SHADER);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("p4kx-viewer-blit"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("p4kx-viewer-blit"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
