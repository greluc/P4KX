use std::sync::Arc;

use eframe::egui_wgpu::wgpu::{self, util::DeviceExt};

use super::GpuReport;
use super::limits::{Rejected, check_buffer, check_texture};
use crate::viewer::scene::Scene;

pub(super) const VERTEX_STRIDE: u64 = 32;

/// The vertex and index buffer of a scene, destroyed when dropped.
#[derive(Debug)]
pub(super) struct MeshBuffers {
    pub(super) vertices: wgpu::Buffer,
    pub(super) indices: wgpu::Buffer,
    pub(super) index_count: u32,
    report: Arc<GpuReport>,
}

impl MeshBuffers {
    /// Uploads `scene` once.
    pub(super) fn upload(
        device: &wgpu::Device,
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
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("p4kx-viewer-vertices"),
            contents: bytemuck::cast_slice(scene.vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("p4kx-viewer-indices"),
            contents: bytemuck::cast_slice(scene.indices()),
            usage: wgpu::BufferUsages::INDEX,
        });
        report.buffer_created(2);
        Ok(Self {
            vertices,
            indices,
            index_count,
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

/// A base-colour texture and the bind group that samples it, destroyed when dropped.
#[derive(Debug)]
pub(super) struct MaterialTexture {
    texture: wgpu::Texture,
    pub(super) group: wgpu::BindGroup,
}

impl MaterialTexture {
    /// Uploads `rgba`, four bytes per texel in sRGB, as a `width` × `height` texture.
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
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width.saturating_mul(4)),
                rows_per_image: Some(height),
            },
            size,
        );
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
