// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use eframe::egui_wgpu::wgpu;

use super::pipelines::{COLOUR_FORMAT, DEPTH_FORMAT};

/// The offscreen picture the model is drawn into and the bind group the blit samples it with.
#[derive(Debug)]
pub(super) struct Target {
    pub(super) extent: [u32; 2],
    pub(super) multisampled: Option<wgpu::TextureView>,
    pub(super) resolved: wgpu::TextureView,
    pub(super) depth: wgpu::TextureView,
    pub(super) blit_group: wgpu::BindGroup,
    textures: Vec<wgpu::Texture>,
}

impl Target {
    pub(super) fn new(
        device: &wgpu::Device,
        extent: [u32; 2],
        samples: u32,
        (blit_layout, blit_sampler, blit_uniform): (
            &wgpu::BindGroupLayout,
            &wgpu::Sampler,
            &wgpu::Buffer,
        ),
    ) -> Self {
        let make = |label: &str, format, count, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attachment = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let resolved = make(
            "p4kx-viewer-picture",
            COLOUR_FORMAT,
            1,
            attachment | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let multisampled =
            (samples > 1).then(|| make("p4kx-viewer-samples", COLOUR_FORMAT, samples, attachment));
        let depth = make("p4kx-viewer-depth", DEPTH_FORMAT, samples, attachment);
        let view =
            |texture: &wgpu::Texture| texture.create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_view = view(&resolved);
        let blit_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p4kx-viewer-blit"),
            layout: blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&resolved_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(blit_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: blit_uniform.as_entire_binding(),
                },
            ],
        });
        Self {
            extent,
            multisampled: multisampled.as_ref().map(view),
            depth: view(&depth),
            resolved: resolved_view,
            blit_group,
            textures: [Some(resolved), multisampled, Some(depth)]
                .into_iter()
                .flatten()
                .collect(),
        }
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        for texture in &self.textures {
            texture.destroy();
        }
    }
}
