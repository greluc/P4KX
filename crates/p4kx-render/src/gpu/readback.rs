// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the drawn picture back from the GPU.

use std::sync::mpsc;
use std::time::Duration;

use super::device::guarded;
use super::limits::check_buffer;
use super::straight::{straighten, under};
use super::teardown::Gpu;
use super::{OffscreenError, Picture};

/// How long a submission or a map may take before the driver counts as not answering.
pub(super) const DEVICE_WAIT: Duration = Duration::from_secs(60);

const ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
pub(super) const BYTES_PER_PIXEL: u32 = 4;

impl Gpu {
    /// Reads the picture drawn for `serial` back as straight-alpha RGBA.
    pub(super) fn read_back(&self, serial: u64) -> Result<Picture, OffscreenError> {
        let renderer = &self.renderer;
        let (Some(target), Some(drawn)) = (
            renderer.target.as_ref(),
            renderer.drawn.filter(|drawn| drawn.serial == serial),
        ) else {
            return Err(OffscreenError::Failed);
        };
        let width = drawn.size[0].min(target.extent[0]);
        let height = drawn.size[1].min(target.extent[1]);
        let row = width.saturating_mul(BYTES_PER_PIXEL);
        let padded = row.next_multiple_of(ALIGNMENT);
        let buffer = self.copy_picture(&target.picture, (width, height, padded))?;
        let mapped = self.map(&buffer, (padded as usize, row as usize));
        buffer.destroy();
        let mut rgba = mapped?;
        if drawn.shading.is_premultiplied() {
            under(&mut rgba, drawn.colours.background);
            straighten(&mut rgba);
        }
        Picture::from_rgba(width, height, rgba)
            .ok_or_else(|| OffscreenError::Readback("the rows are shorter than the picture".into()))
    }

    fn copy_picture(
        &self,
        picture: &wgpu::Texture,
        (width, height, padded): (u32, u32, u32),
    ) -> Result<wgpu::Buffer, OffscreenError> {
        let bytes = u64::from(padded) * u64::from(height);
        check_buffer(bytes, self.device.limits().max_buffer_size)
            .map_err(|_| OffscreenError::Rejected)?;
        let (buffer, failed) = guarded(&self.device, || {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("p4kx-offscreen-readback"),
                size: bytes,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("p4kx-offscreen-readback"),
                });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: picture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            self.queue.submit([encoder.finish()]);
            buffer
        });
        if failed {
            buffer.destroy();
            return Err(OffscreenError::Failed);
        }
        Ok(buffer)
    }

    fn map(
        &self,
        buffer: &wgpu::Buffer,
        (padded, row): (usize, usize),
    ) -> Result<Vec<u8>, OffscreenError> {
        let readback =
            |error: &dyn std::error::Error| OffscreenError::Readback(error.to_string().into());
        let (sender, receiver) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                drop(sender.send(result));
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(DEVICE_WAIT),
            })
            .map_err(|error| readback(&error))?;
        receiver
            .recv_timeout(DEVICE_WAIT)
            .map_err(|error| readback(&error))?
            .map_err(|error| readback(&error))?;
        let view = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|error| readback(&error))?;
        let mut rgba = Vec::with_capacity(view.len() / padded.max(1) * row);
        for padded_row in view.chunks_exact(padded.max(1)) {
            rgba.extend_from_slice(padded_row.get(..row).unwrap_or_default());
        }
        drop(view);
        buffer.unmap();
        Ok(rgba)
    }
}
