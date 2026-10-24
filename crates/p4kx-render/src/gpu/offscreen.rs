// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc;

use thiserror::Error;

use super::device::{block_on, guarded};
use super::limits::check_buffer;
use super::straight::straighten;
use super::{Frame, GpuReport, Renderer, Unavailable};

const ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
const BYTES_PER_PIXEL: u32 = 4;

/// Which graphics adapter to draw on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Adapters {
    /// The most capable adapter, or the software one when there is no other.
    #[default]
    Best,
    /// Only the software adapter (WARP, lavapipe), which gives a picture that does not depend on the graphics card.
    Software,
}

/// Why a picture could not be drawn.
#[derive(Debug, Error)]
pub enum OffscreenError {
    /// No graphics adapter, not even a software one, is available.
    #[error("no graphics adapter is available")]
    NoAdapter(#[source] wgpu::RequestAdapterError),
    /// The adapter refused to create a device.
    #[error("the graphics device could not be created")]
    Device(#[source] wgpu::RequestDeviceError),
    /// The device is too limited for the renderer.
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
    /// The picture has no pixels.
    #[error("the picture must be at least one pixel wide and high")]
    Empty,
    /// The picture is larger than the device draws.
    #[error("the picture is {width} x {height} pixels, the device draws at most {limit} a side")]
    TooLarge {
        /// The wanted width.
        width: u32,
        /// The wanted height.
        height: u32,
        /// The longest side of a texture the device supports.
        limit: u32,
    },
    /// The device cannot draw the shading that was asked for.
    #[error("the graphics device cannot draw the hologram shading")]
    Shading,
    /// The device was lost while drawing.
    #[error("the graphics device was lost")]
    Lost,
    /// Drawing failed, for example for lack of memory.
    #[error("drawing failed, for example for lack of memory")]
    Failed,
    /// The scene is too large for the buffers of the device.
    #[error("the scene is too large for the buffers of the graphics device")]
    Rejected,
    /// The finished picture could not be read back.
    #[error("the picture could not be read back")]
    Readback(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A finished picture: straight-alpha RGBA, eight bits a channel, the top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Picture {
    /// Returns the picture of `rgba` pixels, or `None` when the bytes are not four per pixel.
    #[must_use]
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let wanted = u64::from(width) * u64::from(height) * u64::from(BYTES_PER_PIXEL);
        (u64::try_from(rgba.len()).ok() == Some(wanted)).then_some(Self {
            width,
            height,
            rgba,
        })
    }

    /// Returns the width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the pixels as four bytes each: red, green, blue, alpha.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Writes the picture as a PNG.
    ///
    /// # Errors
    ///
    /// Returns the encoder's error when `writer` fails.
    pub fn write_png(&self, writer: impl Write) -> Result<(), png::EncodingError> {
        let mut encoder = png::Encoder::new(writer, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&self.rgba)
    }
}

/// The renderer on a device made without a window: it draws a [`Frame`] and reads the picture
/// back.
#[derive(Debug)]
pub struct Offscreen {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    adapter: String,
}

impl Offscreen {
    /// Opens a graphics device on the chosen `adapters` and sets the renderer up on it.
    ///
    /// # Errors
    ///
    /// Returns [`OffscreenError`] when there is no adapter, the device cannot be created or it
    /// is too limited for the renderer.
    pub fn new(adapters: Adapters) -> Result<Self, OffscreenError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = request(&instance, adapters)?;
        let info = adapter.get_info();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("p4kx-offscreen"),
            required_limits: adapter.limits(),
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(OffscreenError::Device)?;
        let renderer = Renderer::new(
            &device,
            &queue,
            &adapter,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )?;
        Ok(Self {
            device,
            queue,
            renderer,
            adapter: format!("{} ({:?})", info.name, info.backend),
        })
    }

    /// Returns the name and the backend of the adapter the pictures are drawn on.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.adapter
    }

    /// Returns what the renderer learned about the device and the scenes.
    #[must_use]
    pub fn report(&self) -> Arc<GpuReport> {
        self.renderer.report()
    }

    /// Draws `frame` and returns the picture.
    ///
    /// # Errors
    ///
    /// Returns [`OffscreenError`] when the size is empty or beyond the device, the device is
    /// lost, the scene does not fit its buffers or the drawing or the read-back fails.
    pub fn draw(&mut self, frame: &Frame) -> Result<Picture, OffscreenError> {
        let [width, height] = frame.size;
        if width == 0 || height == 0 {
            return Err(OffscreenError::Empty);
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(OffscreenError::TooLarge {
                width,
                height,
                limit,
            });
        }
        if !self.renderer.supports(&frame.shading) {
            return Err(OffscreenError::Shading);
        }
        let serial = frame.subject.serial();
        let commands = self.renderer.prepare(&self.device, &self.queue, frame);
        self.queue.submit(commands);
        let report = self.renderer.report();
        if report.is_lost() {
            return Err(OffscreenError::Lost);
        }
        if report.is_rejected(serial) {
            return Err(OffscreenError::Rejected);
        }
        if report.has_failed(serial) {
            return Err(OffscreenError::Failed);
        }
        self.read_back(serial)
    }

    fn read_back(&self, serial: u64) -> Result<Picture, OffscreenError> {
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
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| readback(&error))?;
        receiver
            .recv()
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

fn request(instance: &wgpu::Instance, adapters: Adapters) -> Result<wgpu::Adapter, OffscreenError> {
    let ask = |fallback| {
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: fallback,
            compatible_surface: None,
            ..wgpu::RequestAdapterOptions::default()
        }))
    };
    match adapters {
        Adapters::Software => ask(true),
        Adapters::Best => ask(false).or_else(|_| ask(true)),
    }
    .map_err(OffscreenError::NoAdapter)
}
