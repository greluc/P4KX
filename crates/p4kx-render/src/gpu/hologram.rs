// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use super::assembly::buffer_layouts;
use super::pipelines::{COLOUR_FORMAT, DEPTH_FORMAT};

const SHADER: &str = include_str!("hologram.wgsl");

const CAPPED: wgpu::BlendComponent = wgpu::BlendComponent {
    src_factor: wgpu::BlendFactor::Constant,
    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
    operation: wgpu::BlendOperation::Add,
};

/// The pipeline of the hologram shading pass: every surface blended over the background without
/// a depth test, each layer weaker the deeper it lies. The blend constant is the cap that the
/// accumulated opacity approaches and never passes, so many layers cannot turn the shell solid.
#[derive(Debug)]
pub(super) struct HologramPipelines {
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
            colour: pipeline(device, &layout, &module, samples),
        }
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    samples: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("p4kx-viewer-hologram-shell"),
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
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
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
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOUR_FORMAT,
                blend: Some(wgpu::BlendState {
                    color: CAPPED,
                    alpha: CAPPED,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
