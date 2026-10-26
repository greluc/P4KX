// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::GpuReport;
use super::limits::{Rejected, check_buffer, check_texture};
use super::mips::{chain, level_count};
use super::skeleton::SkeletonBuffers;
use crate::scene::Scene;

pub(super) const VERTEX_STRIDE: u64 = 32;

/// The vertex and index buffer of a scene and the buffers of its skeleton, destroyed when
/// dropped.
#[derive(Debug)]
pub(super) struct MeshBuffers {
    pub(super) vertices: wgpu::Buffer,
    pub(super) indices: wgpu::Buffer,
    pub(super) index_count: u32,
    pub(super) skeleton: Option<SkeletonBuffers>,
    report: Arc<GpuReport>,
}

/// Creates a buffer of `contents.len()` bytes and writes `contents` through the queue, so a failure
/// is an error for the enclosing error scope and never a panic.
pub(super) fn filled(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    (label, usage): (&str, wgpu::BufferUsages),
    contents: &[u8],
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: u64::try_from(contents.len()).unwrap_or(u64::MAX),
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, contents);
    buffer
}

impl MeshBuffers {
    /// Uploads `scene` once, with its skeleton when the pipelines for one exist, in which case
    /// they take bind groups of `skeleton_layout`.
    pub(super) fn upload(
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        skeleton_layout: Option<&wgpu::BindGroupLayout>,
        scene: &Scene,
        report: &Arc<GpuReport>,
    ) -> Result<Self, Rejected> {
        let limit = device.limits().max_buffer_size;
        let vertex_bytes = u64::try_from(scene.vertices().len())
            .ok()
            .and_then(|count| count.checked_mul(VERTEX_STRIDE))
            .unwrap_or(u64::MAX);
        let index_bytes = u64::try_from(scene.indices().len())
            .ok()
            .and_then(|count| count.checked_mul(4))
            .unwrap_or(u64::MAX);
        check_buffer(vertex_bytes, limit)?;
        check_buffer(index_bytes, limit)?;
        let index_count = u32::try_from(scene.indices().len()).map_err(|_| Rejected::Buffer {
            bytes: index_bytes,
            limit,
        })?;
        let vertices = filled(
            (device, queue),
            ("p4kx-viewer-vertices", wgpu::BufferUsages::VERTEX),
            bytemuck::cast_slice(scene.vertices()),
        );
        let indices = filled(
            (device, queue),
            ("p4kx-viewer-indices", wgpu::BufferUsages::INDEX),
            bytemuck::cast_slice(scene.indices()),
        );
        report.buffer_created(2);
        let skeleton = skeleton_layout
            .and_then(|layout| SkeletonBuffers::upload((device, queue), layout, scene, report));
        Ok(Self {
            vertices,
            indices,
            index_count,
            skeleton,
            report: Arc::clone(report),
        })
    }
}

impl Drop for MeshBuffers {
    fn drop(&mut self) {
        self.vertices.destroy();
        self.indices.destroy();
        self.report.buffer_destroyed(2);
    }
}

fn write_level(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    (width, height, rgba): (u32, u32, &[u8]),
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            mip_level,
            ..texture.as_image_copy()
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width.saturating_mul(4)),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

/// A base-colour texture and the bind group that samples it, destroyed when dropped.
#[derive(Debug)]
pub(super) struct MaterialTexture {
    texture: wgpu::Texture,
    pub(super) group: wgpu::BindGroup,
}

impl MaterialTexture {
    /// Uploads `rgba`, four bytes per texel in sRGB, as a `width` × `height` texture with a full
    /// mip chain built on the CPU, so a minified texture is sampled from a level that fits.
    pub(super) fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        (width, height, rgba): (u32, u32, &[u8]),
    ) -> Result<Self, Rejected> {
        check_texture(
            width,
            height,
            rgba.len(),
            device.limits().max_texture_dimension_2d,
        )?;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("p4kx-viewer-base-colour"),
            size,
            mip_level_count: level_count(width, height),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        write_level(queue, &texture, 0, (width, height, rgba));
        for (at, level) in (1..).zip(chain(width, height, rgba)) {
            write_level(
                queue,
                &texture,
                at,
                (level.width, level.height, &level.rgba),
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p4kx-viewer-material"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Ok(Self { texture, group })
    }
}

impl Drop for MaterialTexture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
