// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use super::assembly::buffer_layouts;
use super::pipelines::{COLOUR_FORMAT, DEPTH_FORMAT};

const SHADER: &str = include_str!("hologram.wgsl");

/// The pipelines of the hologram shading pass: the depth of the nearest surface first, then the
/// tinted shell on exactly that surface, blended over the background with premultiplied alpha.
#[derive(Debug)]
pub(super) struct HologramPipelines {
    pub(super) depth: wgpu::RenderPipeline,
    pub(super) colour: wgpu::RenderPipeline,
}

impl HologramPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        (frame, material): (&wgpu::BindGroupLayout, &wgpu::BindGroupLayout),
        samples: u32,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("p4kx-viewer-hologram"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p4kx-viewer-hologram"),
            bind_group_layouts: &[Some(frame), Some(material)],
            immediate_size: 0,
        });
        Self {
            depth: pipeline(device, &layout, &module, samples, false),
            colour: pipeline(device, &layout, &module, samples, true),
        }
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    samples: u32,
    shaded: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if shaded {
            "p4kx-viewer-hologram-shell"
        } else {
            "p4kx-viewer-hologram-depth"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &buffer_layouts(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!shaded),
            depth_compare: Some(if shaded {
                wgpu::CompareFunction::LessEqual
            } else {
                wgpu::CompareFunction::Less
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(if shaded { "fs_main" } else { "fs_depth" }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOUR_FORMAT,
                blend: shaded.then_some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: if shaded {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::empty()
                },
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
